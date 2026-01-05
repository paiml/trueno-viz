//! ttop: Terminal Top - 10X Better Than btop
//!
//! A pure Rust system monitor with:
//! - Deterministic rendering for testing
//! - GPU support (NVIDIA + AMD)
//! - Sovereign AI Stack integration
//! - 8ms frame time target (2X faster than btop)
//! - CIELAB perceptual color gradients
//!
//! Install: `cargo install ttop`
//! Run: `ttop`

use ttop::{app, ui};

use anyhow::Result;
use clap::Parser;
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::stdout;
use std::time::{Duration, Instant};

use app::App;

/// ttop: Terminal Top - 10X Better Than btop
#[derive(Parser, Debug)]
#[command(name = "ttop")]
#[command(author = "PAIML Team")]
#[command(version)]
#[command(about = "Pure Rust system monitor - 10X better than btop", long_about = None)]
struct Cli {
    /// Refresh rate in milliseconds
    #[arg(short, long, default_value = "1000")]
    refresh: u64,

    /// Enable deterministic mode for testing
    #[arg(long)]
    deterministic: bool,

    /// Config file path
    #[arg(short, long)]
    config: Option<String>,

    /// Enable syscall tracing (requires --features tracing)
    #[arg(long)]
    trace: bool,

    /// Export trace to file
    #[arg(long)]
    trace_output: Option<String>,

    /// Show frame timing statistics
    #[arg(long)]
    show_fps: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let result = run_app(&mut terminal, &cli);

    // Restore terminal
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, cli: &Cli) -> Result<()> {
    let mut app = App::new(cli.deterministic, cli.show_fps);
    let tick_rate = Duration::from_millis(50);
    let collect_interval = Duration::from_millis(cli.refresh);

    // Frame timing
    let mut frame_times: Vec<Duration> = Vec::with_capacity(60);
    let mut last_frame = Instant::now();

    loop {
        let frame_start = Instant::now();

        terminal.draw(|f| ui::draw(f, &mut app))?;

        // Track frame time
        let frame_time = frame_start.elapsed();
        frame_times.push(frame_time);
        if frame_times.len() > 60 {
            frame_times.remove(0);
        }
        app.update_frame_stats(&frame_times);

        // Collect metrics periodically
        if last_frame.elapsed() >= collect_interval {
            app.collect_metrics();
            last_frame = Instant::now();
        }

        // Handle events
        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && app.handle_key(key.code, key.modifiers) {
                    return Ok(());
                }
            }
        }
    }
}
