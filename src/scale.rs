//! Scale functions for data-to-visual mappings.
//!
//! Scales transform data values to visual properties (position, color, size).
//! Based on the Grammar of Graphics [Wilkinson 2005].

use crate::color::Rgba;
use crate::error::{Error, Result};

/// Trait for scale functions that map domain values to range values.
pub trait Scale<D, R> {
    /// Transform a domain value to a range value.
    fn scale(&self, value: D) -> R;

    /// Get the domain extent.
    fn domain(&self) -> (D, D);

    /// Get the range extent.
    fn range(&self) -> (R, R);
}

/// Linear scale for continuous-to-continuous mapping.
#[derive(Debug, Clone, Copy)]
pub struct LinearScale {
    domain_min: f32,
    domain_max: f32,
    range_min: f32,
    range_max: f32,
}

impl LinearScale {
    /// Create a new linear scale.
    ///
    /// # Errors
    ///
    /// Returns an error if domain_min equals domain_max.
    pub fn new(domain: (f32, f32), range: (f32, f32)) -> Result<Self> {
        if (domain.0 - domain.1).abs() < f32::EPSILON {
            return Err(Error::ScaleDomain(
                "Domain min and max cannot be equal".to_string(),
            ));
        }

        Ok(Self {
            domain_min: domain.0,
            domain_max: domain.1,
            range_min: range.0,
            range_max: range.1,
        })
    }

    /// Create a scale from data extent.
    #[must_use]
    pub fn from_data(data: &[f32], range: (f32, f32)) -> Option<Self> {
        if data.is_empty() {
            return None;
        }

        let min = data.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = data.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

        Self::new((min, max), range).ok()
    }

    /// Invert the scale (range to domain).
    #[must_use]
    pub fn invert(&self, value: f32) -> f32 {
        let t = (value - self.range_min) / (self.range_max - self.range_min);
        self.domain_min + t * (self.domain_max - self.domain_min)
    }
}

impl Scale<f32, f32> for LinearScale {
    fn scale(&self, value: f32) -> f32 {
        let t = (value - self.domain_min) / (self.domain_max - self.domain_min);
        self.range_min + t * (self.range_max - self.range_min)
    }

    fn domain(&self) -> (f32, f32) {
        (self.domain_min, self.domain_max)
    }

    fn range(&self) -> (f32, f32) {
        (self.range_min, self.range_max)
    }
}

/// Logarithmic scale for continuous-to-continuous mapping.
#[derive(Debug, Clone, Copy)]
pub struct LogScale {
    domain_min: f32,
    domain_max: f32,
    range_min: f32,
    range_max: f32,
    base: f32,
}

impl LogScale {
    /// Create a new logarithmic scale with base 10.
    ///
    /// # Errors
    ///
    /// Returns an error if domain contains non-positive values.
    pub fn new(domain: (f32, f32), range: (f32, f32)) -> Result<Self> {
        Self::with_base(domain, range, 10.0)
    }

    /// Create a logarithmic scale with a custom base.
    ///
    /// # Errors
    ///
    /// Returns an error if domain contains non-positive values or base is invalid.
    pub fn with_base(domain: (f32, f32), range: (f32, f32), base: f32) -> Result<Self> {
        if domain.0 <= 0.0 || domain.1 <= 0.0 {
            return Err(Error::ScaleDomain(
                "Log scale domain must be positive".to_string(),
            ));
        }

        if base <= 0.0 || base == 1.0 {
            return Err(Error::ScaleDomain(
                "Log scale base must be positive and not 1".to_string(),
            ));
        }

        Ok(Self {
            domain_min: domain.0,
            domain_max: domain.1,
            range_min: range.0,
            range_max: range.1,
            base,
        })
    }
}

impl Scale<f32, f32> for LogScale {
    fn scale(&self, value: f32) -> f32 {
        let log_base = self.base.ln();
        let log_min = self.domain_min.ln() / log_base;
        let log_max = self.domain_max.ln() / log_base;
        let log_val = value.max(f32::MIN_POSITIVE).ln() / log_base;

        let t = (log_val - log_min) / (log_max - log_min);
        self.range_min + t * (self.range_max - self.range_min)
    }

    fn domain(&self) -> (f32, f32) {
        (self.domain_min, self.domain_max)
    }

    fn range(&self) -> (f32, f32) {
        (self.range_min, self.range_max)
    }
}

/// Color scale for mapping values to colors.
#[derive(Debug, Clone)]
pub struct ColorScale {
    colors: Vec<Rgba>,
    domain_min: f32,
    domain_max: f32,
}

impl ColorScale {
    /// Create a new color scale.
    ///
    /// # Errors
    ///
    /// Returns an error if colors is empty or domain is invalid.
    pub fn new(colors: Vec<Rgba>, domain: (f32, f32)) -> Result<Self> {
        if colors.is_empty() {
            return Err(Error::ScaleDomain(
                "Color scale requires at least one color".to_string(),
            ));
        }

        if (domain.0 - domain.1).abs() < f32::EPSILON {
            return Err(Error::ScaleDomain(
                "Domain min and max cannot be equal".to_string(),
            ));
        }

        Ok(Self {
            colors,
            domain_min: domain.0,
            domain_max: domain.1,
        })
    }

    /// Create a sequential blue scale.
    #[must_use]
    pub fn blues(domain: (f32, f32)) -> Option<Self> {
        Self::new(
            vec![
                Rgba::rgb(247, 251, 255),
                Rgba::rgb(198, 219, 239),
                Rgba::rgb(107, 174, 214),
                Rgba::rgb(33, 113, 181),
                Rgba::rgb(8, 48, 107),
            ],
            domain,
        )
        .ok()
    }

    /// Create a diverging red-blue scale.
    #[must_use]
    pub fn red_blue(domain: (f32, f32)) -> Option<Self> {
        Self::new(
            vec![
                Rgba::rgb(178, 24, 43),
                Rgba::rgb(239, 138, 98),
                Rgba::rgb(247, 247, 247),
                Rgba::rgb(103, 169, 207),
                Rgba::rgb(33, 102, 172),
            ],
            domain,
        )
        .ok()
    }

    /// Create a viridis color scale (perceptually uniform).
    #[must_use]
    pub fn viridis(domain: (f32, f32)) -> Option<Self> {
        Self::new(
            vec![
                Rgba::rgb(68, 1, 84),
                Rgba::rgb(59, 82, 139),
                Rgba::rgb(33, 145, 140),
                Rgba::rgb(94, 201, 98),
                Rgba::rgb(253, 231, 37),
            ],
            domain,
        )
        .ok()
    }

    /// Create a magma color scale (sequential, perceptually uniform).
    #[must_use]
    pub fn magma(domain: (f32, f32)) -> Option<Self> {
        Self::new(
            vec![
                Rgba::rgb(0, 0, 4),
                Rgba::rgb(81, 18, 124),
                Rgba::rgb(183, 55, 121),
                Rgba::rgb(252, 137, 97),
                Rgba::rgb(252, 253, 191),
            ],
            domain,
        )
        .ok()
    }

    /// Create a greyscale color scale.
    #[must_use]
    pub fn greyscale(domain: (f32, f32)) -> Option<Self> {
        Self::new(vec![Rgba::BLACK, Rgba::WHITE], domain).ok()
    }

    /// Create a heat color scale (black-red-yellow-white).
    #[must_use]
    pub fn heat(domain: (f32, f32)) -> Option<Self> {
        Self::new(
            vec![
                Rgba::rgb(0, 0, 0),
                Rgba::rgb(128, 0, 0),
                Rgba::rgb(255, 0, 0),
                Rgba::rgb(255, 128, 0),
                Rgba::rgb(255, 255, 0),
                Rgba::rgb(255, 255, 255),
            ],
            domain,
        )
        .ok()
    }
}

impl Scale<f32, Rgba> for ColorScale {
    fn scale(&self, value: f32) -> Rgba {
        let t = ((value - self.domain_min) / (self.domain_max - self.domain_min)).clamp(0.0, 1.0);

        if self.colors.len() == 1 {
            return self.colors[0];
        }

        let segment_count = self.colors.len() - 1;
        let segment = (t * segment_count as f32).floor() as usize;
        let segment = segment.min(segment_count - 1);

        let local_t = t * segment_count as f32 - segment as f32;

        self.colors[segment].lerp(self.colors[segment + 1], local_t)
    }

    fn domain(&self) -> (f32, f32) {
        (self.domain_min, self.domain_max)
    }

    fn range(&self) -> (Rgba, Rgba) {
        (
            *self.colors.first().unwrap_or(&Rgba::BLACK),
            *self.colors.last().unwrap_or(&Rgba::WHITE),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_scale() {
        let scale = LinearScale::new((0.0, 100.0), (0.0, 1.0)).unwrap();
        assert!((scale.scale(0.0) - 0.0).abs() < 0.001);
        assert!((scale.scale(50.0) - 0.5).abs() < 0.001);
        assert!((scale.scale(100.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_linear_scale_invert() {
        let scale = LinearScale::new((0.0, 100.0), (0.0, 1.0)).unwrap();
        assert!((scale.invert(0.5) - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_log_scale() {
        let scale = LogScale::new((1.0, 1000.0), (0.0, 3.0)).unwrap();
        assert!((scale.scale(1.0) - 0.0).abs() < 0.001);
        assert!((scale.scale(10.0) - 1.0).abs() < 0.001);
        assert!((scale.scale(100.0) - 2.0).abs() < 0.001);
        assert!((scale.scale(1000.0) - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_log_scale_invalid_domain() {
        assert!(LogScale::new((-1.0, 100.0), (0.0, 1.0)).is_err());
        assert!(LogScale::new((0.0, 100.0), (0.0, 1.0)).is_err());
    }

    #[test]
    fn test_color_scale() {
        let scale = ColorScale::new(vec![Rgba::BLACK, Rgba::WHITE], (0.0, 1.0)).unwrap();

        let mid = scale.scale(0.5);
        assert!(mid.r > 100 && mid.r < 150);
    }
}
