//! Statistical transformations for Grammar of Graphics.
//!
//! Transforms data before visualization.

/// Statistical transformation type.
#[derive(Debug, Clone)]
pub enum Stat {
    /// No transformation (identity).
    Identity,
    /// Count occurrences.
    Count,
    /// Bin continuous data.
    Bin {
        /// Number of bins.
        bins: usize,
    },
    /// Compute density estimation.
    Density,
    /// Compute summary statistics (for boxplot).
    Boxplot,
    /// Fit a smooth curve.
    Smooth,
    /// Compute 2D binning (for tile/heatmap).
    Bin2d {
        /// Number of x bins.
        bins_x: usize,
        /// Number of y bins.
        bins_y: usize,
    },
    /// Sum values.
    Sum,
    /// Mean values.
    Mean,
}

impl Stat {
    /// Create an identity stat (no transformation).
    #[must_use]
    pub fn identity() -> Self {
        Stat::Identity
    }

    /// Create a count stat.
    #[must_use]
    pub fn count() -> Self {
        Stat::Count
    }

    /// Create a binning stat.
    #[must_use]
    pub fn bin(bins: usize) -> Self {
        Stat::Bin { bins }
    }

    /// Create a density estimation stat.
    #[must_use]
    pub fn density() -> Self {
        Stat::Density
    }

    /// Create a boxplot stat.
    #[must_use]
    pub fn boxplot() -> Self {
        Stat::Boxplot
    }

    /// Create a smooth stat.
    #[must_use]
    pub fn smooth() -> Self {
        Stat::Smooth
    }

    /// Create a 2D binning stat.
    #[must_use]
    pub fn bin2d(bins_x: usize, bins_y: usize) -> Self {
        Stat::Bin2d { bins_x, bins_y }
    }

    /// Create a sum stat.
    #[must_use]
    pub fn sum() -> Self {
        Stat::Sum
    }

    /// Create a mean stat.
    #[must_use]
    pub fn mean() -> Self {
        Stat::Mean
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat_bin() {
        let stat = Stat::bin(20);
        match stat {
            Stat::Bin { bins } => assert_eq!(bins, 20),
            _ => panic!("Expected Bin stat"),
        }
    }

    #[test]
    fn test_stat_bin2d() {
        let stat = Stat::bin2d(10, 15);
        match stat {
            Stat::Bin2d { bins_x, bins_y } => {
                assert_eq!(bins_x, 10);
                assert_eq!(bins_y, 15);
            }
            _ => panic!("Expected Bin2d stat"),
        }
    }
}
