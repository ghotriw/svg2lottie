use std::fmt;

#[derive(Debug)]
pub enum SvgToLottieError {
    SvgParseError(String),
    InvalidDimensions { width: f32, height: f32 },
    NoDrawableShapes,
    SerializationError(String),
    CompressionError(String),
    IoError(std::io::Error),
}

impl fmt::Display for SvgToLottieError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SvgParseError(msg) => write!(f, "Failed to parse SVG: {}", msg),
            Self::InvalidDimensions { width, height } => {
                write!(f, "Invalid SVG dimensions: width={}, height={}", width, height)
            }
            Self::NoDrawableShapes => {
                write!(f, "SVG does not contain any drawable vector shapes")
            }
            Self::SerializationError(msg) => {
                write!(f, "Failed to serialize Lottie JSON: {}", msg)
            }
            Self::CompressionError(msg) => {
                write!(f, "Failed to compress Lottie to TGS: {}", msg)
            }
            Self::IoError(err) => write!(f, "IO error: {}", err),
        }
    }
}

impl std::error::Error for SvgToLottieError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for SvgToLottieError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}
