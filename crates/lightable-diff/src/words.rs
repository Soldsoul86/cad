//! One-line headlines for changes, in plain words: "Circle Ø12 moved 5 mm
//! right", "Circle diameter changed from 12 mm to 14 mm", "Text changed
//! from M12 to M16". Every headline is computed from the values compared,
//! so it is mechanically true; lengths carry a unit only when the drawing's
//! `$INSUNITS` gives one.
//!
//! Also decides what counts as revision markup (revision clouds and title
//! block text), which the app counts apart from design changes.

use lightable_dxf::{Drawing, Entity, EntityKind};

use crate::features::{Fields, NumKind, Tolerance, Value, fmt_num, fmt_point};

/// The length unit named by `$INSUNITS`, if it names one.
pub fn units_of(d: &Drawing) -> Option<&'static str> {
    Some(match d.header.insunits()? {
        1 => "in",
        2 => "ft",
        4 => "mm",
        5 => "cm",
        6 => "m",
        7 => "km",
        8 => "µin",
        9 => "mil",
        10 => "yd",
        13 => "µm",
        14 => "dm",
        _ => return None,
    })
}

/// A length with its unit, if known: "5 mm" or "5".
pub fn length(v: f64, units: Option<&str>) -> String {
    match units {
        Some(u) => format!("{} {u}", fmt_num(v)),
        None => fmt_num(v),
    }
}

/// "5 mm right", "3 mm left and 2 mm up", for a displacement.
pub fn displacement(d: [f64; 3], tol: f64, units: Option<&str>) -> String {
    let mut parts = Vec::new();
    if d[0].abs() > tol {
        parts.push(format!(
            "{} {}",
            length(d[0].abs(), units),
            if d[0] > 0.0 { "right" } else { "left" }
        ));
    }
    if d[1].abs() > tol {
        parts.push(format!(
            "{} {}",
            length(d[1].abs(), units),
            if d[1] > 0.0 { "up" } else { "down" }
        ));
    }
    if d[2].abs() > tol {
        parts.push(format!("{} along Z", length(d[2], units)));
    }
    parts.join(" and ")
}

fn text_of(e: &Entity) -> Option<String> {
    match &e.kind {
        EntityKind::Text(t) => Some(t.value.clone()),
        EntityKind::Attrib(a) => Some(a.text.value.clone()),
        EntityKind::MText(m) => Some(m.plain_lines().join(" / ")),
        _ => None,
    }
}

/// What the entity is, in a few words: "Circle Ø12", "Arc R5", "Text
/// “M12”", "Block BOLT". `plain` leaves out the value that changed (so the
/// headline says "Circle diameter changed", not "Circle Ø12 diameter").
fn subject(e: &Entity, plain: bool) -> String {
    match &e.kind {
        EntityKind::Line(_) => "Line".into(),
        EntityKind::Circle(_) if plain => "Circle".into(),
        EntityKind::Circle(c) => format!("Circle Ø{}", fmt_num(2.0 * c.radius)),
        EntityKind::Arc(a) if !plain => format!("Arc R{}", fmt_num(a.radius)),
        EntityKind::Arc(_) => "Arc".into(),
        EntityKind::LwPolyline(_) | EntityKind::Polyline(_) => "Polyline".into(),
        EntityKind::Ellipse(_) => "Ellipse".into(),
        EntityKind::Point(_) => "Point".into(),
        EntityKind::Text(_)
        | EntityKind::MText(_)
        | EntityKind::Attrib(_)
        | EntityKind::AttDef(_)
            if plain =>
        {
            "Text".into()
        }
        EntityKind::Text(_) | EntityKind::MText(_) | EntityKind::Attrib(_) => {
            format!("Text “{}”", text_of(e).unwrap_or_default())
        }
        EntityKind::AttDef(a) => format!("Attribute definition {}", a.tag),
        EntityKind::Insert(i) => format!("Block {}", i.block),
        EntityKind::Dimension(_) => "Dimension".into(),
        EntityKind::Spline(_) => "Spline".into(),
        EntityKind::Vertex(_) => "Vertex".into(),
        EntityKind::Unknown(u) => {
            let t = u.type_name.to_lowercase();
            let mut c = t.chars();
            c.next().map_or(String::new(), |f| {
                f.to_uppercase().collect::<String>() + c.as_str()
            })
        }
    }
}

/// Vertices of a polyline as (x, y, bulge), and whether it is closed.
fn vertices(e: &Entity) -> Option<(Vec<[f64; 3]>, bool)> {
    match &e.kind {
        EntityKind::LwPolyline(p) => Some((
            p.vertices.iter().map(|v| [v.x, v.y, v.bulge]).collect(),
            p.is_closed(),
        )),
        EntityKind::Polyline(p) => Some((
            p.vertices
                .iter()
                .filter_map(|v| match &v.kind {
                    EntityKind::Vertex(v) => Some([v.location.x, v.location.y, v.bulge]),
                    _ => None,
                })
                .collect(),
            p.is_closed(),
        )),
        _ => None,
    }
}

/// A revision cloud: a closed polyline of at least three segments, all of
/// them arcs.
pub fn is_revision_cloud(e: &Entity) -> bool {
    vertices(e).is_some_and(|(v, closed)| closed && v.len() >= 3 && v.iter().all(|p| p[2] != 0.0))
}

/// Revision markup rather than a design change: revision clouds, and
/// anything on a layer whose name starts with TITLE, BORDER or REV (title
/// blocks, borders, revision tables).
pub fn is_markup(e: &Entity) -> bool {
    let layer = e.layer.to_uppercase();
    is_revision_cloud(e)
        || ["TITLE", "BORDER", "REV"]
            .iter()
            .any(|p| layer.starts_with(p))
}

/// Distance from `p` to the segment `a`–`b`, and whether its foot lies on
/// the segment.
fn on_segment(p: [f64; 2], a: [f64; 2], b: [f64; 2], tol: f64) -> bool {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len2 = dx * dx + dy * dy;
    if len2 == 0.0 {
        return false;
    }
    let t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2;
    let off = ((p[0] - a[0]) * dy - (p[1] - a[1]) * dx).abs() / len2.sqrt();
    (-1e-9..=1.0 + 1e-9).contains(&t) && off <= tol
}

/// How a polyline's vertex list changed, when it can be said simply:
/// "corner at (160, 100) chamfered 10 × 10 mm", "2 vertices added".
fn vertex_phrase(a: &Entity, b: &Entity, tol: f64, units: Option<&str>) -> Option<String> {
    let ((va, closed), (vb, _)) = (vertices(a)?, vertices(b)?);
    let near = |p: [f64; 3], q: [f64; 3]| (p[0] - q[0]).abs() <= tol && (p[1] - q[1]).abs() <= tol;
    let removed: Vec<usize> = (0..va.len())
        .filter(|&i| !vb.iter().any(|q| near(va[i], *q)))
        .collect();
    let added: Vec<usize> = (0..vb.len())
        .filter(|&j| !va.iter().any(|p| near(vb[j], *p)))
        .collect();
    if removed.len() == 1 && added.len() == 2 && va.len() >= 3 {
        let i = removed[0];
        let n = va.len();
        let at = |k: usize| [va[k][0], va[k][1]];
        let v = at(i);
        if !closed && (i == 0 || i == n - 1) {
            return None;
        }
        let (prev, next) = (at((i + n - 1) % n), at((i + 1) % n));
        let (p, q) = (
            [vb[added[0]][0], vb[added[0]][1]],
            [vb[added[1]][0], vb[added[1]][1]],
        );
        let straight = vb[added[0]][2] == 0.0 && (added[1] == added[0] + 1);
        let on = |x: [f64; 2]| on_segment(x, v, prev, tol) || on_segment(x, v, next, tol);
        let one_each = (on_segment(p, v, prev, tol) && on_segment(q, v, next, tol))
            || (on_segment(p, v, next, tol) && on_segment(q, v, prev, tol));
        if on(p) && on(q) && one_each {
            let d = |x: [f64; 2]| (x[0] - v[0]).hypot(x[1] - v[1]);
            let corner = fmt_point([v[0], v[1], 0.0]);
            return Some(if straight {
                format!(
                    "corner at {corner} chamfered {} × {}",
                    fmt_num(d(p)),
                    length(d(q), units)
                )
            } else {
                format!("corner at {corner} rounded")
            });
        }
    }
    match (removed.len(), added.len()) {
        (0, 0) => None,
        (0, k) => Some(format!(
            "{k} {} added",
            if k == 1 { "vertex" } else { "vertices" }
        )),
        (k, 0) => Some(format!(
            "{k} {} removed",
            if k == 1 { "vertex" } else { "vertices" }
        )),
        (r, k) => Some(format!(
            "{k} {} added and {r} removed",
            if k == 1 { "vertex" } else { "vertices" }
        )),
    }
}

fn value_words(v: &Value, units: Option<&str>) -> String {
    match v {
        Value::Name(s) if s.is_empty() => "(none)".into(),
        Value::Name(s) | Value::Text(s) => s.clone(),
        Value::Num(n, NumKind::Length) => length(*n, units),
        Value::Num(n, NumKind::Angle) => format!("{}°", fmt_num(*n)),
        Value::Num(n, _) => fmt_num(*n),
        Value::Pos(p) | Value::Dir(p) => fmt_point(*p),
    }
}

/// The headline of a changed entity.
#[allow(clippy::too_many_arguments)]
pub fn changed(
    tol: &Tolerance,
    a: &Entity,
    a_common: &Fields,
    a_shape: &Fields,
    b: &Entity,
    b_common: &Fields,
    b_shape: &Fields,
    units: Option<&str>,
) -> String {
    let differs = |label: &str, x: &Fields, y: &Fields| {
        let find = |f: &Fields| f.iter().find(|(l, _)| l == label).map(|(_, v)| v.clone());
        match (find(x), find(y)) {
            (Some(p), Some(q)) => !tol.value_equal(&p, &q),
            (p, q) => p.is_some() != q.is_some(),
        }
    };
    let circle = matches!(a.kind, EntityKind::Circle(_));
    let text_changed = text_of(a).is_some() && differs("text", a_shape, b_shape);
    let radius_changed = differs("radius", a_shape, b_shape);
    let plain = (circle && radius_changed) || text_changed || radius_changed;
    let mut phrases: Vec<String> = Vec::new();
    if let Some(d) = tol.moved_by(a_shape, b_shape) {
        phrases.push(format!("moved {}", displacement(d, tol.length, units)));
    } else {
        let mut vertex_words = None;
        if differs("vertex count", a_shape, b_shape) || vertices(a).is_some() {
            vertex_words = vertex_phrase(a, b, tol.length, units);
        }
        let mut seen_vertices = false;
        for (label, va) in a_shape {
            let Some((_, vb)) = b_shape.iter().find(|(l, _)| l == label) else {
                continue;
            };
            if tol.value_equal(va, vb) || (label == "formatting" && text_changed) {
                continue;
            }
            let is_vertex_field = label.starts_with("vertex")
                || label.starts_with("bulge")
                || label.starts_with("width ");
            if is_vertex_field && vertex_words.is_some() {
                if !seen_vertices {
                    phrases.push(vertex_words.clone().unwrap_or_default());
                    seen_vertices = true;
                }
                continue;
            }
            let label_s = label.as_ref();
            phrases.push(match (label_s, va, vb) {
                ("radius", Value::Num(x, _), Value::Num(y, _)) if circle => format!(
                    "diameter changed from {} to {}",
                    length(2.0 * x, units),
                    length(2.0 * y, units)
                ),
                ("text", _, _) if text_changed => {
                    format!(
                        "changed from {} to {}",
                        value_words(va, units),
                        value_words(vb, units)
                    )
                }
                (_, Value::Pos(x), Value::Pos(y)) => format!(
                    "{label_s} moved {}",
                    displacement([y[0] - x[0], y[1] - x[1], y[2] - x[2]], tol.length, units)
                ),
                (
                    "boundary" | "pattern lines" | "other hatch data" | "data" | "extended data"
                    | "formatting",
                    _,
                    _,
                ) => {
                    format!("{label_s} changed")
                }
                _ => format!(
                    "{label_s} changed from {} to {}",
                    value_words(va, units),
                    value_words(vb, units)
                ),
            });
        }
        if !seen_vertices && let Some(w) = vertex_words {
            phrases.insert(0, w);
        }
    }
    for (label, va) in a_common {
        let Some((_, vb)) = b_common.iter().find(|(l, _)| l == label) else {
            continue;
        };
        if !tol.value_equal(va, vb) {
            phrases.push(format!(
                "{label} changed from {} to {}",
                value_words(va, units),
                value_words(vb, units)
            ));
        }
    }
    if phrases.is_empty() {
        phrases.push("changed".into());
    }
    // "Text changed from M12 to M16": the subject already says what changed.
    format!("{} {}", subject(a, plain), phrases.join("; "))
}

/// The headline of an added or deleted entity.
pub fn added_or_deleted(e: &Entity, added: bool) -> String {
    let what = if is_revision_cloud(e) {
        "Revision cloud".to_owned()
    } else {
        subject(e, false)
    };
    format!("{what} {}", if added { "added" } else { "deleted" })
}

/// A sentence from a detail: first letter upper case.
pub fn sentence(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| {
        f.to_uppercase().collect::<String>() + c.as_str()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displacements_name_their_directions() {
        assert_eq!(
            displacement([5.0, 0.0, 0.0], 1e-9, Some("mm")),
            "5 mm right"
        );
        assert_eq!(
            displacement([-3.0, 2.5, 0.0], 1e-9, None),
            "3 left and 2.5 up"
        );
        assert_eq!(
            displacement([0.0, -1.0, 0.0], 1e-9, Some("in")),
            "1 in down"
        );
    }

    #[test]
    fn segments() {
        assert!(on_segment([5.0, 0.0], [0.0, 0.0], [10.0, 0.0], 1e-9));
        assert!(!on_segment([11.0, 0.0], [0.0, 0.0], [10.0, 0.0], 1e-9));
        assert!(!on_segment([5.0, 1.0], [0.0, 0.0], [10.0, 0.0], 1e-9));
    }
}
