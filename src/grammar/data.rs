//! Data abstraction for Grammar of Graphics.
//!
//! Provides a simple columnar data structure for visualization.

use std::collections::HashMap;

/// A value in a data frame.
#[derive(Debug, Clone, PartialEq)]
pub enum DataValue {
    /// A numeric value.
    Number(f32),
    /// A text value.
    Text(String),
    /// A missing value.
    Null,
}

impl DataValue {
    /// Get as f32, or None if not a number.
    #[must_use]
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            DataValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Get as string.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            DataValue::Text(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

impl From<f32> for DataValue {
    fn from(v: f32) -> Self {
        DataValue::Number(v)
    }
}

impl From<&str> for DataValue {
    fn from(s: &str) -> Self {
        DataValue::Text(s.to_string())
    }
}

impl From<String> for DataValue {
    fn from(s: String) -> Self {
        DataValue::Text(s)
    }
}

/// A simple columnar data frame.
#[derive(Debug, Clone, Default)]
pub struct DataFrame {
    /// Column data keyed by column name.
    columns: HashMap<String, Vec<DataValue>>,
    /// Number of rows.
    n_rows: usize,
}

impl DataFrame {
    /// Create a new empty data frame.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create from x and y arrays.
    #[must_use]
    pub fn from_xy(x: &[f32], y: &[f32]) -> Self {
        let n = x.len().min(y.len());
        let mut df = Self::new();
        df.add_column_f32("x", &x[..n]);
        df.add_column_f32("y", &y[..n]);
        df
    }

    /// Create from a single data array.
    #[must_use]
    pub fn from_data(data: &[f32]) -> Self {
        let mut df = Self::new();
        df.add_column_f32("data", data);
        df
    }

    /// Add a numeric column.
    pub fn add_column_f32(&mut self, name: &str, data: &[f32]) {
        let values: Vec<DataValue> = data.iter().map(|&v| DataValue::Number(v)).collect();
        self.n_rows = self.n_rows.max(values.len());
        self.columns.insert(name.to_string(), values);
    }

    /// Add a text column.
    pub fn add_column_str(&mut self, name: &str, data: &[&str]) {
        let values: Vec<DataValue> = data
            .iter()
            .map(|&s| DataValue::Text(s.to_string()))
            .collect();
        self.n_rows = self.n_rows.max(values.len());
        self.columns.insert(name.to_string(), values);
    }

    /// Get a column as f32 values.
    #[must_use]
    pub fn get_f32(&self, name: &str) -> Option<Vec<f32>> {
        self.columns
            .get(name)
            .map(|col| col.iter().filter_map(|v| v.as_f32()).collect())
    }

    /// Get a column.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&[DataValue]> {
        self.columns.get(name).map(|v| v.as_slice())
    }

    /// Get number of rows.
    #[must_use]
    pub fn nrow(&self) -> usize {
        self.n_rows
    }

    /// Get number of columns.
    #[must_use]
    pub fn ncol(&self) -> usize {
        self.columns.len()
    }

    /// Check if a column exists.
    #[must_use]
    pub fn has_column(&self, name: &str) -> bool {
        self.columns.contains_key(name)
    }

    /// Get column names.
    #[must_use]
    pub fn columns(&self) -> Vec<&str> {
        self.columns.keys().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dataframe_from_xy() {
        let df = DataFrame::from_xy(&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0]);
        assert_eq!(df.nrow(), 3);
        assert_eq!(df.ncol(), 2);
        assert!(df.has_column("x"));
        assert!(df.has_column("y"));
    }

    #[test]
    fn test_dataframe_get_f32() {
        let df = DataFrame::from_xy(&[1.0, 2.0], &[3.0, 4.0]);
        let x = df.get_f32("x").unwrap();
        assert_eq!(x, vec![1.0, 2.0]);
    }

    #[test]
    fn test_data_value_conversions() {
        let num: DataValue = 42.0f32.into();
        assert_eq!(num.as_f32(), Some(42.0));

        let text: DataValue = "hello".into();
        assert_eq!(text.as_str(), Some("hello"));
    }
}
