//! M1 acceptance: every corpus file loads without panicking, plus spot
//! checks of what was read and a cheap robustness sweep over damaged input.

use std::path::PathBuf;

use cad_dxf::{Color, Drawing, EntityKind, LineWeight, Point3, Version, Warning, read_bytes};

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus")
}

fn corpus_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(corpus_dir())
        .expect("tests/corpus exists")
        .map(|e| e.expect("readable dir entry").path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("dxf")))
        .collect();
    files.sort();
    files
}

fn load(name: &str) -> (Drawing, Vec<Warning>) {
    cad_dxf::read_file(corpus_dir().join(name))
        .unwrap_or_else(|e| panic!("{name} failed to load: {e}"))
}

fn kinds(d: &Drawing) -> Vec<&str> {
    d.entities.iter().map(|e| e.kind.type_name()).collect()
}

fn parsed(d: &Drawing) -> usize {
    d.entities
        .iter()
        .filter(|e| !matches!(e.kind, EntityKind::Unknown(_)))
        .count()
}

#[test]
fn at_least_ten_corpus_files_load() {
    let files = corpus_files();
    assert!(files.len() >= 10, "only {} corpus files", files.len());
    for path in files {
        if let Err(err) = cad_dxf::read_file(&path) {
            panic!("{} failed to load: {err}", path.display());
        }
    }
}

#[test]
fn r12_single_line() {
    let (d, w) = load("r12_single_line.dxf");
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(d.header.version(), Some(Version::R12));
    assert_eq!(d.header.extmax(), Some(Point3::new(100.0, 50.0, 0.0)));
    let EntityKind::Line(line) = &d.entities[0].kind else {
        panic!()
    };
    assert_eq!(line.end, Point3::new(100.0, 50.0, 0.0));
}

#[test]
fn r12_circle_arc_layers() {
    let (d, w) = load("r12_circle_arc_layers.dxf");
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(kinds(&d), ["CIRCLE", "ARC"]);
    assert_eq!(d.header.current_layer(), Some("GEOMETRY"));
    assert_eq!(
        d.tables.layers.get("geometry").unwrap().color,
        Color::Index(3)
    );
    assert_eq!(d.tables.other[0].name, "LTYPE");
}

#[test]
fn ac1015_floor_plan() {
    let (d, w) = load("ac1015_floor_plan.dxf");
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(d.header.version(), Some(Version::R2000));
    assert_eq!(d.header.codepage(), Some("ANSI_1252"));
    assert_eq!(d.header.insunits(), Some(4));
    assert_eq!(d.header.text_style(), Some("Standard"));
    assert_eq!(d.header.dim_style(), Some("Standard"));
    let layers = &d.tables.layers;
    assert_eq!(layers.len(), 4);
    assert!(!layers.get("DOORS").unwrap().on);
    let walls = layers.get("WALLS").unwrap();
    assert_eq!(walls.lineweight, LineWeight::Hundredths(50));
    assert_eq!(
        walls.extra.len(),
        1,
        "plot style handle kept: {:?}",
        walls.extra
    );
    let hidden = layers.get("HIDDEN_REF").unwrap();
    assert!(hidden.frozen() && hidden.locked() && !hidden.plot);
    let on_walls = d.entities.iter().filter(|e| e.layer == "WALLS").count();
    assert_eq!(on_walls, 4);
    assert_eq!(parsed(&d), 6);
}

#[test]
fn ac1032_colors_and_lineweights() {
    let (d, w) = load("ac1032_colors_lineweights.dxf");
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(d.header.version(), Some(Version::R2018));
    assert_eq!(d.header.extmin(), Some(Point3::new(-5.0, -5.0, 0.0)));
    assert_eq!(
        d.tables.layers.get("site").unwrap().color,
        Color::Rgb(255, 128, 0)
    );
    let e = &d.entities;
    assert_eq!(e[0].color, Color::Rgb(255, 0, 0));
    assert_eq!(e[0].lineweight, LineWeight::Hundredths(35));
    assert_eq!(e[1].color, Color::ByBlock);
    assert_eq!(e[1].linetype, "BYBLOCK");
    assert_eq!(e[1].lineweight, LineWeight::ByBlock);
    assert_eq!(e[2].color, Color::ByLayer);
    // A zero-radius circle is degenerate but valid.
    assert!(matches!(&e[3].kind, EntityKind::Circle(c) if c.radius == 0.0));
}

#[test]
fn crlf_and_padded_files_match_plain_ones() {
    let (d, w) = load("crlf_line_endings.dxf");
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(kinds(&d), ["LINE", "CIRCLE", "ARC"]);
    assert_eq!(parsed(&d), 3);

    let (d, w) = load("padded_group_codes.dxf");
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(d.header.insunits(), Some(1));
    let EntityKind::Circle(c) = &d.entities[1].kind else {
        panic!()
    };
    assert_eq!(c.center, Point3::new(10.0, -0.1, 0.0));
    assert_eq!(c.radius, 0.5);
}

#[test]
fn unknown_entities_are_kept_verbatim() {
    let (d, w) = load("unknown_entities.dxf");
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(
        kinds(&d),
        [
            "LWPOLYLINE",
            "TEXT",
            "POINT",
            "LINE",
            "POLYLINE",
            "VERTEX",
            "VERTEX",
            "SEQEND",
            "ACME_CUSTOM_THING"
        ]
    );
    assert_eq!(parsed(&d), 1);
    let EntityKind::Unknown(poly) = &d.entities[0].kind else {
        panic!()
    };
    assert_eq!(poly.pairs.len(), 16);
    assert_eq!(d.entities[1].layer, "ANNOT");
}

#[test]
fn all_sections_are_captured() {
    let (d, w) = load("all_sections.dxf");
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(d.header.version(), Some(Version::R2004));
    assert!(!d.classes.is_empty());
    assert!(!d.blocks.is_empty());
    assert!(!d.objects.is_empty());
    assert_eq!(d.tables.other[0].name, "VPORT");
    assert_eq!(d.other_sections[0].name, "ACDSDATA");
    assert_eq!(parsed(&d), 1);
}

#[test]
fn entities_only_file_without_eof() {
    let (d, w) = load("entities_only_no_eof.dxf");
    assert_eq!(parsed(&d), 2);
    assert_eq!(d.header.version(), None);
    assert_eq!(w.len(), 1);
    assert!(w[0].message.contains("EOF"));
}

#[test]
fn arcs_and_extras() {
    let (d, w) = load("arcs_and_extras.dxf");
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(parsed(&d), 5);
    let EntityKind::Arc(wrap) = &d.entities[0].kind else {
        panic!()
    };
    assert_eq!((wrap.start_angle, wrap.end_angle), (350.0, 10.0));
    let EntityKind::Arc(far) = &d.entities[2].kind else {
        panic!()
    };
    assert_eq!(far.center.x, -1_000_000.5);
    assert_eq!(far.thickness, 2.0);
    let extra_codes: Vec<i16> = d.entities[3].extra.iter().map(|p| p.code).collect();
    assert_eq!(extra_codes, [102, 330, 102, 1001, 1000, 1040, 1070]);
    let EntityKind::Arc(flipped) = &d.entities[4].kind else {
        panic!()
    };
    assert_eq!(flipped.extrusion.z, -1.0);
}

#[test]
fn empty_values() {
    let (d, w) = load("empty_values.dxf");
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(d.header.text_style(), Some(""));
    assert_eq!(d.entities.len(), 2);
    assert_eq!(parsed(&d), 1);
}

#[test]
fn damaged_entities_degrade_to_unknown_with_warnings() {
    let (d, w) = load("damaged_entities.dxf");
    assert_eq!(d.entities.len(), 5);
    assert_eq!(parsed(&d), 1);
    assert_eq!(w.len(), 4, "{w:?}");
    assert!(w.iter().all(|w| w.line.is_some()));
}

#[test]
fn utf8_names() {
    let (d, w) = load("utf8_names.dxf");
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(d.header.current_layer(), Some("Wände"));
    assert!(d.tables.layers.contains("WÄNDE"));
    assert!(d.tables.layers.contains(&d.entities[0].layer));
    assert_eq!(d.entities[1].layer, "配管");
}

/// Truncating every corpus file at every byte, and corrupting bytes, must
/// never panic. A cheap stand-in until cargo-fuzz arrives in M2.
#[test]
fn damaged_input_never_panics() {
    let mut seed: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for path in corpus_files() {
        let bytes = std::fs::read(&path).unwrap();
        for cut in 0..=bytes.len() {
            let _ = read_bytes(&bytes[..cut]);
        }
        for _ in 0..300 {
            let mut damaged = bytes.clone();
            for _ in 0..1 + next() % 4 {
                let at = (next() % damaged.len() as u64) as usize;
                damaged[at] = match next() % 4 {
                    0 => b'\n',
                    1 => b'-',
                    2 => b'0' + (next() % 10) as u8,
                    _ => next() as u8,
                };
            }
            let _ = read_bytes(&damaged);
        }
    }
}
