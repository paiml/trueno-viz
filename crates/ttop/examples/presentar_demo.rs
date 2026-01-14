//! Demo: Presentar CPU panel rendering
//!
//! Run with: cargo run --example presentar_demo --features presentar

#[cfg(feature = "presentar")]
fn main() {
    use presentar_core::Rect;
    use presentar_terminal::CellBuffer;
    use ttop::app::App;
    use ttop::panels_presentar::draw_cpu_presentar;

    println!("Presentar CPU Panel Demo\n");
    println!("Creating mock app...");

    let app = App::new_mock();

    // Create a buffer and render
    let mut buffer = CellBuffer::new(80, 20);
    let area = Rect::new(0.0, 0.0, 80.0, 18.0);

    draw_cpu_presentar(&mut buffer, &app, area);

    // Print the buffer contents
    println!("Rendered output:\n");
    for y in 0..buffer.height() {
        for x in 0..buffer.width() {
            if let Some(cell) = buffer.get(x, y) {
                print!("{}", cell.symbol);
            } else {
                print!(" ");
            }
        }
        println!();
    }

    println!("\n✓ Presentar CPU panel rendered successfully!");
}

#[cfg(not(feature = "presentar"))]
fn main() {
    eprintln!("Error: Run with --features presentar");
    eprintln!("  cargo run --example presentar_demo --features presentar");
    std::process::exit(1);
}
