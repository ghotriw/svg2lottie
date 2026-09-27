#[derive(Debug, Clone)]
pub struct SvgToLottieOptions {
    /// Canvas width in pixels. Defaults to 512 for Telegram TGS mode or original SVG width.
    pub width: Option<u32>,
    /// Canvas height in pixels. Defaults to 512 for Telegram TGS mode or original SVG height.
    pub height: Option<u32>,
    /// Framerate of the animation. Telegram requires 30 or 60 fps (default: 60).
    pub fps: u32,
    /// Total duration in frames. Telegram allows up to 180 frames @ 60fps (default: 60).
    pub duration_frames: u32,
    /// When true, enables Telegram TGS mode:
    /// - Adds `"tgs": 1` to Lottie JSON root.
    /// - Scales and centers graphics within a 480x480 area inside 512x512 canvas.
    /// - Ensures exact JSON key order for rlottie parser compatibility.
    pub tgs_compatible: bool,
    /// Name of the animation composition.
    pub name: Option<String>,
}

impl Default for SvgToLottieOptions {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            fps: 60,
            duration_frames: 60,
            tgs_compatible: false,
            name: None,
        }
    }
}

impl SvgToLottieOptions {
    /// Presets optimized for Telegram Animated Stickers and Custom Emojis (.tgs):
    /// 512x512 canvas, 60 fps, 1 second (60 frames), centered vector paths with tgs flag.
    pub fn telegram_sticker() -> Self {
        Self {
            width: Some(512),
            height: Some(512),
            fps: 60,
            duration_frames: 60,
            tgs_compatible: true,
            name: Some("Vector Emoji".to_string()),
        }
    }

    /// Presets for generic Lottie JSON without Telegram-specific padding or flags.
    pub fn generic(width: Option<u32>, height: Option<u32>, fps: u32) -> Self {
        Self {
            width,
            height,
            fps,
            duration_frames: fps,
            tgs_compatible: false,
            name: Some("Vector Animation".to_string()),
        }
    }

    pub fn with_fps(mut self, fps: u32) -> Self {
        self.fps = fps;
        self
    }

    pub fn with_dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    pub fn with_duration_frames(mut self, frames: u32) -> Self {
        self.duration_frames = frames;
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_tgs_compatible(mut self, tgs_compatible: bool) -> Self {
        self.tgs_compatible = tgs_compatible;
        self
    }
}
