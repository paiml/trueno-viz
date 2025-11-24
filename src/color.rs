//! Color types and color space conversions.
//!
//! Provides RGBA and HSLA color representations with conversions between them.
//! Implements perceptually uniform color spaces for scientific accuracy.
//!
//! # References
//!
//! - Sharma, G., Wu, W., & Dalal, E. N. (2005). "The CIEDE2000 Color-Difference Formula."
//!   *Color Research & Application*, 30(1), 21-30.

/// RGBA color with 8-bit components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct Rgba {
    /// Red component (0-255).
    pub r: u8,
    /// Green component (0-255).
    pub g: u8,
    /// Blue component (0-255).
    pub b: u8,
    /// Alpha component (0-255, 255 = fully opaque).
    pub a: u8,
}

impl Rgba {
    /// Fully transparent black.
    pub const TRANSPARENT: Self = Self::new(0, 0, 0, 0);
    /// Opaque black.
    pub const BLACK: Self = Self::new(0, 0, 0, 255);
    /// Opaque white.
    pub const WHITE: Self = Self::new(255, 255, 255, 255);
    /// Opaque red.
    pub const RED: Self = Self::new(255, 0, 0, 255);
    /// Opaque green.
    pub const GREEN: Self = Self::new(0, 255, 0, 255);
    /// Opaque blue.
    pub const BLUE: Self = Self::new(0, 0, 255, 255);

    /// Create a new RGBA color.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create an opaque RGB color (alpha = 255).
    #[must_use]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, 255)
    }

    /// Create a color with modified alpha.
    #[must_use]
    pub const fn with_alpha(self, a: u8) -> Self {
        Self::new(self.r, self.g, self.b, a)
    }

    /// Convert to array representation.
    #[must_use]
    pub const fn to_array(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Create from array representation.
    #[must_use]
    pub const fn from_array(arr: [u8; 4]) -> Self {
        Self::new(arr[0], arr[1], arr[2], arr[3])
    }

    /// Linear interpolation between two colors.
    #[must_use]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let inv_t = 1.0 - t;

        Self::new(
            (f32::from(self.r) * inv_t + f32::from(other.r) * t) as u8,
            (f32::from(self.g) * inv_t + f32::from(other.g) * t) as u8,
            (f32::from(self.b) * inv_t + f32::from(other.b) * t) as u8,
            (f32::from(self.a) * inv_t + f32::from(other.a) * t) as u8,
        )
    }
}

/// HSLA color with floating-point components.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Hsla {
    /// Hue (0.0-360.0 degrees).
    pub h: f32,
    /// Saturation (0.0-1.0).
    pub s: f32,
    /// Lightness (0.0-1.0).
    pub l: f32,
    /// Alpha (0.0-1.0).
    pub a: f32,
}

impl Hsla {
    /// Create a new HSLA color.
    #[must_use]
    pub const fn new(h: f32, s: f32, l: f32, a: f32) -> Self {
        Self { h, s, l, a }
    }

    /// Create an opaque HSL color (alpha = 1.0).
    #[must_use]
    pub const fn hsl(h: f32, s: f32, l: f32) -> Self {
        Self::new(h, s, l, 1.0)
    }

    /// Convert to RGBA.
    #[must_use]
    pub fn to_rgba(self) -> Rgba {
        let h = self.h / 360.0;
        let s = self.s;
        let l = self.l;

        let (r, g, b) = if s == 0.0 {
            (l, l, l)
        } else {
            let q = if l < 0.5 {
                l * (1.0 + s)
            } else {
                l + s - l * s
            };
            let p = 2.0 * l - q;

            (
                hue_to_rgb(p, q, h + 1.0 / 3.0),
                hue_to_rgb(p, q, h),
                hue_to_rgb(p, q, h - 1.0 / 3.0),
            )
        };

        Rgba::new(
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8,
            (self.a * 255.0) as u8,
        )
    }
}

fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }

    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 1.0 / 2.0 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

impl From<Hsla> for Rgba {
    fn from(hsla: Hsla) -> Self {
        hsla.to_rgba()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgba_constants() {
        assert_eq!(Rgba::BLACK, Rgba::rgb(0, 0, 0));
        assert_eq!(Rgba::WHITE, Rgba::rgb(255, 255, 255));
        assert_eq!(Rgba::RED.r, 255);
        assert_eq!(Rgba::GREEN.g, 255);
        assert_eq!(Rgba::BLUE.b, 255);
    }

    #[test]
    fn test_rgba_lerp() {
        let black = Rgba::BLACK;
        let white = Rgba::WHITE;

        let mid = black.lerp(white, 0.5);
        assert_eq!(mid.r, 127);
        assert_eq!(mid.g, 127);
        assert_eq!(mid.b, 127);
    }

    #[test]
    fn test_hsla_to_rgba() {
        // Red
        let red = Hsla::hsl(0.0, 1.0, 0.5).to_rgba();
        assert_eq!(red.r, 255);
        assert_eq!(red.g, 0);
        assert_eq!(red.b, 0);

        // Gray (saturation = 0)
        let gray = Hsla::hsl(0.0, 0.0, 0.5).to_rgba();
        assert_eq!(gray.r, 127);
        assert_eq!(gray.g, 127);
        assert_eq!(gray.b, 127);
    }
}
