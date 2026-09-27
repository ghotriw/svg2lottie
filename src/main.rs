use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;

use clap::Parser;
use svg2lottie::{convert_svg_to_tgs, convert_svg_to_value, SvgToLottieOptions};

#[derive(Parser, Debug)]
#[command(
    name = "svg2lottie",
    version,
    about = "Convert SVG files into Lottie animations and Telegram TGS stickers",
    long_about = "A fast, pure-Rust converter that turns SVG vector files into Lottie JSON and Telegram-compatible TGS animations."
)]
struct Args {
    /// Input SVG file path, or '-' to read from stdin
    #[arg(value_name = "INPUT_SVG")]
    input: PathBuf,

    /// Output file path (.json or .tgs), or '-' to write to stdout
    #[arg(short, long, value_name = "OUTPUT")]
    output: Option<PathBuf>,

    /// Force output as compressed Telegram TGS (gzipped Lottie).
    /// Defaults to true if output path ends with '.tgs'.
    #[arg(long)]
    tgs: bool,

    /// Canvas width in pixels (defaults to 512 for Telegram TGS)
    #[arg(long)]
    width: Option<u32>,

    /// Canvas height in pixels (defaults to 512 for Telegram TGS)
    #[arg(long)]
    height: Option<u32>,

    /// Animation framerate (Telegram requires 30 or 60 fps)
    #[arg(long, default_value_t = 60)]
    fps: u32,

    /// Total duration in frames (defaults to 60 frames = 1.0 second @ 60fps)
    #[arg(long, default_value_t = 60)]
    duration_frames: u32,

    /// Inset padding in pixels on each side (defaults to 0 for full bleed, 16 for standard stickers)
    #[arg(long, default_value_t = 0)]
    padding: u32,

    /// Pretty-print output JSON (only applies to uncompressed JSON output)
    #[arg(short, long)]
    pretty: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let svg_content = if args.input.to_str() == Some("-") {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        buffer
    } else {
        fs::read_to_string(&args.input)?
    };

    let is_tgs = args.tgs
        || args
            .output
            .as_ref()
            .and_then(|p| p.extension())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("tgs"));

    let options = SvgToLottieOptions {
        width: args.width.or(if is_tgs { Some(512) } else { None }),
        height: args.height.or(if is_tgs { Some(512) } else { None }),
        fps: args.fps,
        duration_frames: args.duration_frames,
        tgs_compatible: is_tgs,
        padding: args.padding,
        name: args
            .input
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string()),
    };

    if is_tgs {
        let tgs_bytes = convert_svg_to_tgs(&svg_content, &options)?;
        match args.output {
            Some(ref path) if path.to_str() != Some("-") => {
                fs::write(path, tgs_bytes)?;
                eprintln!("Successfully created TGS: {}", path.display());
            }
            _ => {
                io::stdout().write_all(&tgs_bytes)?;
            }
        }
    } else {
        let json_value = convert_svg_to_value(&svg_content, &options)?;
        let json_str = if args.pretty {
            serde_json::to_string_pretty(&json_value)?
        } else {
            serde_json::to_string(&json_value)?
        };

        match args.output {
            Some(ref path) if path.to_str() != Some("-") => {
                fs::write(path, json_str)?;
                eprintln!("Successfully created Lottie JSON: {}", path.display());
            }
            _ => {
                io::stdout().write_all(json_str.as_bytes())?;
                println!();
            }
        }
    }

    Ok(())
}
