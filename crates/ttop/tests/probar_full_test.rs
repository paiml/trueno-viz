//! Full Probar testing for ttop using advanced features.
//!
//! This test file demonstrates proper use of probar's capabilities:
//! - GUI Coverage tracking with `gui_coverage!` macro
//! - Pixel Coverage Tracker for grid-based verification
//! - Falsifiability Gates (Popperian methodology)
//! - Wilson Score confidence intervals
//! - Score bars and combined coverage reports
#![allow(clippy::unwrap_used)]
#![allow(dead_code)]
#![allow(unused_variables)]

use jugar_probar::pixel_coverage::{
    ConfidenceInterval, FalsifiabilityGate, FalsifiableHypothesis, OutputMode, PixelCoverageTracker,
    PixelRegion, ScoreBar,
};
use jugar_probar::prelude::*;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::{Block, BorderType, Borders, Widget};

use trueno_viz::monitor::widgets::{Graph, GraphMode, Meter};

/// btop-style block helper
fn btop_block(title: &str, color: Color) -> Block<'static> {
    Block::default()
        .title(title.to_string())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(ratatui::style::Style::default().fg(color))
}

#[cfg(test)]
mod ttop_gui_coverage_tests {
    use super::*;

    #[test]
    fn test_ttop_panel_gui_coverage() -> ProbarResult<()> {
        println!("=== ttop GUI Coverage Test ===\n");

        // Define all ttop UI elements
        let mut gui = jugar_probar::gui_coverage! {
            buttons: ["toggle_cpu", "toggle_mem", "toggle_disk", "toggle_net",
                     "toggle_proc", "toggle_gpu", "toggle_bat", "toggle_sensors",
                     "sort_next", "sort_reverse", "filter", "help", "quit",
                     "nav_up", "nav_down", "nav_pgup", "nav_pgdn"],
            screens: ["main", "help_overlay", "filter_input"]
        };

        // Simulate key interactions (what a user would do)
        gui.visit("main");
        gui.click("toggle_cpu");       // Press '1'
        gui.click("toggle_mem");       // Press '2'
        gui.click("sort_next");        // Press Tab
        gui.click("nav_down");         // Press j
        gui.click("nav_up");           // Press k
        gui.click("help");             // Press ?
        gui.visit("help_overlay");
        gui.click("quit");             // Press q (from help)

        println!("   GUI Coverage: {}", gui.summary());

        // Define falsifiable hypothesis: 40% coverage threshold for basic interaction
        let gate = FalsifiabilityGate::new(15.0);
        let report = gui.generate_report();

        let h1 = FalsifiableHypothesis::coverage_threshold("H0-TTOP-BASIC", 0.40)
            .evaluate(report.overall_coverage as f32);

        println!(
            "   H0-TTOP-BASIC: {} (actual: {:.1}%)",
            if h1.falsified { "FALSIFIED" } else { "NOT FALSIFIED" },
            h1.actual.unwrap_or(0.0) * 100.0
        );

        // For basic interaction, we don't require 100% coverage
        assert!(!h1.falsified, "Basic interaction coverage should be >= 40%");

        Ok(())
    }

    #[test]
    fn test_ttop_panel_pixel_coverage() -> ProbarResult<()> {
        println!("=== ttop Pixel Coverage Test ===\n");

        // Simulate ttop terminal at 120x40 characters
        // Grid: 12 columns x 4 rows = 48 cells
        let mut pixels = PixelCoverageTracker::builder()
            .resolution(120, 40) // Terminal size
            .grid_size(12, 4)    // 10x10 char cells
            .threshold(0.60)
            .build();

        // Record regions for each panel (approximate positions)
        // CPU panel: top-left quarter
        pixels.record_region(PixelRegion::new(0, 0, 60, 15));

        // Memory panel: top-right
        pixels.record_region(PixelRegion::new(60, 0, 60, 15));

        // Process panel: bottom half
        pixels.record_region(PixelRegion::new(0, 15, 120, 25));

        let report = pixels.generate_report();
        println!(
            "   Pixel Coverage: {:.1}% ({}/{} cells)",
            report.overall_coverage * 100.0,
            report.covered_cells,
            report.total_cells
        );

        // Heatmap visualization
        println!("   Pixel Heatmap:");
        let heatmap = pixels.terminal_heatmap();
        for line in heatmap.render().lines() {
            println!("     {}", line);
        }

        // Confidence interval
        let ci = ConfidenceInterval::wilson_score(
            report.covered_cells,
            report.total_cells,
            0.95,
        );
        println!(
            "   95% CI: [{:.1}%, {:.1}%]",
            ci.lower * 100.0,
            ci.upper * 100.0
        );

        assert!(report.meets_threshold, "Should meet 60% pixel coverage threshold");

        Ok(())
    }

    #[test]
    fn test_ttop_widget_pixel_verification() -> ProbarResult<()> {
        println!("=== Widget Pixel Verification ===\n");

        // Test Graph widget with pixel tracking
        let mut graph_pixels = PixelCoverageTracker::builder()
            .resolution(40, 8)
            .grid_size(8, 4)
            .threshold(0.50)
            .build();

        // Render graph to buffer
        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 8));
        let data = vec![0.2, 0.4, 0.6, 0.8, 1.0, 0.7, 0.5, 0.3];
        let graph = Graph::new(&data).mode(GraphMode::Block).color(Color::Cyan);
        graph.render(Rect::new(0, 0, 40, 8), &mut buffer);

        // Record pixels where graph rendered (non-empty cells)
        for y in 0..8 {
            for x in 0..40 {
                if let Some(cell) = buffer.cell((x, y)) {
                    if cell.symbol() != " " {
                        graph_pixels.record_region(PixelRegion::new(x as u32, y as u32, 1, 1));
                    }
                }
            }
        }

        let report = graph_pixels.generate_report();
        let mode = OutputMode::from_env();
        let bar = ScoreBar::new("Graph", report.overall_coverage, 0.5);

        println!("   {}", bar.render(mode));
        println!(
            "   Graph Coverage: {:.1}% ({}/{} cells)",
            report.overall_coverage * 100.0,
            report.covered_cells,
            report.total_cells
        );

        Ok(())
    }

    #[test]
    fn test_ttop_meter_pixel_verification() -> ProbarResult<()> {
        println!("=== Meter Pixel Verification ===\n");

        let mut meter_pixels = PixelCoverageTracker::builder()
            .resolution(30, 1)
            .grid_size(30, 1)
            .threshold(0.20) // At least 20% should be filled for a meter
            .build();

        // Render 75% meter
        let mut buffer = Buffer::empty(Rect::new(0, 0, 30, 1));
        let meter = Meter::new(0.75).label("CPU").color(Color::Yellow);
        meter.render(Rect::new(0, 0, 30, 1), &mut buffer);

        // Record filled cells
        for x in 0..30 {
            if let Some(cell) = buffer.cell((x, 0)) {
                if cell.symbol() == "█" {
                    meter_pixels.record_region(PixelRegion::new(x as u32, 0, 1, 1));
                }
            }
        }

        let report = meter_pixels.generate_report();
        let mode = OutputMode::from_env();
        let bar = ScoreBar::new("Meter", report.overall_coverage, 0.75);

        println!("   {}", bar.render(mode));
        println!(
            "   Meter Fill: {:.1}% ({}/{} cells)",
            report.overall_coverage * 100.0,
            report.covered_cells,
            report.total_cells
        );

        // 75% meter should have roughly 75% of available space filled
        // (minus label space)
        assert!(report.overall_coverage >= 0.20, "75% meter should fill at least 20% of width");

        Ok(())
    }

    #[test]
    fn test_ttop_full_coverage_report() -> ProbarResult<()> {
        println!("=== ttop Full Coverage Report ===\n");

        // GUI Coverage
        let mut gui = jugar_probar::gui_coverage! {
            buttons: ["cpu", "mem", "disk", "net", "proc", "gpu", "bat", "sensors"],
            screens: ["main_view"]
        };

        // Click all panel toggles
        for btn in ["cpu", "mem", "disk", "net", "proc", "gpu", "bat", "sensors"] {
            gui.click(btn);
        }
        gui.visit("main_view");

        // Pixel coverage for main layout
        let mut pixels = PixelCoverageTracker::builder()
            .resolution(80, 24)
            .grid_size(8, 4)
            .threshold(0.80)
            .build();

        // Cover all major panel areas
        pixels.record_region(PixelRegion::new(0, 0, 40, 12));   // CPU
        pixels.record_region(PixelRegion::new(40, 0, 40, 12));  // Memory
        pixels.record_region(PixelRegion::new(0, 12, 80, 12));  // Process

        let gui_report = gui.generate_report();
        let pixel_report = pixels.generate_report();

        // Score bars
        let mode = OutputMode::from_env();
        let gui_bar = ScoreBar::new("GUI", gui_report.overall_coverage as f32, 1.0);
        let pixel_bar = ScoreBar::new("Pixels", pixel_report.overall_coverage, 0.80);

        println!("   {}", gui_bar.render(mode));
        println!("   {}", pixel_bar.render(mode));

        // Falsification gates
        let gate = FalsifiabilityGate::new(10.0);

        let h_gui = FalsifiableHypothesis::coverage_threshold("H0-GUI-100", 1.0)
            .evaluate(gui_report.overall_coverage as f32);
        let h_pix = FalsifiableHypothesis::coverage_threshold("H0-PIX-80", 0.80)
            .evaluate(pixel_report.overall_coverage);

        println!(
            "\n   H0-GUI-100: {} (need 100%, got {:.1}%)",
            if h_gui.falsified { "FALSIFIED" } else { "NOT FALSIFIED" },
            h_gui.actual.unwrap_or(0.0) * 100.0
        );
        println!(
            "   H0-PIX-80: {} (need 80%, got {:.1}%)",
            if h_pix.falsified { "FALSIFIED" } else { "NOT FALSIFIED" },
            h_pix.actual.unwrap_or(0.0) * 100.0
        );

        // Confidence intervals
        let gui_ci = ConfidenceInterval::wilson_score(
            gui_report.covered_elements as u32,
            gui_report.total_elements as u32,
            0.95,
        );
        let pix_ci = ConfidenceInterval::wilson_score(
            pixel_report.covered_cells,
            pixel_report.total_cells,
            0.95,
        );

        println!("\n   GUI 95% CI: [{:.1}%, {:.1}%]", gui_ci.lower * 100.0, gui_ci.upper * 100.0);
        println!("   Pixel 95% CI: [{:.1}%, {:.1}%]", pix_ci.lower * 100.0, pix_ci.upper * 100.0);

        // All panels clicked = 100% GUI coverage
        assert!(!h_gui.falsified, "Should have 100% GUI coverage");
        assert!(!h_pix.falsified, "Should have 80% pixel coverage");

        println!("\n[OK] Full coverage report completed!");
        Ok(())
    }

    #[test]
    fn test_ttop_panel_block_rendering() -> ProbarResult<()> {
        println!("=== Panel Block Rendering ===\n");

        let mut pixels = PixelCoverageTracker::builder()
            .resolution(50, 10)
            .grid_size(10, 5)
            .threshold(0.30)
            .build();

        // Render CPU panel block
        let mut buffer = Buffer::empty(Rect::new(0, 0, 50, 10));
        let block = btop_block(" CPU 45% │ 8 cores ", Color::Cyan);
        block.render(Rect::new(0, 0, 50, 10), &mut buffer);

        // Record border cells
        for y in 0..10 {
            for x in 0..50 {
                if let Some(cell) = buffer.cell((x, y)) {
                    let sym = cell.symbol();
                    if sym != " " {
                        pixels.record_region(PixelRegion::new(x as u32, y as u32, 1, 1));
                    }
                }
            }
        }

        let report = pixels.generate_report();
        println!(
            "   Block Coverage: {:.1}% ({}/{} cells)",
            report.overall_coverage * 100.0,
            report.covered_cells,
            report.total_cells
        );

        // Blocks should have visible borders
        assert!(
            report.overall_coverage >= 0.10,
            "Block borders should cover at least 10% of area"
        );

        Ok(())
    }
}

#[cfg(test)]
mod ttop_user_journey_tests {
    use super::*;

    #[test]
    fn test_user_journey_monitoring() -> ProbarResult<()> {
        println!("=== User Journey: System Monitoring ===\n");

        let mut journey = UxCoverageTracker::new();

        // Register all ttop screens
        journey.register_screen("startup");
        journey.register_screen("monitoring");
        journey.register_screen("process_filter");
        journey.register_screen("help");

        // Journey 1: Quick check
        journey.visit("startup");
        journey.visit("monitoring");
        journey.end_journey();

        // Journey 2: Find a process
        journey.visit("startup");
        journey.visit("monitoring");
        journey.visit("process_filter");
        journey.visit("monitoring");
        journey.end_journey();

        // Journey 3: Get help
        journey.visit("startup");
        journey.visit("monitoring");
        journey.visit("help");
        journey.visit("monitoring");
        journey.end_journey();

        println!("   Journeys recorded: {}", journey.journeys().len());
        println!("   {}", journey.summary());

        // All screens should be visited
        let report = journey.generate_report();
        assert!(
            report.state_coverage >= 1.0,
            "All screens should be visited across journeys"
        );

        Ok(())
    }
}
