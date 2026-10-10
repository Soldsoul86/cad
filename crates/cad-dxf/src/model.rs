//! The in-memory drawing model.

use std::fmt;

use crate::tokenizer::Pair;
use crate::value::{Value, parse_f64, parse_int};

/// A point or vector in drawing units.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point3 {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

/// The default extrusion direction (the world Z axis).
pub const Z_AXIS: Point3 = Point3::new(0.0, 0.0, 1.0);

/// An object handle: a hexadecimal identifier unique within the drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Handle(pub u64);

impl Handle {
    pub fn parse(raw: &str) -> Option<Self> {
        let text = raw.trim();
        if text.is_empty() || text.len() > 16 {
            return None;
        }
        u64::from_str_radix(text, 16).ok().map(Self)
    }
}

impl fmt::Display for Handle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:X}", self.0)
    }
}

/// DXF file format versions this library targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Version {
    R12,
    R2000,
    R2004,
    R2007,
    R2010,
    R2013,
    R2018,
}

impl Version {
    /// Maps a `$ACADVER` value such as `AC1032`.
    pub fn from_acadver(raw: &str) -> Option<Self> {
        Some(match raw.trim() {
            "AC1009" => Self::R12,
            "AC1015" => Self::R2000,
            "AC1018" => Self::R2004,
            "AC1021" => Self::R2007,
            "AC1024" => Self::R2010,
            "AC1027" => Self::R2013,
            "AC1032" => Self::R2018,
            _ => return None,
        })
    }

    pub fn acadver(self) -> &'static str {
        match self {
            Self::R12 => "AC1009",
            Self::R2000 => "AC1015",
            Self::R2004 => "AC1018",
            Self::R2007 => "AC1021",
            Self::R2010 => "AC1024",
            Self::R2013 => "AC1027",
            Self::R2018 => "AC1032",
        }
    }
}

/// An entity or layer colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Color {
    ByBlock,
    #[default]
    ByLayer,
    /// ACI colour index, 1–255.
    Index(u8),
    /// 24-bit true colour.
    Rgb(u8, u8, u8),
}

impl Color {
    /// From a group 62 value (0 = ByBlock, 256 = ByLayer, 1–255 = index).
    pub fn from_aci(value: i64) -> Option<Self> {
        match value {
            0 => Some(Self::ByBlock),
            256 => Some(Self::ByLayer),
            1..=255 => Some(Self::Index(value as u8)),
            _ => None,
        }
    }

    /// From a group 420 value (`0x00RRGGBB`). Some writers set flag bits in
    /// the top byte, so only the low 24 bits are used.
    pub fn from_true_color(value: i64) -> Option<Self> {
        if !(i64::from(i32::MIN)..=i64::from(u32::MAX)).contains(&value) {
            return None;
        }
        let rgb = value & 0x00FF_FFFF;
        Some(Self::Rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8))
    }
}

/// Line weight (group 370).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineWeight {
    ByLayer,
    ByBlock,
    Default,
    /// Hundredths of a millimetre (0–211).
    Hundredths(u16),
}

impl LineWeight {
    pub fn from_code(value: i64) -> Option<Self> {
        match value {
            -1 => Some(Self::ByLayer),
            -2 => Some(Self::ByBlock),
            -3 => Some(Self::Default),
            0..=211 => Some(Self::Hundredths(value as u16)),
            _ => None,
        }
    }
}

/// One `$NAME` header variable and its pairs, as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderVariable {
    pub name: String,
    pub pairs: Vec<Pair>,
}

/// The `HEADER` section.
///
/// All variables are kept in file order in [`Header::variables`]; the typed
/// accessors read from there, so there is a single source of truth.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Header {
    pub variables: Vec<HeaderVariable>,
}

impl Header {
    /// Looks up a variable by name (e.g. `"$INSUNITS"`), ignoring ASCII case.
    pub fn get(&self, name: &str) -> Option<&HeaderVariable> {
        self.variables
            .iter()
            .find(|v| v.name.eq_ignore_ascii_case(name))
    }

    fn raw(&self, name: &str, code: i16) -> Option<&str> {
        self.get(name)?
            .pairs
            .iter()
            .find(|p| p.code == code)
            .map(|p| p.raw.as_str())
    }

    fn name_value(&self, name: &str, code: i16) -> Option<&str> {
        self.raw(name, code).map(str::trim)
    }

    fn point(&self, name: &str) -> Option<Point3> {
        let coord = |code| self.raw(name, code).and_then(parse_f64);
        Some(Point3::new(
            coord(10)?,
            coord(20)?,
            coord(30).unwrap_or(0.0),
        ))
    }

    /// The raw `$ACADVER` string, e.g. `AC1032`.
    pub fn acadver(&self) -> Option<&str> {
        self.name_value("$ACADVER", 1)
    }

    /// `$ACADVER` mapped to a supported [`Version`].
    pub fn version(&self) -> Option<Version> {
        self.acadver().and_then(Version::from_acadver)
    }

    /// `$DWGCODEPAGE`, e.g. `ANSI_1252`.
    pub fn codepage(&self) -> Option<&str> {
        self.name_value("$DWGCODEPAGE", 3)
    }

    /// `$INSUNITS` (0 = unitless, 1 = inches, 4 = millimetres, 6 = metres, ...).
    pub fn insunits(&self) -> Option<i16> {
        let value = parse_int(self.raw("$INSUNITS", 70)?)?;
        i16::try_from(value).ok()
    }

    pub fn extmin(&self) -> Option<Point3> {
        self.point("$EXTMIN")
    }

    pub fn extmax(&self) -> Option<Point3> {
        self.point("$EXTMAX")
    }

    /// `$CLAYER`: the current layer.
    pub fn current_layer(&self) -> Option<&str> {
        self.name_value("$CLAYER", 8)
    }

    /// `$TEXTSTYLE`: the current text style.
    pub fn text_style(&self) -> Option<&str> {
        self.name_value("$TEXTSTYLE", 7)
    }

    /// `$DIMSTYLE`: the current dimension style.
    pub fn dim_style(&self) -> Option<&str> {
        self.name_value("$DIMSTYLE", 2)
    }
}

/// Table entries are keyed by name.
pub trait Named {
    fn name(&self) -> &str;
}

/// Case-insensitive comparison of table entry names.
pub fn names_equal(a: &str, b: &str) -> bool {
    a.chars()
        .flat_map(char::to_uppercase)
        .eq(b.chars().flat_map(char::to_uppercase))
}

/// A symbol table whose entries are unique by case-insensitive name.
#[derive(Debug, Clone, PartialEq)]
pub struct Table<T> {
    /// Pairs of the table's own record (handle, owner, max entries, ...).
    pub header: Vec<Pair>,
    pub entries: Vec<T>,
    /// Entries that could not be parsed, kept verbatim (including their
    /// leading `0` pair) so they are not lost.
    pub unparsed: Vec<Vec<Pair>>,
}

impl<T> Default for Table<T> {
    fn default() -> Self {
        Self {
            header: Vec::new(),
            entries: Vec::new(),
            unparsed: Vec::new(),
        }
    }
}

impl<T: Named> Table<T> {
    pub fn get(&self, name: &str) -> Option<&T> {
        self.entries.iter().find(|e| names_equal(e.name(), name))
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut T> {
        self.entries
            .iter_mut()
            .find(|e| names_equal(e.name(), name))
    }

    pub fn contains(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Adds `entry`, replacing and returning any entry with the same name.
    pub fn insert(&mut self, entry: T) -> Option<T> {
        match self.get_mut(entry.name()) {
            Some(existing) => Some(std::mem::replace(existing, entry)),
            None => {
                self.entries.push(entry);
                None
            }
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.entries.iter()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A `LAYER` table entry.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub name: String,
    pub handle: Option<Handle>,
    pub owner: Option<Handle>,
    /// Group 70 flags: 1 = frozen, 2 = frozen in new viewports, 4 = locked.
    pub flags: i16,
    pub color: Color,
    /// False when the layer is switched off (stored as a negative colour).
    pub on: bool,
    pub linetype: String,
    pub lineweight: LineWeight,
    pub plot: bool,
    /// Pairs this library does not interpret, in file order.
    pub extra: Vec<Pair>,
}

impl Layer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            handle: None,
            owner: None,
            flags: 0,
            color: Color::Index(7),
            on: true,
            linetype: "CONTINUOUS".into(),
            lineweight: LineWeight::Default,
            plot: true,
            extra: Vec::new(),
        }
    }

    pub fn frozen(&self) -> bool {
        self.flags & 1 != 0
    }

    pub fn locked(&self) -> bool {
        self.flags & 4 != 0
    }
}

impl Named for Layer {
    fn name(&self) -> &str {
        &self.name
    }
}

/// A table this milestone does not parse yet, kept verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawTable {
    /// The table name from the pair after `0 TABLE` (e.g. `LTYPE`).
    pub name: String,
    /// Everything between that name and `0 ENDTAB`.
    pub pairs: Vec<Pair>,
}

/// The `TABLES` section.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Tables {
    pub layers: Table<Layer>,
    /// Linetypes, text styles, dim styles, block records, ... in file order.
    /// Parsed into typed tables in M2.
    pub other: Vec<RawTable>,
}

/// A section this milestone does not parse, kept verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSection {
    pub name: String,
    /// Everything between `2 <name>` and `0 ENDSEC`.
    pub pairs: Vec<Pair>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub start: Point3,
    pub end: Point3,
    pub thickness: f64,
    pub extrusion: Point3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Circle {
    pub center: Point3,
    pub radius: f64,
    pub thickness: f64,
    pub extrusion: Point3,
}

/// A circular arc drawn counter-clockwise from `start_angle` to `end_angle`.
#[derive(Debug, Clone, PartialEq)]
pub struct Arc {
    pub center: Point3,
    pub radius: f64,
    /// Degrees, as stored in the file.
    pub start_angle: f64,
    /// Degrees, as stored in the file.
    pub end_angle: f64,
    pub thickness: f64,
    pub extrusion: Point3,
}

/// An entity this library does not understand (or could not parse).
///
/// `pairs` holds every pair after the `0 <type>` line, unchanged; this is
/// the authoritative content and is what the writer will emit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unknown {
    pub type_name: String,
    pub pairs: Vec<Pair>,
}

impl Unknown {
    /// The pairs as typed values.
    pub fn group_codes(&self) -> impl Iterator<Item = (i16, Value)> + '_ {
        self.pairs.iter().map(|p| (p.code, p.value()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntityKind {
    Line(Line),
    Circle(Circle),
    Arc(Arc),
    Unknown(Unknown),
}

impl EntityKind {
    /// The DXF type name, e.g. `LINE`.
    pub fn type_name(&self) -> &str {
        match self {
            Self::Line(_) => "LINE",
            Self::Circle(_) => "CIRCLE",
            Self::Arc(_) => "ARC",
            Self::Unknown(u) => &u.type_name,
        }
    }
}

/// A drawing entity: common properties plus its geometry.
///
/// For [`EntityKind::Unknown`] the common fields are a best-effort reading of
/// the raw pairs, for convenience (e.g. filtering by layer); the raw pairs
/// remain authoritative.
#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    pub handle: Option<Handle>,
    pub owner: Option<Handle>,
    pub layer: String,
    pub color: Color,
    pub linetype: String,
    pub lineweight: LineWeight,
    pub visible: bool,
    pub paper_space: bool,
    pub kind: EntityKind,
    /// Pairs of a known entity this library does not interpret (XDATA,
    /// `102` groups, unknown codes), in file order. Empty for `Unknown`.
    pub extra: Vec<Pair>,
}

impl Entity {
    pub fn new(kind: EntityKind) -> Self {
        Self {
            handle: None,
            owner: None,
            layer: "0".into(),
            color: Color::ByLayer,
            linetype: "BYLAYER".into(),
            lineweight: LineWeight::ByLayer,
            visible: true,
            paper_space: false,
            kind,
            extra: Vec::new(),
        }
    }
}

/// A whole DXF document.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Drawing {
    pub header: Header,
    /// The `CLASSES` section, verbatim.
    pub classes: Vec<Pair>,
    pub tables: Tables,
    /// The `BLOCKS` section, verbatim until block parsing lands in M2.
    pub blocks: Vec<Pair>,
    pub entities: Vec<Entity>,
    /// The `OBJECTS` section, verbatim.
    pub objects: Vec<Pair>,
    /// Sections with other names (e.g. `THUMBNAILIMAGE`), verbatim.
    pub other_sections: Vec<RawSection>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_from_aci() {
        assert_eq!(Color::from_aci(0), Some(Color::ByBlock));
        assert_eq!(Color::from_aci(256), Some(Color::ByLayer));
        assert_eq!(Color::from_aci(1), Some(Color::Index(1)));
        assert_eq!(Color::from_aci(255), Some(Color::Index(255)));
        assert_eq!(Color::from_aci(257), None);
        assert_eq!(Color::from_aci(-1), None);
    }

    #[test]
    fn true_color_uses_low_24_bits() {
        assert_eq!(
            Color::from_true_color(0x00FF_8000),
            Some(Color::Rgb(255, 128, 0))
        );
        assert_eq!(
            Color::from_true_color(0xC2FF_8000),
            Some(Color::Rgb(255, 128, 0))
        );
        assert_eq!(Color::from_true_color(1 << 40), None);
    }

    #[test]
    fn lineweights() {
        assert_eq!(LineWeight::from_code(-1), Some(LineWeight::ByLayer));
        assert_eq!(LineWeight::from_code(25), Some(LineWeight::Hundredths(25)));
        assert_eq!(LineWeight::from_code(212), None);
    }

    #[test]
    fn handles_are_hex() {
        assert_eq!(Handle::parse(" 1F "), Some(Handle(0x1F)));
        assert_eq!(Handle::parse("zz"), None);
        assert_eq!(Handle::parse(""), None);
        assert_eq!(Handle(0x2A).to_string(), "2A");
    }

    #[test]
    fn versions_round_trip() {
        for v in [Version::R12, Version::R2000, Version::R2018] {
            assert_eq!(Version::from_acadver(v.acadver()), Some(v));
        }
        assert_eq!(Version::from_acadver("AC1014"), None);
    }

    #[test]
    fn tables_are_case_insensitive() {
        let mut table = Table::default();
        assert!(table.insert(Layer::new("Walls")).is_none());
        assert!(table.contains("WALLS"));
        assert!(table.get("walls").is_some());
        assert!(table.contains("wände") == table.contains("WÄNDE"));
        let old = table.insert(Layer::new("WALLS")).expect("replaced");
        assert_eq!(old.name, "Walls");
        assert_eq!(table.len(), 1);
        assert!(names_equal("wände", "WÄNDE"));
    }

    #[test]
    fn header_accessors() {
        let var = |name: &str, pairs: &[(i16, &str)]| HeaderVariable {
            name: name.into(),
            pairs: pairs.iter().map(|&(c, v)| Pair::new(c, v)).collect(),
        };
        let header = Header {
            variables: vec![
                var("$ACADVER", &[(1, "AC1032")]),
                var("$INSUNITS", &[(70, "     4")]),
                var("$EXTMIN", &[(10, "-1.5"), (20, "2"), (30, "0")]),
                var("$extmax", &[(10, "10"), (20, "20")]),
                var("$CLAYER", &[(8, "WALLS")]),
            ],
        };
        assert_eq!(header.version(), Some(Version::R2018));
        assert_eq!(header.insunits(), Some(4));
        assert_eq!(header.extmin(), Some(Point3::new(-1.5, 2.0, 0.0)));
        assert_eq!(header.extmax(), Some(Point3::new(10.0, 20.0, 0.0)));
        assert_eq!(header.current_layer(), Some("WALLS"));
        assert_eq!(header.codepage(), None);
    }
}
