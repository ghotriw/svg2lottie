use std::io::Write;
use std::sync::{Arc, OnceLock};

use crate::error::SvgToLottieError;
use crate::gradient::{
    create_linear_gradient_fill, create_linear_gradient_stroke, create_radial_gradient_fill,
    create_radial_gradient_stroke,
};
use crate::options::SvgToLottieOptions;

static FONT_DB: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();

fn get_font_database() -> Arc<usvg::fontdb::Database> {
    FONT_DB
        .get_or_init(|| {
            let mut db = usvg::fontdb::Database::new();
            db.load_system_fonts();
            Arc::new(db)
        })
        .clone()
}

#[derive(Debug, Clone)]
struct LottieBezierPath {
    c: bool,
    v: Vec<[f32; 2]>,
    i: Vec<[f32; 2]>,
    o: Vec<[f32; 2]>,
}

fn collect_paths_recursive(group: &usvg::Group, out: &mut Vec<usvg::Path>) {
    for child in group.children() {
        match child {
            usvg::Node::Group(g) => {
                collect_paths_recursive(g, out);
            }
            usvg::Node::Path(p) if p.is_visible() => {
                out.push(*p.clone());
            }
            _ => {}
        }
    }
}

fn flush_subpath(
    subpaths: &mut Vec<LottieBezierPath>,
    vertices: &mut Vec<[f32; 2]>,
    in_tangents: &mut Vec<[f32; 2]>,
    out_tangents: &mut Vec<[f32; 2]>,
    closed: &mut bool,
    is_filled: bool,
) {
    if vertices.len() >= 2 {
        let is_closed = *closed || (is_filled && vertices.len() >= 3);
        let v_len = vertices.len();
        if is_closed && v_len >= 3 {
            let dx = vertices[v_len - 1][0] - vertices[0][0];
            let dy = vertices[v_len - 1][1] - vertices[0][1];
            if dx * dx + dy * dy < 0.0001 {
                in_tangents[0] = in_tangents[v_len - 1];
                vertices.pop();
                in_tangents.pop();
                out_tangents.pop();
            }
        }
        subpaths.push(LottieBezierPath {
            c: is_closed,
            v: std::mem::take(vertices),
            i: std::mem::take(in_tangents),
            o: std::mem::take(out_tangents),
        });
    } else {
        vertices.clear();
        in_tangents.clear();
        out_tangents.clear();
    }
    *closed = false;
}

pub fn convert_svg_to_value(
    svg_data: &str,
    options: &SvgToLottieOptions,
) -> Result<serde_json::Value, SvgToLottieError> {
    let fontdb = get_font_database();
    let opt = usvg::Options {
        fontdb,
        ..Default::default()
    };

    let tree = usvg::Tree::from_str(svg_data, &opt)
        .map_err(|e| SvgToLottieError::SvgParseError(e.to_string()))?;

    let src_w = tree.size().width();
    let src_h = tree.size().height();
    if src_w <= 0.0 || src_h <= 0.0 {
        return Err(SvgToLottieError::InvalidDimensions {
            width: src_w,
            height: src_h,
        });
    }

    let mut paths = Vec::new();
    collect_paths_recursive(tree.root(), &mut paths);

    if paths.is_empty() {
        return Err(SvgToLottieError::NoDrawableShapes);
    }

    let canvas_w = options.width.unwrap_or(src_w.round() as u32);
    let canvas_h = options.height.unwrap_or(src_h.round() as u32);

    let (scale, offset_x, offset_y) = {
        let pad = options.padding as f32;
        let target_w = (canvas_w as f32 - pad * 2.0).max(1.0);
        let target_h = (canvas_h as f32 - pad * 2.0).max(1.0);
        let s = (target_w / src_w).min(target_h / src_h);
        let fitted_w = src_w * s;
        let fitted_h = src_h * s;
        let ox = (canvas_w as f32 - fitted_w) / 2.0;
        let oy = (canvas_h as f32 - fitted_h) / 2.0;
        (s, ox, oy)
    };

    let view_transform = usvg::Transform::from_row(scale, 0.0, 0.0, scale, offset_x, offset_y);

    let mut layers: Vec<serde_json::Value> = Vec::new();
    let mut layer_idx = 1;

    for p in &paths {
        let combined = view_transform.pre_concat(p.abs_transform());
        let transformed_path = match p.data().clone().transform(combined) {
            Some(tp) => tp,
            None => continue,
        };

        let is_filled = p.fill().is_some();
        let mut subpaths: Vec<LottieBezierPath> = Vec::new();
        let mut vertices: Vec<[f32; 2]> = Vec::new();
        let mut in_tangents: Vec<[f32; 2]> = Vec::new();
        let mut out_tangents: Vec<[f32; 2]> = Vec::new();
        let mut closed = false;
        let mut cur_x = 0.0f32;
        let mut cur_y = 0.0f32;

        for seg in transformed_path.segments() {
            match seg {
                usvg::tiny_skia_path::PathSegment::MoveTo(pt) => {
                    flush_subpath(
                        &mut subpaths,
                        &mut vertices,
                        &mut in_tangents,
                        &mut out_tangents,
                        &mut closed,
                        is_filled,
                    );
                    cur_x = pt.x;
                    cur_y = pt.y;
                    vertices.push([cur_x, cur_y]);
                    in_tangents.push([0.0, 0.0]);
                    out_tangents.push([0.0, 0.0]);
                }
                usvg::tiny_skia_path::PathSegment::LineTo(pt) => {
                    cur_x = pt.x;
                    cur_y = pt.y;
                    vertices.push([cur_x, cur_y]);
                    in_tangents.push([0.0, 0.0]);
                    out_tangents.push([0.0, 0.0]);
                }
                usvg::tiny_skia_path::PathSegment::QuadTo(p1, p2) => {
                    let last_idx = vertices.len().saturating_sub(1);
                    if !out_tangents.is_empty() {
                        out_tangents[last_idx] = [
                            (2.0 / 3.0) * (p1.x - cur_x),
                            (2.0 / 3.0) * (p1.y - cur_y),
                        ];
                    }
                    let in_tan = [
                        (2.0 / 3.0) * (p1.x - p2.x),
                        (2.0 / 3.0) * (p1.y - p2.y),
                    ];
                    cur_x = p2.x;
                    cur_y = p2.y;
                    vertices.push([cur_x, cur_y]);
                    in_tangents.push(in_tan);
                    out_tangents.push([0.0, 0.0]);
                }
                usvg::tiny_skia_path::PathSegment::CubicTo(p1, p2, p3) => {
                    let last_idx = vertices.len().saturating_sub(1);
                    if !out_tangents.is_empty() {
                        out_tangents[last_idx] = [p1.x - cur_x, p1.y - cur_y];
                    }
                    let in_tan = [p2.x - p3.x, p2.y - p3.y];
                    cur_x = p3.x;
                    cur_y = p3.y;
                    vertices.push([cur_x, cur_y]);
                    in_tangents.push(in_tan);
                    out_tangents.push([0.0, 0.0]);
                }
                usvg::tiny_skia_path::PathSegment::Close => {
                    closed = true;
                }
            }
        }

        flush_subpath(
            &mut subpaths,
            &mut vertices,
            &mut in_tangents,
            &mut out_tangents,
            &mut closed,
            is_filled,
        );

        if subpaths.is_empty() {
            continue;
        }

        let fill_shape = if let Some(fill) = p.fill() {
            let fill_rule = match fill.rule() {
                usvg::FillRule::NonZero => 1,
                usvg::FillRule::EvenOdd => 2,
            };
            let opacity = fill.opacity().get() * 100.0;

            match fill.paint() {
                usvg::Paint::Color(c) => Some(serde_json::json!({
                    "ty": "fl",
                    "nm": "Fill",
                    "c": { "a": 0, "k": [c.red as f32 / 255.0, c.green as f32 / 255.0, c.blue as f32 / 255.0, 1.0] },
                    "o": { "a": 0, "k": opacity },
                    "r": fill_rule
                })),
                usvg::Paint::LinearGradient(lg) => {
                    Some(create_linear_gradient_fill(lg, combined, fill_rule, opacity))
                }
                usvg::Paint::RadialGradient(rg) => {
                    Some(create_radial_gradient_fill(rg, combined, fill_rule, opacity))
                }
                usvg::Paint::Pattern(_) => None,
            }
        } else {
            None
        };

        let stroke_shape = if let Some(stroke) = p.stroke() {
            let opacity = stroke.opacity().get() * 100.0;
            let sx = (combined.sx * combined.sx + combined.ky * combined.ky).sqrt();
            let sy = (combined.kx * combined.kx + combined.sy * combined.sy).sqrt();
            let effective_scale = (sx + sy) / 2.0;
            let stroke_width = (stroke.width().get() * effective_scale).max(0.5);
            let line_cap = match stroke.linecap() {
                usvg::LineCap::Butt => 1,
                usvg::LineCap::Round => 2,
                usvg::LineCap::Square => 3,
            };
            let line_join = match stroke.linejoin() {
                usvg::LineJoin::Miter | usvg::LineJoin::MiterClip => 1,
                usvg::LineJoin::Round => 2,
                usvg::LineJoin::Bevel => 3,
            };
            let miter_limit = stroke.miterlimit().get();

            match stroke.paint() {
                usvg::Paint::Color(c) => Some(serde_json::json!({
                    "ty": "st",
                    "nm": "Stroke",
                    "c": { "a": 0, "k": [c.red as f32 / 255.0, c.green as f32 / 255.0, c.blue as f32 / 255.0, 1.0] },
                    "o": { "a": 0, "k": opacity },
                    "w": { "a": 0, "k": stroke_width },
                    "lc": line_cap,
                    "lj": line_join,
                    "ml": miter_limit
                })),
                usvg::Paint::LinearGradient(lg) => Some(create_linear_gradient_stroke(
                    lg,
                    combined,
                    stroke_width,
                    line_cap,
                    line_join,
                    miter_limit,
                    opacity,
                )),
                usvg::Paint::RadialGradient(rg) => Some(create_radial_gradient_stroke(
                    rg,
                    combined,
                    stroke_width,
                    line_cap,
                    line_join,
                    miter_limit,
                    opacity,
                )),
                usvg::Paint::Pattern(_) => None,
            }
        } else {
            None
        };

        if fill_shape.is_none() && stroke_shape.is_none() {
            continue;
        }

        let mut shapes: Vec<serde_json::Value> = Vec::new();
        for (i, subpath) in subpaths.into_iter().enumerate() {
            shapes.push(serde_json::json!({
                "ty": "sh",
                "nm": format!("Path {}", i + 1),
                "ks": {
                    "a": 0,
                    "k": {
                        "c": subpath.c,
                        "v": subpath.v,
                        "i": subpath.i,
                        "o": subpath.o
                    }
                }
            }));
        }

        if let Some(fl) = fill_shape {
            shapes.push(fl);
        }
        if let Some(st) = stroke_shape {
            shapes.push(st);
        }

        let layer = serde_json::json!({
            "ddd": 0,
            "ind": layer_idx,
            "ty": 4,
            "nm": format!("Shape Layer {}", layer_idx),
            "sr": 1,
            "ks": {
                "o": { "a": 0, "k": 100 },
                "r": { "a": 0, "k": 0 },
                "p": { "a": 0, "k": [0, 0, 0] },
                "a": { "a": 0, "k": [0, 0, 0] },
                "s": { "a": 0, "k": [100, 100, 100] }
            },
            "ao": 0,
            "shapes": shapes,
            "ip": 0,
            "op": options.duration_frames,
            "st": 0,
            "bm": 0
        });

        layers.push(layer);
        layer_idx += 1;
    }

    if layers.is_empty() {
        return Err(SvgToLottieError::NoDrawableShapes);
    }

    layers.reverse();
    for (i, layer) in layers.iter_mut().enumerate() {
        if let Some(obj) = layer.as_object_mut() {
            obj.insert("ind".to_string(), serde_json::json!(i + 1));
        }
    }

    let mut root_map = serde_json::Map::new();

    if options.tgs_compatible {
        root_map.insert("tgs".to_string(), serde_json::json!(1));
    }
    root_map.insert("v".to_string(), serde_json::json!("5.5.2"));
    root_map.insert("fr".to_string(), serde_json::json!(options.fps));
    root_map.insert("ip".to_string(), serde_json::json!(0));
    root_map.insert("op".to_string(), serde_json::json!(options.duration_frames));
    root_map.insert("w".to_string(), serde_json::json!(canvas_w));
    root_map.insert("h".to_string(), serde_json::json!(canvas_h));
    root_map.insert(
        "nm".to_string(),
        serde_json::json!(options.name.as_deref().unwrap_or("Vector Animation")),
    );
    root_map.insert("ddd".to_string(), serde_json::json!(0));
    root_map.insert("assets".to_string(), serde_json::json!([]));
    root_map.insert("layers".to_string(), serde_json::json!(layers));
    root_map.insert("markers".to_string(), serde_json::json!([]));

    Ok(serde_json::Value::Object(root_map))
}

pub fn convert_svg_to_json(
    svg_data: &str,
    options: &SvgToLottieOptions,
) -> Result<String, SvgToLottieError> {
    let value = convert_svg_to_value(svg_data, options)?;
    serde_json::to_string(&value).map_err(|e| SvgToLottieError::SerializationError(e.to_string()))
}

pub fn convert_svg_to_tgs(
    svg_data: &str,
    options: &SvgToLottieOptions,
) -> Result<Vec<u8>, SvgToLottieError> {
    let value = convert_svg_to_value(svg_data, options)?;
    let json_bytes = serde_json::to_vec(&value)
        .map_err(|e| SvgToLottieError::SerializationError(e.to_string()))?;

    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    encoder
        .write_all(&json_bytes)
        .map_err(|e| SvgToLottieError::CompressionError(e.to_string()))?;
    encoder
        .finish()
        .map_err(|e| SvgToLottieError::CompressionError(e.to_string()))
}
