//! PNG output encoder.
//!
//! Pure Rust PNG encoding using the `png` crate.

use crate::error::Result;
use crate::framebuffer::Framebuffer;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

/// PNG encoder for framebuffer output.
pub struct PngEncoder;

impl PngEncoder {
    /// Write a framebuffer to a PNG file.
    ///
    /// # Errors
    ///
    /// Returns an error if file creation or PNG encoding fails.
    pub fn write_to_file<P: AsRef<Path>>(fb: &Framebuffer, path: P) -> Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);

        let mut encoder = png::Encoder::new(writer, fb.width(), fb.height());
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder.write_header()?;
        // Use compact pixels to handle stride padding
        writer.write_image_data(&fb.to_compact_pixels())?;

        Ok(())
    }

    /// Encode a framebuffer to PNG bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if PNG encoding fails.
    pub fn to_bytes(fb: &Framebuffer) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();

        {
            let mut encoder = png::Encoder::new(&mut buffer, fb.width(), fb.height());
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);

            let mut writer = encoder.write_header()?;
            // Use compact pixels to handle stride padding
            writer.write_image_data(&fb.to_compact_pixels())?;
        }

        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Rgba;

    #[test]
    fn test_png_to_bytes() {
        let mut fb = Framebuffer::new(10, 10).unwrap();
        fb.clear(Rgba::RED);

        let bytes = PngEncoder::to_bytes(&fb).unwrap();
        // PNG magic bytes
        assert_eq!(&bytes[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    }
}
