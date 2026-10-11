//! The built-in examples, made by `tools/gen_example.py` with ezdxf (no
//! third-party drawings). The first one opens when the app starts without
//! a file, so the overlay is the first thing a new user sees.

/// Two revisions of a drawing to compare.
pub struct Example {
    /// Short name for the command line and `?example=`.
    pub id: &'static str,
    /// Shown in the example picker.
    pub title: &'static str,
    /// How the two revisions are called in the summary ("Rev B", "Rev C").
    pub a_label: &'static str,
    pub b_label: &'static str,
    pub a_name: &'static str,
    pub b_name: &'static str,
    pub a: &'static [u8],
    pub b: &'static [u8],
}

pub static EXAMPLES: [Example; 2] = [
    Example {
        id: "bracket",
        title: "Steel bracket, Rev B to Rev C",
        a_label: "Rev B",
        b_label: "Rev C",
        a_name: "bracket-rev-b.dxf",
        b_name: "bracket-rev-c.dxf",
        a: include_bytes!("../assets/bracket-rev-b.dxf"),
        b: include_bytes!("../assets/bracket-rev-c.dxf"),
    },
    Example {
        id: "plan",
        title: "Apartment plan, Rev A to Rev B",
        a_label: "Rev A",
        b_label: "Rev B",
        a_name: "plan-rev-a.dxf",
        b_name: "plan-rev-b.dxf",
        a: include_bytes!("../assets/plan-rev-a.dxf"),
        b: include_bytes!("../assets/plan-rev-b.dxf"),
    },
];

/// The example called `id`; the default one for an empty or unknown id.
pub fn find(id: &str) -> &'static Example {
    EXAMPLES.iter().find(|e| e.id == id).unwrap_or(&EXAMPLES[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn examples_load_and_the_bracket_is_the_default() {
        assert_eq!(find("").id, "bracket");
        assert_eq!(find("plan").id, "plan");
        for e in &EXAMPLES {
            assert!(lightable_dxf::read_bytes(e.a).is_ok(), "{}", e.a_name);
            assert!(lightable_dxf::read_bytes(e.b).is_ok(), "{}", e.b_name);
        }
    }
}
