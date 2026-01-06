//! Example: Headless ttop usage
//!
//! Demonstrates how to use ttop's App in deterministic mode
//! for testing and automation.
//!
//! Run: cargo run --example headless

use ttop::app::App;

fn main() {
    println!("ttop Headless Example");
    println!("=====================\n");

    // Create app in deterministic mode (no real-time updates)
    let mut app = App::new(true, false);

    // Collect metrics once
    app.collect_metrics();

    // Query app state
    println!("Panel Visibility:");
    println!("  CPU: {}", app.panels.cpu);
    println!("  Memory: {}", app.panels.memory);
    println!("  GPU: {}", app.panels.gpu);
    println!("  Process: {}", app.panels.process);
    println!();

    println!("Help Visible: {}", app.show_help);
    println!("Tree View: {}", app.show_tree);
    println!();

    // Simulate key presses
    use crossterm::event::KeyCode;
    use crossterm::event::KeyModifiers;

    // Toggle help panel
    app.handle_key(KeyCode::Char('?'), KeyModifiers::NONE);
    println!("After '?': Help Visible: {}", app.show_help);

    // Toggle tree view
    app.handle_key(KeyCode::Char('t'), KeyModifiers::NONE);
    println!("After 't': Tree View: {}", app.show_tree);

    // Toggle CPU panel (key '1')
    let cpu_before = app.panels.cpu;
    app.handle_key(KeyCode::Char('1'), KeyModifiers::NONE);
    println!("After '1': CPU Panel: {} -> {}", cpu_before, app.panels.cpu);

    println!("\nDone! Use 'ttop' for the full TUI experience.");
}
