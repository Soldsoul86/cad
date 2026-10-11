//! Golden SVG snapshots: every corpus file is rendered to SVG and compared
//! with `tests/snapshots/<file>.svg`. A rendering change shows up as a diff
//! of those files (GitHub shows SVG diffs side by side).
//!
//! On a mismatch the new SVGs are written to `target/snapshots-new/` and the
//! test fails. To accept the changes, run with `UPDATE_SNAPSHOTS=1`.

use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn corpus_renders_match_the_snapshots() {
    let corpus = root().join("tests/corpus");
    let snapshots = root().join("tests/snapshots");
    let new_dir = root().join("target/snapshots-new");
    let update = std::env::var_os("UPDATE_SNAPSHOTS").is_some();
    let mut files: Vec<PathBuf> = std::fs::read_dir(&corpus)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "dxf"))
        .collect();
    files.sort();
    assert!(files.len() >= 198);
    let mut changed = Vec::new();
    for path in &files {
        let (drawing, _) = lightable_dxf::read_file(path).unwrap();
        let scene = lightable_render::build(&drawing);
        // SVG text is always LF, so snapshots compare the same on Windows.
        let svg = lightable_render::svg::to_svg(&scene, None);
        let name = format!("{}.svg", path.file_stem().unwrap().to_string_lossy());
        let expected = std::fs::read_to_string(snapshots.join(&name))
            .ok()
            .map(|s| s.replace("\r\n", "\n"));
        if expected.as_deref() == Some(svg.as_str()) {
            continue;
        }
        if update {
            std::fs::create_dir_all(&snapshots).unwrap();
            std::fs::write(snapshots.join(&name), &svg).unwrap();
        } else {
            std::fs::create_dir_all(&new_dir).unwrap();
            std::fs::write(new_dir.join(&name), &svg).unwrap();
            changed.push(name);
        }
    }
    assert!(
        changed.is_empty(),
        "{} snapshot(s) differ; new versions are in target/snapshots-new/ \
         (run with UPDATE_SNAPSHOTS=1 to accept):\n{}",
        changed.len(),
        changed.join("\n")
    );
}
