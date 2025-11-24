//! Scatter plot implementation.
//!
//! Performance target: 10K points < 5ms

use crate::color::Rgba;
use crate::error::{Error, Result};
use crate::framebuffer::Framebuffer;
use crate::scale::{LinearScale, Scale};

/// Builder for creating scatter plots.
#[derive(Debug, Clone)]
pub struct ScatterPlot {
    x_data: Vec<f32>,
    y_data: Vec<f32>,
    color: Rgba,
    point_size: f32,
    alpha: f32,
    width: u32,
    height: u32,
    margin: u32,
}

impl Default for ScatterPlot {
    fn default() -> Self {
        Self::new()
    }
}

impl ScatterPlot {
    /// Create a new scatter plot builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            x_data: Vec::new(),
            y_data: Vec::new(),
            color: Rgba::BLUE,
            point_size: 3.0,
            alpha: 1.0,
            width: 800,
            height: 600,
            margin: 40,
        }
    }

    /// Set the x-axis data.
    #[must_use]
    pub fn x(mut self, data: &[f32]) -> Self {
        self.x_data = data.to_vec();
        self
    }

    /// Set the y-axis data.
    #[must_use]
    pub fn y(mut self, data: &[f32]) -> Self {
        self.y_data = data.to_vec();
        self
    }

    /// Set the point color.
    #[must_use]
    pub fn color(mut self, color: Rgba) -> Self {
        self.color = color;
        self
    }

    /// Set the point size in pixels.
    #[must_use]
    pub fn size(mut self, size: f32) -> Self {
        self.point_size = size;
        self
    }

    /// Set the alpha transparency (0.0 - 1.0).
    #[must_use]
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Set the output dimensions.
    #[must_use]
    pub fn dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Get the number of points.
    #[must_use]
    pub fn point_count(&self) -> usize {
        self.x_data.len().min(self.y_data.len())
    }

    /// Build and validate the scatter plot.
    ///
    /// # Errors
    ///
    /// Returns an error if data is empty or x/y lengths don't match.
    pub fn build(self) -> Result<Self> {
        if self.x_data.is_empty() || self.y_data.is_empty() {
            return Err(Error::EmptyData);
        }

        if self.x_data.len() != self.y_data.len() {
            return Err(Error::DataLengthMismatch {
                x_len: self.x_data.len(),
                y_len: self.y_data.len(),
            });
        }

        Ok(self)
    }

    /// Render the scatter plot to a framebuffer.
    ///
    /// # Errors
    ///
    /// Returns an error if rendering fails.
    pub fn render(&self, fb: &mut Framebuffer) -> Result<()> {
        // Calculate plot area
        let plot_width = self.width - 2 * self.margin;
        let plot_height = self.height - 2 * self.margin;

        // Create scales from data
        let x_scale = LinearScale::from_data(&self.x_data, (self.margin as f32, (self.margin + plot_width) as f32))
            .ok_or(Error::EmptyData)?;

        let y_scale = LinearScale::from_data(&self.y_data, ((self.margin + plot_height) as f32, self.margin as f32))
            .ok_or(Error::EmptyData)?;

        // Apply alpha to color
        let color = self.color.with_alpha((self.alpha * 255.0) as u8);

        // Render each point
        let point_count = self.point_count();
        for i in 0..point_count {
            let px = x_scale.scale(self.x_data[i]) as i32;
            let py = y_scale.scale(self.y_data[i]) as i32;

            // Draw filled circle (simple box for now)
            let radius = (self.point_size / 2.0) as i32;
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx * dx + dy * dy <= radius * radius {
                        let x = (px + dx) as u32;
                        let y = (py + dy) as u32;
                        if self.alpha < 1.0 {
                            fb.blend_pixel(x, y, color);
                        } else {
                            fb.set_pixel(x, y, color);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Render to a new framebuffer.
    ///
    /// # Errors
    ///
    /// Returns an error if rendering fails.
    pub fn to_framebuffer(&self) -> Result<Framebuffer> {
        let mut fb = Framebuffer::new(self.width, self.height)?;
        fb.clear(Rgba::WHITE);
        self.render(&mut fb)?;
        Ok(fb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scatter_plot_builder() {
        let plot = ScatterPlot::new()
            .x(&[1.0, 2.0, 3.0])
            .y(&[4.0, 5.0, 6.0])
            .color(Rgba::RED)
            .size(5.0)
            .build()
            .unwrap();

        assert_eq!(plot.point_count(), 3);
    }

    #[test]
    fn test_scatter_plot_empty_data() {
        let result = ScatterPlot::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_scatter_plot_length_mismatch() {
        let result = ScatterPlot::new()
            .x(&[1.0, 2.0, 3.0])
            .y(&[4.0, 5.0])
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_scatter_plot_render() {
        let plot = ScatterPlot::new()
            .x(&[1.0, 2.0, 3.0])
            .y(&[4.0, 5.0, 6.0])
            .dimensions(100, 100)
            .build()
            .unwrap();

        let fb = plot.to_framebuffer();
        assert!(fb.is_ok());
    }
}
