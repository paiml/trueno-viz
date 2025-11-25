//! Coordinate systems for Grammar of Graphics.
//!
//! Defines how positions are mapped to the plotting area.

/// Coordinate system type.
#[derive(Debug, Clone)]
pub enum Coord {
    /// Cartesian coordinates (x, y).
    Cartesian {
        /// X axis limits.
        xlim: Option<(f32, f32)>,
        /// Y axis limits.
        ylim: Option<(f32, f32)>,
        /// Whether to flip x and y.
        flip: bool,
    },
    /// Polar coordinates (r, theta).
    Polar {
        /// Start angle in radians.
        start: f32,
        /// Direction: 1 for clockwise, -1 for counter-clockwise.
        direction: i8,
    },
    /// Fixed aspect ratio coordinates.
    Fixed {
        /// Aspect ratio (y/x).
        ratio: f32,
    },
}

impl Default for Coord {
    fn default() -> Self {
        Coord::cartesian()
    }
}

impl Coord {
    /// Create a Cartesian coordinate system.
    #[must_use]
    pub fn cartesian() -> Self {
        Coord::Cartesian {
            xlim: None,
            ylim: None,
            flip: false,
        }
    }

    /// Create a polar coordinate system.
    #[must_use]
    pub fn polar() -> Self {
        Coord::Polar {
            start: 0.0,
            direction: 1,
        }
    }

    /// Create a fixed aspect ratio coordinate system.
    #[must_use]
    pub fn fixed(ratio: f32) -> Self {
        Coord::Fixed { ratio }
    }

    /// Set x-axis limits.
    #[must_use]
    pub fn xlim(mut self, min: f32, max: f32) -> Self {
        if let Coord::Cartesian { ref mut xlim, .. } = self {
            *xlim = Some((min, max));
        }
        self
    }

    /// Set y-axis limits.
    #[must_use]
    pub fn ylim(mut self, min: f32, max: f32) -> Self {
        if let Coord::Cartesian { ref mut ylim, .. } = self {
            *ylim = Some((min, max));
        }
        self
    }

    /// Flip x and y axes.
    #[must_use]
    pub fn flip(mut self) -> Self {
        if let Coord::Cartesian { flip: ref mut f, .. } = self {
            *f = true;
        }
        self
    }

    /// Set polar start angle.
    #[must_use]
    pub fn start_angle(mut self, start: f32) -> Self {
        if let Coord::Polar { start: ref mut s, .. } = self {
            *s = start;
        }
        self
    }

    /// Set polar direction (1 = clockwise, -1 = counter-clockwise).
    #[must_use]
    pub fn direction(mut self, dir: i8) -> Self {
        if let Coord::Polar { direction: ref mut d, .. } = self {
            *d = if dir >= 0 { 1 } else { -1 };
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coord_cartesian() {
        let c = Coord::cartesian().xlim(0.0, 10.0).ylim(-5.0, 5.0);
        match c {
            Coord::Cartesian { xlim, ylim, flip } => {
                assert_eq!(xlim, Some((0.0, 10.0)));
                assert_eq!(ylim, Some((-5.0, 5.0)));
                assert!(!flip);
            }
            _ => panic!("Expected Cartesian"),
        }
    }

    #[test]
    fn test_coord_flip() {
        let c = Coord::cartesian().flip();
        match c {
            Coord::Cartesian { flip, .. } => assert!(flip),
            _ => panic!("Expected Cartesian"),
        }
    }

    #[test]
    fn test_coord_polar() {
        let c = Coord::polar().start_angle(std::f32::consts::PI).direction(-1);
        match c {
            Coord::Polar { start, direction } => {
                assert!((start - std::f32::consts::PI).abs() < 0.001);
                assert_eq!(direction, -1);
            }
            _ => panic!("Expected Polar"),
        }
    }
}
