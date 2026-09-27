pub mod converter;
pub mod error;
pub mod gradient;
pub mod options;

pub use converter::{convert_svg_to_json, convert_svg_to_tgs, convert_svg_to_value};
pub use error::SvgToLottieError;
pub use options::SvgToLottieOptions;

/// Converts an SVG string into a Lottie JSON string with default options.
pub fn svg_to_lottie_json(svg_data: &str) -> Result<String, SvgToLottieError> {
    convert_svg_to_json(svg_data, &SvgToLottieOptions::default())
}

/// Converts an SVG string into a Telegram-compatible TGS (gzipped Lottie) with default options.
pub fn svg_to_tgs(svg_data: &str) -> Result<Vec<u8>, SvgToLottieError> {
    convert_svg_to_tgs(svg_data, &SvgToLottieOptions::telegram_sticker())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_shapes_conversion() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <rect x="10" y="10" width="80" height="80" fill="#ffcc00" stroke="#000000" stroke-width="2"/>
            <circle cx="50" cy="50" r="30" fill="#ff0000"/>
        </svg>"##;

        let options = SvgToLottieOptions::telegram_sticker();
        let value = convert_svg_to_value(svg, &options).expect("Conversion should succeed");

        assert_eq!(value["tgs"], 1);
        assert_eq!(value["w"], 512);
        assert_eq!(value["h"], 512);
        assert_eq!(value["layers"].as_array().unwrap().len(), 2);

        // Verify "ty" is serialized first for rlottie streaming parser compatibility
        let json_str = serde_json::to_string(&value).unwrap();
        assert!(json_str.contains(r#"{"ty":"sh""#));
        assert!(json_str.contains(r#"{"ty":"fl""#));
    }

    #[test]
    fn test_gradient_conversion() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200">
            <defs>
                <linearGradient id="lg1" x1="0" y1="0" x2="1" y2="1">
                    <stop offset="0%" stop-color="#ff0000" stop-opacity="1"/>
                    <stop offset="100%" stop-color="#0000ff" stop-opacity="0.5"/>
                </linearGradient>
                <radialGradient id="rg1" cx="50%" cy="50%" r="50%">
                    <stop offset="0%" stop-color="#00ff00"/>
                    <stop offset="100%" stop-color="#ffff00"/>
                </radialGradient>
            </defs>
            <rect x="10" y="10" width="80" height="80" fill="url(#lg1)"/>
            <circle cx="150" cy="150" r="40" fill="url(#rg1)"/>
        </svg>"##;

        let options = SvgToLottieOptions::default();
        let value = convert_svg_to_value(svg, &options).expect("Conversion should succeed");

        let json_str = serde_json::to_string(&value).unwrap();
        assert!(json_str.contains(r#"{"ty":"gf""#));
        assert!(json_str.contains(r#""nm":"Linear Gradient Fill""#));
        assert!(json_str.contains(r#""nm":"Radial Gradient Fill""#));
    }

    #[test]
    fn test_default_is_generic() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 120 80">
            <rect x="0" y="0" width="120" height="80" fill="#ff0000"/>
        </svg>"##;

        let json_str = svg_to_lottie_json(svg).expect("Generic conversion should succeed");
        let value: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // Generic mode should keep original dimensions and omit "tgs" flag
        assert!(value.get("tgs").is_none());
        assert_eq!(value["w"], 120);
        assert_eq!(value["h"], 80);
    }

    #[test]
    fn test_gradient_opacity_independence() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <defs>
                <linearGradient id="lg" x1="0" y1="0" x2="1" y2="0" gradientUnits="userSpaceOnUse">
                    <stop offset="0%" stop-color="#ff0000" stop-opacity="0.5"/>
                    <stop offset="100%" stop-color="#0000ff" stop-opacity="1.0"/>
                </linearGradient>
            </defs>
            <rect x="0" y="0" width="100" height="100" fill="url(#lg)" fill-opacity="0.8"/>
        </svg>"##;

        let options = SvgToLottieOptions::default();
        let value = convert_svg_to_value(svg, &options).expect("Conversion should succeed");

        let shapes = value["layers"][0]["shapes"].as_array().unwrap();
        let gf = shapes.iter().find(|s| s["ty"] == "gf").unwrap();

        // Overall shape opacity "o" should be 80.0
        assert_eq!(gf["o"]["k"], 80.0);

        // Gradient stops k array: [offset, r, g, b, ..., offset, a, offset, a]
        // Stop 0 alpha must be exactly 0.5 (not multiplied by fill opacity 0.8)
        let k_arr = gf["g"]["k"]["k"].as_array().unwrap();
        // 2 stops = 8 color floats + 4 alpha floats: [0, r, g, b, 1, r, g, b, 0, a1, 1, a2]
        assert_eq!(k_arr.len(), 12);
        let a1 = k_arr[9].as_f64().unwrap();
        assert!((a1 - 0.5).abs() < 1e-4, "Stop 0 alpha should be 0.5, got {}", a1);
    }

    #[test]
    fn test_stroke_width_group_scale() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200">
            <g transform="scale(3)">
                <line x1="10" y1="10" x2="50" y2="10" stroke="#000000" stroke-width="2"/>
            </g>
        </svg>"##;

        let options = SvgToLottieOptions::default();
        let value = convert_svg_to_value(svg, &options).expect("Conversion should succeed");

        let shapes = value["layers"][0]["shapes"].as_array().unwrap();
        let st = shapes.iter().find(|s| s["ty"] == "st").unwrap();

        let w = st["w"]["k"].as_f64().unwrap();
        assert!((w - 6.0).abs() < 1e-3, "Stroke width should be scaled by group transform (2 * 3 = 6.0), got {}", w);
    }

    #[test]
    fn test_gradient_transform_order() {
        // Gradient starts at (10, 0) and ends at (20, 0)
        // gradientTransform="rotate(90)" rotates points: (10, 0) -> (0, 10), (20, 0) -> (0, 20)
        // Group transform translates: (0, 10) -> (100, 10), (0, 20) -> (100, 20)
        // If order were reversed: translate first (110, 0), then rotate -> (0, 110)
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200">
            <defs>
                <linearGradient id="lg" x1="10" y1="0" x2="20" y2="0" gradientUnits="userSpaceOnUse" gradientTransform="rotate(90)">
                    <stop offset="0%" stop-color="#ff0000"/>
                    <stop offset="100%" stop-color="#0000ff"/>
                </linearGradient>
            </defs>
            <g transform="translate(100, 0)">
                <rect x="0" y="0" width="50" height="50" fill="url(#lg)"/>
            </g>
        </svg>"##;

        let options = SvgToLottieOptions::default();
        let value = convert_svg_to_value(svg, &options).expect("Conversion should succeed");

        let shapes = value["layers"][0]["shapes"].as_array().unwrap();
        let gf = shapes.iter().find(|s| s["ty"] == "gf").unwrap();

        let s_x = gf["s"]["k"][0].as_f64().unwrap();
        let s_y = gf["s"]["k"][1].as_f64().unwrap();

        assert!((s_x - 100.0).abs() < 1e-2, "Expected start X ~ 100.0, got {}", s_x);
        assert!((s_y - 10.0).abs() < 1e-2, "Expected start Y ~ 10.0, got {}", s_y);
    }

    #[test]
    fn test_tgs_compression() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <path d="M 10 10 L 90 90 L 10 90 Z" fill="#336699" />
        </svg>"##;

        let tgs_bytes = svg_to_tgs(svg).expect("TGS generation should succeed");
        assert!(!tgs_bytes.is_empty());
        assert_eq!(tgs_bytes[0], 0x1f);
        assert_eq!(tgs_bytes[1], 0x8b);
    }

    #[test]
    fn test_padding_scaling() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <rect x="0" y="0" width="100" height="100" fill="#ff0000"/>
        </svg>"##;

        // 1. Without padding (default, full bleed):
        let opt_no_pad = SvgToLottieOptions::telegram_custom_emoji();
        let val_no_pad = convert_svg_to_value(svg, &opt_no_pad).unwrap();
        let shapes = val_no_pad["layers"][0]["shapes"].as_array().unwrap();
        let sh = shapes.iter().find(|s| s["ty"] == "sh").unwrap();
        let vertices = sh["ks"]["k"]["v"].as_array().unwrap();
        let min_x = vertices.iter().map(|p| p[0].as_f64().unwrap()).fold(f64::INFINITY, f64::min);
        let max_x = vertices.iter().map(|p| p[0].as_f64().unwrap()).fold(f64::NEG_INFINITY, f64::max);
        assert!((min_x - 0.0).abs() < 1e-2, "Expected min_x 0.0, got {}", min_x);
        assert!((max_x - 512.0).abs() < 1e-2, "Expected max_x 512.0, got {}", max_x);

        // 2. With 16px padding (480x480 inside 512x512):
        let opt_pad = SvgToLottieOptions::telegram_sticker().with_padding(16);
        let val_pad = convert_svg_to_value(svg, &opt_pad).unwrap();
        let shapes_pad = val_pad["layers"][0]["shapes"].as_array().unwrap();
        let sh_pad = shapes_pad.iter().find(|s| s["ty"] == "sh").unwrap();
        let vertices_pad = sh_pad["ks"]["k"]["v"].as_array().unwrap();
        let min_x_pad = vertices_pad.iter().map(|p| p[0].as_f64().unwrap()).fold(f64::INFINITY, f64::min);
        let max_x_pad = vertices_pad.iter().map(|p| p[0].as_f64().unwrap()).fold(f64::NEG_INFINITY, f64::max);
        assert!((min_x_pad - 16.0).abs() < 1e-2, "Expected min_x 16.0, got {}", min_x_pad);
        assert!((max_x_pad - 496.0).abs() < 1e-2, "Expected max_x 496.0 (16+480), got {}", max_x_pad);
    }
}
