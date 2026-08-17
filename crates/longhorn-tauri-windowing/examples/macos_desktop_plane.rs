//! Prints the live macOS desktop plane this machine reports.
//!
//! Native evidence for Card 226: run it on a genuinely mixed-scale desktop and
//! compare the output against Core Graphics directly. `main` is the process's
//! main thread, which is what `AppKitDesktopPlane` requires.
//!
//! ```sh
//! cargo run -p longhorn-tauri-windowing --example macos_desktop_plane
//! ```

fn main() {
    #[cfg(not(target_os = "macos"))]
    println!("the macOS desktop plane exists only on macOS");

    #[cfg(target_os = "macos")]
    {
        use longhorn_tauri_windowing::{AppKitDesktopPlane, NativeDesktopPlane};

        match AppKitDesktopPlane.displays() {
            Ok(displays) => {
                println!("displays: {}", displays.len());
                for display in &displays {
                    let full = display.full_bounds();
                    let work = display.work_area();
                    println!(
                        "  full=({}, {} {}x{})  work=({}, {} {}x{})  physical={}x{}  scale={}  main={}",
                        full.origin().x().get(),
                        full.origin().y().get(),
                        full.size().width(),
                        full.size().height(),
                        work.origin().x().get(),
                        work.origin().y().get(),
                        work.size().width(),
                        work.size().height(),
                        display.physical_size().width(),
                        display.physical_size().height(),
                        display.scale().thousandths(),
                        display.is_main(),
                    );
                }
            }
            Err(detail) => println!("refused: {detail}"),
        }
    }
}
