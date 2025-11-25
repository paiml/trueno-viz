//! Geometry types for Grammar of Graphics.
//!
//! Defines visual representations of data.

use super::aes::Aes;
use super::stat::Stat;

/// Shape types for point geometries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PointShape {
    /// Filled circle.
    #[default]
    Circle,
    /// Filled square.
    Square,
    /// Filled triangle.
    Triangle,
    /// Diamond shape.
    Diamond,
    /// Cross (+).
    Cross,
    /// X shape.
    X,
}

/// Geometry type specification.
#[derive(Debug, Clone)]
pub enum GeomType {
    /// Points.
    Point {
        /// Point shape.
        shape: PointShape,
    },
    /// Lines connecting points.
    Line {
        /// Line width.
        width: f32,
    },
    /// Area under a line.
    Area {
        /// Fill alpha.
        alpha: f32,
    },
    /// Bars.
    Bar {
        /// Bar width (0-1 fraction of available space).
        width: f32,
    },
    /// Histogram bars.
    Histogram {
        /// Number of bins.
        bins: usize,
    },
    /// Box plot.
    Boxplot,
    /// Violin plot.
    Violin,
    /// Tile/rectangle (for heatmaps).
    Tile,
    /// Text labels.
    Text,
    /// Horizontal line.
    Hline {
        /// Y intercept.
        yintercept: f32,
    },
    /// Vertical line.
    Vline {
        /// X intercept.
        xintercept: f32,
    },
    /// Smooth curve (loess/lm).
    Smooth {
        /// Smoothing method.
        method: SmoothMethod,
    },
}

/// Smoothing method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SmoothMethod {
    /// Local polynomial regression (LOESS).
    #[default]
    Loess,
    /// Linear regression.
    Linear,
}

/// A geometry layer.
#[derive(Debug, Clone)]
pub struct Geom {
    /// The geometry type.
    pub geom_type: GeomType,
    /// Layer-specific aesthetics.
    pub aes: Option<Aes>,
    /// Statistical transformation.
    pub stat: Option<Stat>,
}

impl Geom {
    /// Create a point geometry.
    #[must_use]
    pub fn point() -> Self {
        Self {
            geom_type: GeomType::Point {
                shape: PointShape::Circle,
            },
            aes: None,
            stat: None,
        }
    }

    /// Create a line geometry.
    #[must_use]
    pub fn line() -> Self {
        Self {
            geom_type: GeomType::Line { width: 1.0 },
            aes: None,
            stat: None,
        }
    }

    /// Create an area geometry.
    #[must_use]
    pub fn area() -> Self {
        Self {
            geom_type: GeomType::Area { alpha: 0.3 },
            aes: None,
            stat: None,
        }
    }

    /// Create a bar geometry.
    #[must_use]
    pub fn bar() -> Self {
        Self {
            geom_type: GeomType::Bar { width: 0.8 },
            aes: None,
            stat: Some(Stat::Count),
        }
    }

    /// Create a histogram geometry.
    #[must_use]
    pub fn histogram() -> Self {
        Self {
            geom_type: GeomType::Histogram { bins: 30 },
            aes: None,
            stat: Some(Stat::Bin { bins: 30 }),
        }
    }

    /// Create a box plot geometry.
    #[must_use]
    pub fn boxplot() -> Self {
        Self {
            geom_type: GeomType::Boxplot,
            aes: None,
            stat: Some(Stat::Boxplot),
        }
    }

    /// Create a violin plot geometry.
    #[must_use]
    pub fn violin() -> Self {
        Self {
            geom_type: GeomType::Violin,
            aes: None,
            stat: Some(Stat::Density),
        }
    }

    /// Create a tile geometry (for heatmaps).
    #[must_use]
    pub fn tile() -> Self {
        Self {
            geom_type: GeomType::Tile,
            aes: None,
            stat: None,
        }
    }

    /// Create a text geometry.
    #[must_use]
    pub fn text() -> Self {
        Self {
            geom_type: GeomType::Text,
            aes: None,
            stat: None,
        }
    }

    /// Create a horizontal line.
    #[must_use]
    pub fn hline(yintercept: f32) -> Self {
        Self {
            geom_type: GeomType::Hline { yintercept },
            aes: None,
            stat: None,
        }
    }

    /// Create a vertical line.
    #[must_use]
    pub fn vline(xintercept: f32) -> Self {
        Self {
            geom_type: GeomType::Vline { xintercept },
            aes: None,
            stat: None,
        }
    }

    /// Create a smooth line.
    #[must_use]
    pub fn smooth() -> Self {
        Self {
            geom_type: GeomType::Smooth {
                method: SmoothMethod::Loess,
            },
            aes: None,
            stat: Some(Stat::Smooth),
        }
    }

    /// Set the point shape.
    #[must_use]
    pub fn shape(mut self, shape: PointShape) -> Self {
        if let GeomType::Point { shape: ref mut s } = self.geom_type {
            *s = shape;
        }
        self
    }

    /// Set the line/bar width.
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        match &mut self.geom_type {
            GeomType::Line { width: ref mut w } => *w = width,
            GeomType::Bar { width: ref mut w } => *w = width,
            _ => {}
        }
        self
    }

    /// Set the number of bins.
    #[must_use]
    pub fn bins(mut self, bins: usize) -> Self {
        if let GeomType::Histogram { bins: ref mut b } = self.geom_type {
            *b = bins;
            self.stat = Some(Stat::Bin { bins });
        }
        self
    }

    /// Set the area alpha.
    #[must_use]
    pub fn alpha(mut self, alpha: f32) -> Self {
        if let GeomType::Area { alpha: ref mut a } = self.geom_type {
            *a = alpha.clamp(0.0, 1.0);
        }
        self
    }

    /// Add layer-specific aesthetics.
    #[must_use]
    pub fn aes(mut self, aes: Aes) -> Self {
        self.aes = Some(aes);
        self
    }

    /// Set statistical transformation.
    #[must_use]
    pub fn stat(mut self, stat: Stat) -> Self {
        self.stat = Some(stat);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geom_point() {
        let g = Geom::point().shape(PointShape::Square);
        match g.geom_type {
            GeomType::Point { shape } => assert_eq!(shape, PointShape::Square),
            _ => panic!("Expected point geom"),
        }
    }

    #[test]
    fn test_geom_line_width() {
        let g = Geom::line().width(2.5);
        match g.geom_type {
            GeomType::Line { width } => assert!((width - 2.5).abs() < 0.01),
            _ => panic!("Expected line geom"),
        }
    }

    #[test]
    fn test_geom_histogram_bins() {
        let g = Geom::histogram().bins(50);
        match g.geom_type {
            GeomType::Histogram { bins } => assert_eq!(bins, 50),
            _ => panic!("Expected histogram geom"),
        }
    }

    #[test]
    fn test_geom_with_aes() {
        let g = Geom::point().aes(Aes::new().color("category"));
        assert!(g.aes.is_some());
        assert_eq!(g.aes.unwrap().color, Some("category".to_string()));
    }
}
