# svg2lottie

[![Crates.io](https://img.shields.io/crates/v/svg2lottie.svg)](https://crates.io/crates/svg2lottie)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

Converts SVG vector files into Lottie JSON and Telegram `.tgs` stickers and custom emojis. Pure Rust, no C/C++ dependencies.

Built on [`usvg`](https://github.com/RazrFalcon/resvg) — handles static vector geometry, transforms, bezier curves, arc approximations, group opacity, dashed strokes (`stroke-dasharray`), linear and radial gradients (fills and strokes). Dynamic SVG animations (SMIL/CSS keyframes) and pattern fills are not supported. It packages static vector artwork into a valid, looping Lottie timeline composition required by Telegram and Lottie players.

JSON output uses `serde_json` with `preserve_order` to keep `"ty"` as the first key in shape objects — this is required for Telegram's `rlottie` SAX parser, which otherwise skips properties and renders blank frames. See [details below](#telegram-compatibility).

## Installation

```bash
cargo install svg2lottie
```

Or as a library:

```toml
[dependencies]
svg2lottie = "0.1.2"
```

## CLI

```bash
# Standard Lottie JSON
svg2lottie input.svg -o output.json
svg2lottie input.svg -o output.json --pretty

# Telegram Custom Emoji (512x512, full-bleed 0px padding)
svg2lottie input.svg -o emoji.tgs

# Telegram Sticker (512x512, 16px padding -> 480x480 box)
svg2lottie input.svg -o sticker.tgs --padding 16

# Pipe from stdin
cat input.svg | svg2lottie - --tgs > sticker.tgs
```

| Option | Description | Default |
|---|---|---|
| `<INPUT_SVG>` | Input SVG file, or `-` for stdin | *required* |
| `-o, --output` | Output file (`.json` or `.tgs`), or `-` for stdout | `stdout` |
| `--tgs` | Force TGS output (auto-detected from `.tgs` extension) | off |
| `--width` | Canvas width | SVG width (or 512 for TGS) |
| `--height` | Canvas height | SVG height (or 512 for TGS) |
| `--padding` | Inset padding per side (0 for full-bleed emoji, 16 for stickers) | `0` |
| `--fps` | Framerate (Telegram accepts 30 or 60) | `60` |
| `--duration-frames` | Total frame count | `60` |
| `-p, --pretty` | Pretty-print JSON | off |

## Library

```rust
use svg2lottie::{convert_svg_to_json, convert_svg_to_tgs, SvgToLottieOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
        <circle cx="50" cy="50" r="40" fill="#ffcc00" />
    </svg>"#;

    // Generic Lottie JSON (original SVG dimensions, 60fps, 60 frames)
    let json = svg2lottie::svg_to_lottie_json(svg)?;

    // Telegram Custom Emoji (512x512, 0px padding, gzipped .tgs)
    let tgs = svg2lottie::svg_to_tgs(svg)?;

    // Telegram Sticker with 16px padding (480x480 box)
    let sticker_opts = SvgToLottieOptions::telegram_sticker().with_padding(16);
    let sticker_tgs = convert_svg_to_tgs(svg, &sticker_opts)?;

    // Custom Lottie configuration
    let opts = SvgToLottieOptions::generic(Some(800), Some(600), 30)
        .with_duration_frames(90)
        .with_name("Vector Graphic");
    let custom_json = convert_svg_to_json(svg, &opts)?;

    Ok(())
}
```

## Telegram compatibility

Telegram clients use Samsung's `rlottie` for rendering. `rlottie` parses JSON as a stream — if `"ks"` or `"nm"` appear before `"ty"` in a shape object, the parser skips them and the shape renders as blank.

This crate serializes with `serde_json`'s `preserve_order` feature and constructs all JSON objects with `"ty"` first. This is an implementation-level guarantee, not a schema-level one — if you manipulate the output JSON with tools that reorder keys, it may break.

## License

[MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
