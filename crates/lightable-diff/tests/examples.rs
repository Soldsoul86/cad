//! The built-in examples (crates/lightable/assets, made by
//! tools/gen_example.py) say exactly what changed, in plain words.

use std::path::PathBuf;

use lightable_diff::{ChangeKind, DiffOptions};

fn diff(a: &str, b: &str) -> lightable_diff::Diff {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../lightable/assets");
    let (a, _) = lightable_dxf::read_file(dir.join(a)).unwrap();
    let (b, _) = lightable_dxf::read_file(dir.join(b)).unwrap();
    lightable_diff::diff(&a, &b, &DiffOptions::default())
}

#[test]
fn bracket_rev_c_has_exactly_four_design_changes() {
    let d = diff("bracket-rev-b.dxf", "bracket-rev-c.dxf");
    assert_eq!(d.units.as_deref(), Some("mm"));
    let mut design: Vec<&str> = d.design_changes().map(|c| c.headline.as_str()).collect();
    design.sort();
    assert_eq!(
        design,
        [
            "Circle diameter changed from 12 mm to 14 mm",
            "Circle Ø12 moved 5 mm right",
            "Polyline corner at (200, 160) chamfered 10 × 10 mm",
            "Text changed from M12 to M16",
        ]
    );
    let mut markup: Vec<&str> = d
        .changes
        .iter()
        .filter(|c| c.markup)
        .map(|c| c.headline.as_str())
        .collect();
    markup.sort();
    assert_eq!(
        markup,
        ["Revision cloud added", "Text changed from REV B to REV C"]
    );
    // Design changes come first, then markup.
    assert!(d.changes.iter().take(4).all(|c| !c.markup));
    assert!(
        d.changes
            .iter()
            .all(|c| c.certain && c.shown && c.bounds.is_some())
    );
}

#[test]
fn plan_example_headlines_use_millimetres() {
    let d = diff("plan-rev-a.dxf", "plan-rev-b.dxf");
    let all: Vec<&str> = d.changes.iter().map(|c| c.headline.as_str()).collect();
    for want in [
        "Arc R900 moved 600 mm up",
        "Text changed from BEDROOM to MASTER BEDROOM",
        "Line deleted",
        "In block CHAIR: Polyline added",
        "Layer FURNITURE: color changed from 8 to 6",
    ] {
        assert!(all.contains(&want), "missing {want:?} in {all:#?}");
    }
    assert_eq!(d.count(ChangeKind::Deleted), 1);
}

#[test]
fn without_insunits_lengths_have_no_unit() {
    // The test pairs are in metres ($INSUNITS = 6); a drawing without
    // $INSUNITS gets bare numbers.
    let line = |y: f64| {
        let mut e =
            lightable_dxf::Entity::new(lightable_dxf::EntityKind::Line(lightable_dxf::Line {
                start: lightable_dxf::Point3::new(0.0, y, 0.0),
                end: lightable_dxf::Point3::new(10.0, y, 0.0),
                thickness: 0.0,
                extrusion: lightable_dxf::Z_AXIS,
            }));
        e.handle = Some(lightable_dxf::Handle(1));
        e
    };
    let a = lightable_dxf::Drawing {
        entities: vec![line(0.0)],
        ..Default::default()
    };
    let b = lightable_dxf::Drawing {
        entities: vec![line(2.0)],
        ..Default::default()
    };
    let d = lightable_diff::diff(&a, &b, &DiffOptions::default());
    assert_eq!(d.units, None);
    assert_eq!(d.changes[0].headline, "Line moved 2 up");
}
