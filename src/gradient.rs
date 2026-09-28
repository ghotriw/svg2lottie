use usvg::Transform;

/// Encodes linear or radial gradient color/alpha stops into standard Lottie array format:
/// First `p * 4` entries: `[offset, r, g, b, ...]`, followed by optional opacity stops: `[offset, a, ...]`.
pub fn build_lottie_gradient_stops(stops: &[usvg::Stop]) -> (usize, Vec<f32>) {
    let p = stops.len();
    let mut color_data = Vec::with_capacity(p * 4);
    let mut alpha_data = Vec::with_capacity(p * 2);
    let mut has_partial_alpha = false;

    for stop in stops {
        let offset = stop.offset().get();
        let color = stop.color();
        let alpha = stop.opacity().get();

        color_data.push(offset);
        color_data.push(color.red as f32 / 255.0);
        color_data.push(color.green as f32 / 255.0);
        color_data.push(color.blue as f32 / 255.0);

        if (alpha - 1.0).abs() > 0.001 {
            has_partial_alpha = true;
        }
        alpha_data.push(offset);
        alpha_data.push(alpha);
    }

    if has_partial_alpha {
        color_data.extend(alpha_data);
    }

    (p, color_data)
}

pub fn create_linear_gradient_fill(
    lg: &usvg::LinearGradient,
    transform: Transform,
    fill_rule: u8,
    opacity: f32,
) -> serde_json::Value {
    let combined_tf = transform.pre_concat(lg.transform());

    let mut start_pt = usvg::tiny_skia_path::Point::from_xy(lg.x1(), lg.y1());
    let mut end_pt = usvg::tiny_skia_path::Point::from_xy(lg.x2(), lg.y2());

    combined_tf.map_point(&mut start_pt);
    combined_tf.map_point(&mut end_pt);

    let (points_count, gradient_k) = build_lottie_gradient_stops(lg.stops());

    serde_json::json!({
        "ty": "gf",
        "nm": "Linear Gradient Fill",
        "t": 1,
        "s": { "a": 0, "k": [start_pt.x, start_pt.y] },
        "e": { "a": 0, "k": [end_pt.x, end_pt.y] },
        "o": { "a": 0, "k": opacity },
        "r": fill_rule,
        "g": {
            "p": points_count,
            "k": { "a": 0, "k": gradient_k }
        }
    })
}

pub fn create_radial_gradient_fill(
    rg: &usvg::RadialGradient,
    transform: Transform,
    fill_rule: u8,
    opacity: f32,
) -> serde_json::Value {
    let combined_tf = transform.pre_concat(rg.transform());

    let mut center = usvg::tiny_skia_path::Point::from_xy(rg.cx(), rg.cy());
    let r = rg.r().get();
    let mut edge = usvg::tiny_skia_path::Point::from_xy(rg.cx() + r, rg.cy());

    combined_tf.map_point(&mut center);
    combined_tf.map_point(&mut edge);

    let (points_count, gradient_k) = build_lottie_gradient_stops(rg.stops());

    serde_json::json!({
        "ty": "gf",
        "nm": "Radial Gradient Fill",
        "t": 2,
        "s": { "a": 0, "k": [center.x, center.y] },
        "e": { "a": 0, "k": [edge.x, edge.y] },
        "o": { "a": 0, "k": opacity },
        "r": fill_rule,
        "g": {
            "p": points_count,
            "k": { "a": 0, "k": gradient_k }
        }
    })
}

#[allow(clippy::too_many_arguments)]
pub fn create_linear_gradient_stroke(
    lg: &usvg::LinearGradient,
    transform: Transform,
    stroke_width: f32,
    line_cap: u8,
    line_join: u8,
    miter_limit: f32,
    opacity: f32,
    dashes: Option<serde_json::Value>,
) -> serde_json::Value {
    let combined_tf = transform.pre_concat(lg.transform());

    let mut start_pt = usvg::tiny_skia_path::Point::from_xy(lg.x1(), lg.y1());
    let mut end_pt = usvg::tiny_skia_path::Point::from_xy(lg.x2(), lg.y2());

    combined_tf.map_point(&mut start_pt);
    combined_tf.map_point(&mut end_pt);

    let (points_count, gradient_k) = build_lottie_gradient_stops(lg.stops());

    let mut stroke_obj = serde_json::json!({
        "ty": "gs",
        "nm": "Linear Gradient Stroke",
        "t": 1,
        "s": { "a": 0, "k": [start_pt.x, start_pt.y] },
        "e": { "a": 0, "k": [end_pt.x, end_pt.y] },
        "o": { "a": 0, "k": opacity },
        "w": { "a": 0, "k": stroke_width },
        "lc": line_cap,
        "lj": line_join,
        "ml": miter_limit,
        "g": {
            "p": points_count,
            "k": { "a": 0, "k": gradient_k }
        }
    });

    if let (Some(d), Some(map)) = (dashes, stroke_obj.as_object_mut()) {
        map.insert("d".to_string(), d);
    }

    stroke_obj
}

#[allow(clippy::too_many_arguments)]
pub fn create_radial_gradient_stroke(
    rg: &usvg::RadialGradient,
    transform: Transform,
    stroke_width: f32,
    line_cap: u8,
    line_join: u8,
    miter_limit: f32,
    opacity: f32,
    dashes: Option<serde_json::Value>,
) -> serde_json::Value {
    let combined_tf = transform.pre_concat(rg.transform());

    let mut center = usvg::tiny_skia_path::Point::from_xy(rg.cx(), rg.cy());
    let r = rg.r().get();
    let mut edge = usvg::tiny_skia_path::Point::from_xy(rg.cx() + r, rg.cy());

    combined_tf.map_point(&mut center);
    combined_tf.map_point(&mut edge);

    let (points_count, gradient_k) = build_lottie_gradient_stops(rg.stops());

    let mut stroke_obj = serde_json::json!({
        "ty": "gs",
        "nm": "Radial Gradient Stroke",
        "t": 2,
        "s": { "a": 0, "k": [center.x, center.y] },
        "e": { "a": 0, "k": [edge.x, edge.y] },
        "o": { "a": 0, "k": opacity },
        "w": { "a": 0, "k": stroke_width },
        "lc": line_cap,
        "lj": line_join,
        "ml": miter_limit,
        "g": {
            "p": points_count,
            "k": { "a": 0, "k": gradient_k }
        }
    });

    if let (Some(d), Some(map)) = (dashes, stroke_obj.as_object_mut()) {
        map.insert("d".to_string(), d);
    }

    stroke_obj
}
