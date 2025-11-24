//! Geometric primitives for visualization.
//!
//! Provides basic geometric types used for rendering plots.

/// A 2D point with floating-point coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    /// X coordinate.
    pub x: f32,
    /// Y coordinate.
    pub y: f32,
}

impl Point {
    /// Origin point (0, 0).
    pub const ORIGIN: Self = Self::new(0.0, 0.0);

    /// Create a new point.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Calculate the distance to another point.
    #[must_use]
    pub fn distance(self, other: Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Linear interpolation between two points.
    #[must_use]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self::new(
            self.x + (other.x - self.x) * t,
            self.y + (other.y - self.y) * t,
        )
    }
}

/// A line segment between two points.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Line {
    /// Start point.
    pub start: Point,
    /// End point.
    pub end: Point,
}

impl Line {
    /// Create a new line segment.
    #[must_use]
    pub const fn new(start: Point, end: Point) -> Self {
        Self { start, end }
    }

    /// Create a line from coordinates.
    #[must_use]
    pub const fn from_coords(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        Self::new(Point::new(x0, y0), Point::new(x1, y1))
    }

    /// Get the length of the line.
    #[must_use]
    pub fn length(&self) -> f32 {
        self.start.distance(self.end)
    }
}

/// A rectangle defined by position and size.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    /// X coordinate of the top-left corner.
    pub x: f32,
    /// Y coordinate of the top-left corner.
    pub y: f32,
    /// Width of the rectangle.
    pub width: f32,
    /// Height of the rectangle.
    pub height: f32,
}

impl Rect {
    /// Create a new rectangle.
    #[must_use]
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    /// Create a rectangle from two corner points.
    #[must_use]
    pub fn from_corners(top_left: Point, bottom_right: Point) -> Self {
        Self::new(
            top_left.x,
            top_left.y,
            bottom_right.x - top_left.x,
            bottom_right.y - top_left.y,
        )
    }

    /// Check if a point is inside the rectangle.
    #[must_use]
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }

    /// Get the center point of the rectangle.
    #[must_use]
    pub fn center(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Get the area of the rectangle.
    #[must_use]
    pub fn area(&self) -> f32 {
        self.width * self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_distance() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(3.0, 4.0);
        assert!((p1.distance(p2) - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_point_lerp() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(10.0, 10.0);
        let mid = p1.lerp(p2, 0.5);
        assert!((mid.x - 5.0).abs() < 0.001);
        assert!((mid.y - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_line_length() {
        let line = Line::from_coords(0.0, 0.0, 3.0, 4.0);
        assert!((line.length() - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_rect_contains() {
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!(rect.contains(Point::new(5.0, 5.0)));
        assert!(!rect.contains(Point::new(15.0, 5.0)));
    }

    #[test]
    fn test_rect_area() {
        let rect = Rect::new(0.0, 0.0, 10.0, 5.0);
        assert!((rect.area() - 50.0).abs() < 0.001);
    }
}
