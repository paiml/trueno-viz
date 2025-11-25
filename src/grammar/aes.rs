//! Aesthetic mappings for Grammar of Graphics.
//!
//! Maps data columns to visual properties.

use crate::color::Rgba;

/// Aesthetic mapping specification.
///
/// Maps data columns to visual properties like x, y, color, size, shape.
#[derive(Debug, Clone, Default)]
pub struct Aes {
    /// X position mapping (column name).
    pub x: Option<String>,
    /// Y position mapping (column name).
    pub y: Option<String>,
    /// Color mapping (column name).
    pub color: Option<String>,
    /// Size mapping (column name).
    pub size: Option<String>,
    /// Shape mapping (column name).
    pub shape: Option<String>,
    /// Alpha/opacity mapping (column name).
    pub alpha: Option<String>,
    /// Fill color mapping (column name).
    pub fill: Option<String>,
    /// Group mapping (column name).
    pub group: Option<String>,
    /// Label mapping (column name).
    pub label: Option<String>,

    // Fixed values (not data-mapped)
    /// Fixed color value.
    pub color_value: Option<Rgba>,
    /// Fixed size value.
    pub size_value: Option<f32>,
    /// Fixed alpha value.
    pub alpha_value: Option<f32>,
}

impl Aes {
    /// Create a new aesthetic mapping.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Map x position to a column.
    #[must_use]
    pub fn x(mut self, column: &str) -> Self {
        self.x = Some(column.to_string());
        self
    }

    /// Map y position to a column.
    #[must_use]
    pub fn y(mut self, column: &str) -> Self {
        self.y = Some(column.to_string());
        self
    }

    /// Map color to a column.
    #[must_use]
    pub fn color(mut self, column: &str) -> Self {
        self.color = Some(column.to_string());
        self
    }

    /// Map size to a column.
    #[must_use]
    pub fn size(mut self, column: &str) -> Self {
        self.size = Some(column.to_string());
        self
    }

    /// Map shape to a column.
    #[must_use]
    pub fn shape(mut self, column: &str) -> Self {
        self.shape = Some(column.to_string());
        self
    }

    /// Map alpha/opacity to a column.
    #[must_use]
    pub fn alpha(mut self, column: &str) -> Self {
        self.alpha = Some(column.to_string());
        self
    }

    /// Map fill color to a column.
    #[must_use]
    pub fn fill(mut self, column: &str) -> Self {
        self.fill = Some(column.to_string());
        self
    }

    /// Map group to a column.
    #[must_use]
    pub fn group(mut self, column: &str) -> Self {
        self.group = Some(column.to_string());
        self
    }

    /// Map label to a column.
    #[must_use]
    pub fn label(mut self, column: &str) -> Self {
        self.label = Some(column.to_string());
        self
    }

    /// Set a fixed color value.
    #[must_use]
    pub fn color_value(mut self, color: Rgba) -> Self {
        self.color_value = Some(color);
        self
    }

    /// Set a fixed size value.
    #[must_use]
    pub fn size_value(mut self, size: f32) -> Self {
        self.size_value = Some(size);
        self
    }

    /// Set a fixed alpha value.
    #[must_use]
    pub fn alpha_value(mut self, alpha: f32) -> Self {
        self.alpha_value = Some(alpha.clamp(0.0, 1.0));
        self
    }

    /// Merge another Aes, with other taking precedence.
    #[must_use]
    pub fn merge(&self, other: &Aes) -> Aes {
        Aes {
            x: other.x.clone().or_else(|| self.x.clone()),
            y: other.y.clone().or_else(|| self.y.clone()),
            color: other.color.clone().or_else(|| self.color.clone()),
            size: other.size.clone().or_else(|| self.size.clone()),
            shape: other.shape.clone().or_else(|| self.shape.clone()),
            alpha: other.alpha.clone().or_else(|| self.alpha.clone()),
            fill: other.fill.clone().or_else(|| self.fill.clone()),
            group: other.group.clone().or_else(|| self.group.clone()),
            label: other.label.clone().or_else(|| self.label.clone()),
            color_value: other.color_value.or(self.color_value),
            size_value: other.size_value.or(self.size_value),
            alpha_value: other.alpha_value.or(self.alpha_value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aes_builder() {
        let aes = Aes::new()
            .x("xvar")
            .y("yvar")
            .color("category")
            .size_value(5.0);

        assert_eq!(aes.x, Some("xvar".to_string()));
        assert_eq!(aes.y, Some("yvar".to_string()));
        assert_eq!(aes.color, Some("category".to_string()));
        assert_eq!(aes.size_value, Some(5.0));
    }

    #[test]
    fn test_aes_merge() {
        let base = Aes::new().x("x").y("y").color_value(Rgba::RED);
        let override_aes = Aes::new().y("y2").size_value(3.0);

        let merged = base.merge(&override_aes);
        assert_eq!(merged.x, Some("x".to_string())); // From base
        assert_eq!(merged.y, Some("y2".to_string())); // Overridden
        assert_eq!(merged.color_value, Some(Rgba::RED)); // From base
        assert_eq!(merged.size_value, Some(3.0)); // From override
    }
}
