//! Drafting app binary. Until the viewer lands (M4) it only loads a DXF and
//! prints a summary, which is handy for checking corpus files by hand.

use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: cad-app <file.dxf>");
        return ExitCode::FAILURE;
    };
    match cad_dxf::read_file(&path) {
        Ok((drawing, warnings)) => {
            println!(
                "{path}: version {}, {} layers, {} entities",
                drawing.header.acadver().unwrap_or("unknown"),
                drawing.tables.layers.len(),
                drawing.entities.len()
            );
            for w in &warnings {
                println!("warning: {w}");
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{path}: {err}");
            ExitCode::FAILURE
        }
    }
}
