//! Drafting app binary. Until the viewer lands (M4) it is a command-line
//! tool for checking files by hand:
//!
//!     cad-app FILE.dxf                         summary and warnings
//!     cad-app convert IN.dxf OUT.dxf r12|r2018 write as R12 or R2018
//!     cad-app generate IN.dxf OUT.dxf r12|r2018 always regenerate (never reuse bytes)
//!     cad-app save IN.dxf OUT.dxf              save back (unchanged = identical)

use std::process::ExitCode;

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

fn run(args: &[String]) -> Result<(), String> {
    match args {
        [cmd, input, output, version] if cmd == "convert" || cmd == "generate" => {
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
        }
        [cmd, input, output] if cmd == "save" => {
            let drawing = load(input)?;
            let warnings = drawing
                .save_file(output)
                .map_err(|e| format!("{output}: {e}"))?;
            print_warnings(output, &warnings);
            Ok(())
        }
        [path] => {
            let drawing = load(path)?;
            println!(
                "{path}: version {}, {} layers, {} entities",
                drawing.header.acadver().unwrap_or("unknown"),
                drawing.tables.layers.len(),
                drawing.entities.len()
            );
            Ok(())
        }
        _ => {
            Err("usage: cad-app FILE.dxf | convert|generate IN OUT r12|r2018 | save IN OUT".into())
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
