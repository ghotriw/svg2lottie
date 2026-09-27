# svg2lottie

[![Crates.io](https://img.shields.io/badge/crates.io-v0.1.0-orange.svg)](https://crates.io)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

Converts SVG files to Lottie JSON and Telegram `.tgs` stickers. Pure Rust, no C/C++ dependencies.

Built on [`usvg`](https://github.com/RazrFalcon/resvg) — handles transforms, bezier curves, arc approximations, linear and radial gradients (fills and strokes). Patterns are not supported.

JSON output uses `serde_json` with `preserve_order` to keep `"ty"` as the first key in shape objects — this is required for Telegram's `rlottie` SAX parser, which otherwise skips properties and renders blank frames. See [details below](#telegram-compatibility).

## Installation

```bash
cargo install svg2lottie
```

Or as a library:

```toml
[dependencies]
svg2lottie = "0.1.0"
```

## CLI

```bash
svg2lottie input.svg -o output.json
svg2lottie input.svg -o output.json --pretty
svg2lottie input.svg -o sticker.tgs
cat input.svg | svg2lottie - --tgs > sticker.tgs
svg2lottie input.svg -o output.json --width 1024 --height 1024 --fps 30
```

| Option | Description | Default |
|---|---|---|
| `<INPUT_SVG>` | Input SVG file, or `-` for stdin | *required* |
| `-o, --output` | Output file (`.json` or `.tgs`), or `-` for stdout | `stdout` |
| `--tgs` | Force TGS output (auto-detected from `.tgs` extension) | off |
| `--width` | Canvas width | SVG width (or 512 for TGS) |
| `--height` | Canvas height | SVG height (or 512 for TGS) |
| `--fps` | Framerate (Telegram accepts 30 or 60) | `60` |
| `--duration-frames` | Total frame count | `60` |
| `-p, --pretty` | Pretty-print JSON | off |

## Library

```rust
use svg2lottie::{convert_svg_to_json, SvgToLottieOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
        <circle cx="50" cy="50" r="40" fill="#ffcc00" />
    </svg>"#;

    // Lottie JSON with default options (dimensions from SVG, 60fps, 60 frames)
    let json = svg2lottie::svg_to_lottie_json(svg)?;

    // Telegram TGS (512x512, gzipped)
    let tgs = svg2lottie::svg_to_tgs(svg)?;

    // Custom options
    let opts = SvgToLottieOptions::generic(Some(800), Some(600), 30)
        .with_duration_frames(90)
        .with_name("My Animation");
    let json = convert_svg_to_json(svg, &opts)?;

    Ok(())
}
```

## Telegram compatibility

Telegram clients use Samsung's `rlottie` for rendering. `rlottie` parses JSON as a stream — if `"ks"` or `"nm"` appear before `"ty"` in a shape object, the parser skips them and the shape renders as blank.

This crate serializes with `serde_json`'s `preserve_order` feature and constructs all JSON objects with `"ty"` first. This is an implementation-level guarantee, not a schema-level one — if you manipulate the output JSON with tools that reorder keys, it may break.

## License

[MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
