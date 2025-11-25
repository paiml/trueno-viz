//! Ecosystem integrations.
//!
//! Provides native integration with:
//! - trueno-db: Query result visualization
//! - trueno-graph: Graph layout and visualization
//! - aprender: ML model and result visualization
//! - entrenar: Training metrics visualization

#[cfg(feature = "ml")]
#[cfg_attr(docsrs, doc(cfg(feature = "ml")))]
pub mod aprender;

// TODO: Implement TV-018 (trueno-graph integration)
