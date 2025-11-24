//! Output encoders (PNG, SVG, terminal).

mod png_encoder;
mod terminal;

pub use png_encoder::PngEncoder;
pub use terminal::{TerminalEncoder, TerminalMode};
