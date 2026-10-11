//! Files the user hands over, in plain words: what a file that cannot be
//! read is and what to do about it, which of two files is the earlier
//! revision, and notes on comparisons that are probably not what the user
//! meant (the same file twice, unrelated drawings). Pure functions, so the
//! wording is tested.

use lightable_dxf::{DxfError, Warning};

/// A problem shown in its own window: a heading and what to do next.
#[derive(Debug, Clone, PartialEq)]
pub struct Problem {
    pub title: String,
    pub body: String,
    /// The reader's own message, for "Details".
    pub detail: Option<String>,
}

fn problem(title: impl Into<String>, body: impl Into<String>, detail: Option<String>) -> Problem {
    Problem {
        title: title.into(),
        body: body.into(),
        detail,
    }
}

/// The file name without a folder.
pub fn short(name: &str) -> &str {
    name.rsplit(['/', '\\']).next().unwrap_or(name)
}

/// Megabytes, for "Reading big.dxf (50 MB)…".
pub fn size(bytes: usize) -> String {
    match bytes {
        n if n >= 1_000_000 => format!("{:.0} MB", n as f64 / 1e6),
        n if n >= 1_000 => format!("{:.0} kB", n as f64 / 1e3),
        n => format!("{n} bytes"),
    }
}

/// What a file that could not be read as DXF most likely is, and what to
/// do instead.
pub fn unreadable(name: &str, bytes: &[u8], err: &DxfError) -> Problem {
    let file = short(name);
    let detail = Some(clip(&err.to_string()));
    let starts = |p: &[u8]| bytes.starts_with(p);
    let lower = file.to_ascii_lowercase();
    if starts(b"AC10") || starts(b"AC1.") || lower.ends_with(".dwg") {
        return problem(
            format!("{file} is a DWG file"),
            "Lightable reads DXF files; DWG is not supported yet. In your CAD program, \
             open the drawing and use Save As (or Export) with the file type \"DXF\", \
             then choose the DXF file here.",
            detail,
        );
    }
    if starts(b"%PDF") {
        return problem(
            format!("{file} is a PDF"),
            "Lightable compares the drawing data in DXF files, not printed sheets. \
             Export both revisions as DXF from the program that made them.",
            detail,
        );
    }
    let picture = starts(b"\x89PNG") || starts(b"\xff\xd8\xff") || starts(b"GIF8");
    if picture {
        return problem(
            format!("{file} is a picture"),
            "Lightable compares DXF drawing files, not images. Export the drawing as DXF.",
            detail,
        );
    }
    if starts(b"PK\x03\x04") {
        return problem(
            format!("{file} is a compressed file"),
            "This looks like a ZIP-based file (for example a ZIP archive or an office \
             document). If it contains DXF files, unpack it and choose the DXF files.",
            detail,
        );
    }
    if bytes.iter().all(u8::is_ascii_whitespace) {
        return problem(
            format!("{file} is empty"),
            "The file has no content. Save the drawing again as DXF and choose the new file.",
            detail,
        );
    }
    if matches!(err, DxfError::BinaryNotSupported) {
        return problem(
            format!("{file} is a binary DXF"),
            "Binary DXF is not supported yet. Save the drawing as ASCII (text) DXF \
             and choose that file.",
            detail,
        );
    }
    let text = bytes.iter().take(4096).filter(|b| **b == 0).count() == 0;
    problem(
        format!("{file} is not a DXF file Lightable can read"),
        if text {
            "The file does not have the structure of a DXF drawing. Choose a DXF file \
             saved by a CAD program."
        } else {
            "The file is not text, so it is not an ASCII DXF drawing. Choose a DXF file \
             saved by a CAD program."
        },
        detail,
    )
}

/// A file that was read but ends before its last section is complete
/// (cut off while copying or saving). Comparing it would report false
/// changes, so it is refused.
pub fn incomplete(name: &str, warnings: &[Warning]) -> Option<Problem> {
    let cut = warnings
        .iter()
        .find(|w| w.message.contains("has no ENDSEC"))?;
    Some(problem(
        format!("{} looks incomplete", short(name)),
        "The file ends before the drawing does, as if it was cut off while being \
         copied, downloaded or saved. Comparing it would report changes that are not \
         real. Save or copy the file again and choose the new copy.",
        Some(cut.to_string()),
    ))
}

/// Puts two files in revision order: the one changed earlier first; with
/// no dates or equal dates, by name ("rev-a" before "rev-b"). Returns
/// whether the order was changed.
pub fn earlier_first<T>(files: &mut [(String, T, Option<f64>)]) -> bool {
    if files.len() != 2 {
        return false;
    }
    let swap = match (files[0].2, files[1].2) {
        (Some(a), Some(b)) if a != b => a > b,
        _ => natural_gt(short(&files[0].0), short(&files[1].0)),
    };
    if swap {
        files.swap(0, 1);
    }
    swap
}

/// Name order with numbers compared as numbers ("rev 9" before "rev 10").
fn natural_gt(a: &str, b: &str) -> bool {
    fn key(s: &str) -> Vec<(u8, u64, String)> {
        let mut out = Vec::new();
        let mut chars = s.chars().peekable();
        while let Some(&c) = chars.peek() {
            let mut part = String::new();
            let digits = c.is_ascii_digit();
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() != digits {
                    break;
                }
                part.push(d.to_ascii_lowercase());
                chars.next();
            }
            if digits {
                out.push((0, part.parse().unwrap_or(u64::MAX), String::new()));
            } else {
                out.push((1, 0, part));
            }
        }
        out
    }
    key(a) > key(b)
}

/// Notes on a comparison that is probably not what the user meant.
pub fn notes(same_bytes: bool, diff: &lightable_diff::Diff) -> Vec<String> {
    if same_bytes {
        return vec![
            "You chose the same file twice (both files have identical content). \
             Choose the earlier and the later revision."
                .to_owned(),
        ];
    }
    let matched =
        diff.unchanged + diff.equivalent + diff.count(lightable_diff::ChangeKind::Changed);
    let all = matched
        + diff.count(lightable_diff::ChangeKind::Added)
        + diff.count(lightable_diff::ChangeKind::Deleted);
    if all >= 10 && matched * 10 < all {
        return vec![
            "These drawings have almost nothing in common, so nearly everything shows \
             as added or deleted. Check that they are two revisions of the same drawing."
                .to_owned(),
        ];
    }
    Vec::new()
}

/// A reader message shortened for display (some quote whole binary lines).
fn clip(s: &str) -> String {
    let clean: String = s
        .chars()
        .map(|c| if c.is_control() { '·' } else { c })
        .collect();
    match clean.char_indices().nth(120) {
        Some((i, _)) => format!("{}…", &clean[..i]),
        None => clean,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_problem(name: &str, bytes: &[u8]) -> Problem {
        let err = lightable_dxf::read_bytes(bytes).expect_err("should not read");
        unreadable(name, bytes, &err)
    }

    #[test]
    fn a_dwg_file_says_how_to_get_a_dxf() {
        let p = read_problem("bracket.dwg", b"AC1032\0\0\0\0\0\0\x03\0\0");
        assert_eq!(p.title, "bracket.dwg is a DWG file");
        assert!(p.body.contains("DWG is not supported yet"));
        assert!(p.body.contains("Save As"));
        assert!(p.body.contains("\"DXF\""));
    }

    #[test]
    fn other_file_kinds_are_named() {
        assert_eq!(
            read_problem("a.pdf", b"%PDF-1.7\n1 0 obj\n").title,
            "a.pdf is a PDF"
        );
        assert_eq!(
            read_problem("x.png", b"\x89PNG\r\n\x1a\n....").title,
            "x.png is a picture"
        );
        assert_eq!(
            read_problem("x.zip", b"PK\x03\x04....").title,
            "x.zip is a compressed file"
        );
        assert_eq!(read_problem("e.dxf", b"").title, "e.dxf is empty");
        assert_eq!(
            read_problem("folder/notes.txt", b"hello\nworld\n").title,
            "notes.txt is not a DXF file Lightable can read"
        );
    }

    #[test]
    fn binary_garbage_is_not_shown_to_the_user() {
        let bytes: Vec<u8> = (0..2000u32).map(|i| (i * 7919 % 251) as u8).collect();
        let p = read_problem("noise.dxf", &bytes);
        assert!(p.body.contains("not text"));
        let detail = p.detail.unwrap();
        assert!(detail.chars().count() <= 121);
        assert!(!detail.chars().any(char::is_control));
    }

    #[test]
    fn a_cut_off_file_is_refused() {
        let full = include_bytes!("../assets/bracket-rev-b.dxf");
        let (_, warnings) = lightable_dxf::read_bytes(&full[..3000]).unwrap();
        let p = incomplete("cut.dxf", &warnings).expect("refused");
        assert_eq!(p.title, "cut.dxf looks incomplete");
        assert!(p.body.contains("Save or copy the file again"));
        let (_, ok) = lightable_dxf::read_bytes(full).unwrap();
        assert!(incomplete("ok.dxf", &ok).is_none());
    }

    #[test]
    fn the_earlier_revision_comes_first() {
        let mut by_date = [
            ("b.dxf".to_owned(), (), Some(5.0)),
            ("a.dxf".to_owned(), (), Some(9.0)),
        ];
        assert!(!earlier_first(&mut by_date));
        let mut newer_first = [
            ("old.dxf".to_owned(), (), Some(9.0)),
            ("new.dxf".to_owned(), (), Some(5.0)),
        ];
        assert!(earlier_first(&mut newer_first));
        assert_eq!(newer_first[0].0, "new.dxf");
        let mut by_name = [
            ("x/rev 10.dxf".to_owned(), (), None),
            ("x/rev 9.dxf".to_owned(), (), None),
        ];
        assert!(earlier_first(&mut by_name));
        assert_eq!(by_name[0].0, "x/rev 9.dxf");
        let mut same_date = [
            ("Rev-B.dxf".to_owned(), (), Some(1.0)),
            ("rev-a.dxf".to_owned(), (), Some(1.0)),
        ];
        assert!(earlier_first(&mut same_date));
    }

    #[test]
    fn notes_for_the_same_file_and_for_unrelated_drawings() {
        let ex = |id: &str| {
            let e = crate::examples::find(id);
            (
                lightable_dxf::read_bytes(e.a).unwrap().0,
                lightable_dxf::read_bytes(e.b).unwrap().0,
            )
        };
        let (a, b) = ex("bracket");
        let (_, plan) = ex("plan");
        let options = lightable_diff::DiffOptions::default();
        assert!(notes(false, &lightable_diff::diff(&a, &b, &options)).is_empty());
        let same = notes(true, &lightable_diff::diff(&b, &b, &options));
        assert!(same[0].starts_with("You chose the same file twice"));
        let unrelated = notes(false, &lightable_diff::diff(&a, &plan, &options));
        assert!(
            unrelated[0].contains("almost nothing in common"),
            "{unrelated:?}"
        );
    }

    #[test]
    fn sizes_read_naturally() {
        assert_eq!(size(57_448_175), "57 MB");
        assert_eq!(size(63_627), "64 kB");
        assert_eq!(size(12), "12 bytes");
    }
}
