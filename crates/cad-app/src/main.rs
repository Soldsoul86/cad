//! The drafting app (working name). With no command, opens the viewer:
//!
//!     cad-app [FILE.dxf]                       open the viewer
//!     cad-app --bench FILE.dxf [FRAMES]        animated pan/zoom, print frame times
//!     cad-app --shot FILE.dxf OUT.ppm          save a screenshot of the window and quit
//!     cad-app info FILE.dxf                    summary and warnings
//!     cad-app convert IN.dxf OUT.dxf r12|r2018 write as R12 or R2018
//!     cad-app generate IN.dxf OUT.dxf r12|r2018 always regenerate (never reuse bytes)
//!     cad-app save IN.dxf OUT.dxf              save back (unchanged = identical)
//!
//! On the web it runs in the canvas with id `cad_canvas`; drop a file on it.

mod app;
mod gpu;

#[cfg(not(target_arch = "wasm32"))]
mod cli {
    use cad_dxf::{Drawing, Version, Warning};

    fn print_warnings(path: &str, warnings: &[Warning]) {
        for w in warnings {
            println!("warning: {path}: {w}");
        }
    }

    fn load(path: &str) -> Result<Drawing, String> {
        let (drawing, warnings) = cad_dxf::read_file(path).map_err(|e| format!("{path}: {e}"))?;
        print_warnings(path, &warnings);
        Ok(drawing)
    }

    /// Runs a command-line command; `None` if `args` are not one.
    pub fn run(args: &[String]) -> Option<Result<(), String>> {
        Some(match args {
            [cmd, input, output, version] if cmd == "convert" || cmd == "generate" => (|| {
                let version = match version.to_ascii_lowercase().as_str() {
                    "r12" => Version::R12,
                    "r2018" => Version::R2018,
                    other => return Err(format!("unknown version {other:?}; use r12 or r2018")),
                };
                let drawing = load(input)?;
                let warnings = if cmd == "generate" {
                    let (bytes, warnings) =
                        drawing.generate_bytes(version).map_err(|e| e.to_string())?;
                    std::fs::write(output, bytes).map_err(|e| format!("{output}: {e}"))?;
                    warnings
                } else {
                    drawing
                        .write_file(output, version)
                        .map_err(|e| format!("{output}: {e}"))?
                };
                print_warnings(output, &warnings);
                Ok(())
            })(
            ),
            [cmd, input, output] if cmd == "save" => (|| {
                let drawing = load(input)?;
                let warnings = drawing
                    .save_file(output)
                    .map_err(|e| format!("{output}: {e}"))?;
                print_warnings(output, &warnings);
                Ok(())
            })(),
            [cmd, path] if cmd == "info" => (|| {
                let drawing = load(path)?;
                println!(
                    "{path}: version {}, {} layers, {} entities",
                    drawing.header.acadver().unwrap_or("unknown"),
                    drawing.tables.layers.len(),
                    drawing.entities.len()
                );
                Ok(())
            })(),
            _ => return None,
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(result) = cli::run(&args) {
        return match result {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("{message}");
                std::process::ExitCode::FAILURE
            }
        };
    }
    let (bench, path) = match args.as_slice() {
        [flag, path, rest @ ..] if flag == "--bench" => {
            let frames = rest.first().and_then(|n| n.parse().ok()).unwrap_or(600);
            (Some(app::Bench::new(frames)), Some(path.clone()))
        }
        [flag, path, out] if flag == "--shot" => (
            Some(app::Bench::screenshot(out.clone())),
            Some(path.clone()),
        ),
        [path] => (None, Some(path.clone())),
        [] => (None, None),
        _ => {
            eprintln!(
                "usage: cad-app [FILE.dxf] | --bench FILE.dxf [FRAMES] | info|convert|generate|save ..."
            );
            return std::process::ExitCode::FAILURE;
        }
    };
    let file = match path {
        Some(path) => match std::fs::read(&path) {
            Ok(bytes) => Some((path, bytes)),
            Err(e) => {
                eprintln!("{path}: {e}");
                return std::process::ExitCode::FAILURE;
            }
        },
        None => None,
    };
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("cad (working name)"),
        // The benchmark measures frames as fast as they can be drawn.
        glow_options: eframe::egui_glow::GlowConfiguration {
            vsync: bench.is_none(),
            ..Default::default()
        },
        ..Default::default()
    };
    let result = eframe::run_native(
        "cad",
        options,
        Box::new(|cc| Ok(Box::new(app::CadApp::new(cc, file, bench)))),
    );
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;
    wasm_bindgen_futures::spawn_local(async {
        let Some(canvas) = eframe::web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("cad_canvas"))
            .and_then(|e| e.dyn_into::<eframe::web_sys::HtmlCanvasElement>().ok())
        else {
            return;
        };
        let _ = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| Ok(Box::new(app::CadApp::new(cc, None, None)))),
            )
            .await;
    });
}
