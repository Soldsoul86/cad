//! The drafting app (working name). With no command, opens the viewer:
//!
//!     cad-app [FILE.dxf]                       open the viewer
//!     cad-app --bench FILE.dxf [FRAMES]        animated pan/zoom, print frame times
//!     cad-app --shot FILE.dxf OUT.ppm          save a screenshot of the window and quit
//!     cad-app info FILE.dxf                    summary and warnings
//!     cad-app convert IN.dxf OUT.dxf r12|r2018 write as R12 or R2018
//!     cad-app generate IN.dxf OUT.dxf r12|r2018 always regenerate (never reuse bytes)
//!     cad-app save IN.dxf OUT.dxf              save back (unchanged = identical)
//!     cad-app diff A.dxf B.dxf [--json OUT]    list changes from A to B
//!     cad-app --diff A.dxf B.dxf [--side]      compare A and B in the viewer
//!     cad-app --diff A.dxf B.dxf --shot OUT.ppm   ... save a screenshot and quit
//!
//! On the web it runs in the canvas with id `cad_canvas`; drop a file on it.

mod app;
mod compare;
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
            [cmd, a, b, rest @ ..] if cmd == "diff" && matches!(rest, [] | [_, _]) => {
                (|| {
                    let (da, db) = (load(a)?, load(b)?);
                    let diff = cad_diff::diff(&da, &db, &cad_diff::DiffOptions::default());
                    for c in &diff.changes {
                        println!("{c}");
                    }
                    println!(
                        "{} added, {} deleted, {} changed, {} unchanged, {} equivalent",
                        diff.count(cad_diff::ChangeKind::Added),
                        diff.count(cad_diff::ChangeKind::Deleted),
                        diff.count(cad_diff::ChangeKind::Changed),
                        diff.unchanged,
                        diff.equivalent
                    );
                    match rest {
                        [flag, out] if flag == "--json" => std::fs::write(out, diff.to_json(a, b))
                            .map_err(|e| format!("{out}: {e}")),
                        [flag, _] => Err(format!("unknown option {flag}")),
                        _ => Ok(()),
                    }
                })()
            }
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
    let mut side_by_side = false;
    let (bench, paths) = match args.as_slice() {
        [flag, path, rest @ ..] if flag == "--bench" => {
            let frames = rest.first().and_then(|n| n.parse().ok()).unwrap_or(600);
            (Some(app::Bench::new(frames)), vec![path.clone()])
        }
        [flag, path, out] if flag == "--shot" => (
            Some(app::Bench::screenshot(out.clone())),
            vec![path.clone()],
        ),
        [flag, a, b, rest @ ..] if flag == "--diff" => {
            let mut bench = None;
            let mut rest = rest.iter();
            while let Some(opt) = rest.next() {
                match (
                    opt.as_str(),
                    (opt == "--shot").then(|| rest.next()).flatten(),
                ) {
                    ("--side", _) => side_by_side = true,
                    ("--shot", Some(out)) => bench = Some(app::Bench::screenshot(out.clone())),
                    _ => {
                        eprintln!("usage: cad-app --diff A.dxf B.dxf [--side] [--shot OUT.ppm]");
                        return std::process::ExitCode::FAILURE;
                    }
                }
            }
            (bench, vec![a.clone(), b.clone()])
        }
        [path] => (None, vec![path.clone()]),
        [] => (None, Vec::new()),
        _ => {
            eprintln!(
                "usage: cad-app [FILE.dxf] | --diff A.dxf B.dxf | --bench FILE.dxf [FRAMES] | info|convert|generate|save|diff ..."
            );
            return std::process::ExitCode::FAILURE;
        }
    };
    let mut files = Vec::new();
    for path in paths {
        match std::fs::read(&path) {
            Ok(bytes) => files.push((path, bytes)),
            Err(e) => {
                eprintln!("{path}: {e}");
                return std::process::ExitCode::FAILURE;
            }
        }
    }
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
        Box::new(move |cc| Ok(Box::new(app::CadApp::new(cc, files, bench, side_by_side)))),
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
                Box::new(|cc| Ok(Box::new(app::CadApp::new(cc, Vec::new(), None, false)))),
            )
            .await;
    });
}
