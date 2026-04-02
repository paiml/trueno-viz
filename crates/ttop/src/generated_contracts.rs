// Auto-generated contract macros for ttop crate — DO NOT EDIT
// These macros mirror the ones in trueno-viz/src/generated_contracts.rs
// for use in the ttop workspace member.

/// Preconditions for equation `display_format` (display-format-v1.yaml).
/// Call: `contract_pre_display_format!(slice_expr)` or `contract_pre_display_format!()`
macro_rules! contract_pre_display_format {
    () => {{}};
    ($input:expr) => {{
        let input = &$input;
        debug_assert!(
            input.len() > 0,
            "Contract display_format: precondition violated — input.len() > 0"
        );
    }};
}

/// Preconditions for equation `layout` (tensor-layout-v1.yaml).
macro_rules! contract_pre_layout {
    () => {{}};
    ($input:expr) => {{
        let input = &$input;
        debug_assert!(
            input.len() > 0,
            "Contract layout: precondition violated — input.len() > 0"
        );
    }};
}

/// Preconditions for equation `draw` (preprocessing-normalization-v1.yaml).
macro_rules! contract_pre_draw {
    () => {{}};
    ($input:expr) => {{
        let input = &$input;
        debug_assert!(
            input.len() > 0,
            "Contract draw: precondition violated — input.len() > 0"
        );
    }};
}

/// Preconditions for equation `configuration` (configuration-v1.yaml).
macro_rules! contract_pre_configuration {
    () => {{}};
    ($input:expr) => {{
        let input = &$input;
        debug_assert!(
            input.len() > 0,
            "Contract configuration: precondition violated — input.len() > 0"
        );
    }};
}
