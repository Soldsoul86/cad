//! Drawing entities.

use crate::model::{Color, Handle, LineWeight, Point3};
use crate::tokenizer::Pair;
use crate::value::Value;

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

/// One vertex of a [`LwPolyline`], in the entity's coordinate system.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LwVertex {
    pub x: f64,
    pub y: f64,
    pub start_width: f64,
    pub end_width: f64,
    /// Bulge of the segment from this vertex to the next (see
    /// `cad_geom::bulge`); 0 for a straight segment.
    pub bulge: f64,
    /// Group 91 (R2010+), if present.
    pub id: Option<i32>,
}

/// A lightweight polyline (`LWPOLYLINE`).
#[derive(Debug, Clone, PartialEq)]
pub struct LwPolyline {
    /// Group 70: 1 = closed, 128 = linetype generation across vertices.
    pub flags: i16,
    /// Group 43, if the file sets one width for every segment.
    pub constant_width: Option<f64>,
    pub elevation: f64,
    pub thickness: f64,
    pub extrusion: Point3,
    pub vertices: Vec<LwVertex>,
}

impl LwPolyline {
    pub fn is_closed(&self) -> bool {
        self.flags & 1 != 0
    }
}

/// An ellipse or elliptical arc.
#[derive(Debug, Clone, PartialEq)]
pub struct Ellipse {
    pub center: Point3,
    /// End point of the major axis, relative to the center.
    pub major_axis: Point3,
    /// Minor axis length divided by major axis length.
    pub ratio: f64,
    /// Start parameter in radians (0 for a full ellipse).
    pub start_param: f64,
    /// End parameter in radians (2π for a full ellipse).
    pub end_param: f64,
    pub extrusion: Point3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub location: Point3,
    pub thickness: f64,
    /// Group 50: angle of the X axis used when displaying the point, degrees.
    pub x_axis_angle: f64,
    pub extrusion: Point3,
}

/// Single-line text (`TEXT`); also the text part of [`Attrib`] and
/// [`AttDef`].
///
/// `value` is the decoded text (`\U+XXXX`, `\M+nXXXX` and `%%` codes
/// resolved, see [`crate::decode_text`]); `raw_value` keeps the string as
/// written when it differs, so the file can be saved back unchanged.
#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    /// First alignment point (group 10).
    pub insert: Point3,
    /// Second alignment point (group 11), used when the text is not
    /// left/baseline aligned.
    pub align_point: Option<Point3>,
    pub height: f64,
    pub value: String,
    /// The value as written, if decoding changed it.
    pub raw_value: Option<String>,
    /// Degrees.
    pub rotation: f64,
    pub width_factor: f64,
    /// Degrees.
    pub oblique: f64,
    pub style: String,
    /// Group 71: 2 = backward, 4 = upside down.
    pub generation_flags: i16,
    /// Group 72: 0 left, 1 center, 2 right, 3 aligned, 4 middle, 5 fit.
    pub horizontal_align: i16,
    /// Group 73 (74 in attributes): 0 baseline, 1 bottom, 2 middle, 3 top.
    pub vertical_align: i16,
    pub thickness: f64,
    pub extrusion: Point3,
}

/// An attribute value attached to an [`Insert`] (`ATTRIB`).
#[derive(Debug, Clone, PartialEq)]
pub struct Attrib {
    pub text: Text,
    pub tag: String,
    /// Group 70: 1 = invisible, 2 = constant, 4 = verify, 8 = preset.
    pub flags: i16,
    /// Group 73.
    pub field_length: i16,
}

/// An attribute definition inside a block (`ATTDEF`).
#[derive(Debug, Clone, PartialEq)]
pub struct AttDef {
    /// The default value and its placement.
    pub text: Text,
    pub tag: String,
    /// Group 3: the prompt shown when inserting, as written.
    pub prompt: String,
    /// Group 70: 1 = invisible, 2 = constant, 4 = verify, 8 = preset.
    pub flags: i16,
    /// Group 73.
    pub field_length: i16,
}

/// Multi-line text (`MTEXT`).
///
/// `text` is the full string (the `3` chunks and final `1` pair joined) with
/// character escapes decoded (see [`crate::decode_mtext`]) but inline
/// formatting codes (`\P`, `\f...;`, ...) kept. `raw_text` keeps the string
/// as written when it differs.
#[derive(Debug, Clone, PartialEq)]
pub struct MText {
    pub insert: Point3,
    pub height: f64,
    /// Group 41: reference rectangle width, 0 for no wrapping.
    pub reference_width: f64,
    /// Group 71: 1 top-left ... 9 bottom-right.
    pub attachment: i16,
    /// Group 72: 1 left to right, 3 top to bottom, 5 by style.
    pub drawing_direction: i16,
    pub text: String,
    /// The text as written, if decoding changed it.
    pub raw_text: Option<String>,
    pub style: String,
    /// Group 11: X-axis direction; takes precedence over `rotation`.
    pub x_direction: Option<Point3>,
    /// Group 50, degrees.
    pub rotation: f64,
    /// Group 73: 1 at least, 2 exact.
    pub line_spacing_style: i16,
    /// Group 44.
    pub line_spacing_factor: f64,
    pub extrusion: Point3,
}

/// A block reference (`INSERT`), possibly a rectangular array (`MINSERT`
/// semantics via column/row counts) and possibly carrying attributes.
#[derive(Debug, Clone, PartialEq)]
pub struct Insert {
    pub block: String,
    pub insert: Point3,
    /// Groups 41, 42, 43; default 1.
    pub scale: Point3,
    /// Degrees.
    pub rotation: f64,
    pub columns: i16,
    pub rows: i16,
    pub column_spacing: f64,
    pub row_spacing: f64,
    pub extrusion: Point3,
    /// The `ATTRIB` entities that followed the insert, each with
    /// [`EntityKind::Attrib`] (or `Unknown` if one could not be parsed).
    pub attributes: Vec<Entity>,
    /// The closing `SEQEND`, if the attributes had one.
    pub seqend: Option<Box<Entity>>,
}

/// The geometry that differs between dimension types.
#[derive(Debug, Clone, PartialEq)]
pub enum DimensionKind {
    /// Type 0: horizontal, vertical or rotated.
    Linear {
        /// Group 13: first extension line origin.
        first: Point3,
        /// Group 14: second extension line origin.
        second: Point3,
        /// Group 50, degrees.
        angle: f64,
        /// Group 52, degrees.
        oblique: f64,
    },
    /// Type 1.
    Aligned {
        first: Point3,
        second: Point3,
        oblique: f64,
    },
    /// Type 2: angle between two lines.
    Angular {
        /// Groups 13 and 14: the first line.
        first_start: Point3,
        first_end: Point3,
        /// Group 15: start of the second line (its end is the definition
        /// point, group 10).
        second_start: Point3,
        /// Group 16: point on the dimension arc.
        arc_point: Point3,
    },
    /// Type 3.
    Diameter {
        /// Group 15: the chord point opposite the definition point.
        chord_point: Point3,
        leader_length: f64,
    },
    /// Type 4.
    Radius {
        /// Group 15: point on the arc (the definition point is the center).
        chord_point: Point3,
        leader_length: f64,
    },
    /// Type 5: angle given by a vertex and two points.
    Angular3Point {
        first: Point3,
        second: Point3,
        vertex: Point3,
    },
    /// Type 6.
    Ordinate {
        /// Group 13.
        feature: Point3,
        /// Group 14.
        leader_end: Point3,
        /// Flag 64: measures X rather than Y.
        x_type: bool,
    },
}

/// A dimension (`DIMENSION`). The drawn geometry lives in the anonymous
/// block named by `block`.
#[derive(Debug, Clone, PartialEq)]
pub struct Dimension {
    pub block: String,
    pub style: String,
    /// Group 10.
    pub definition_point: Point3,
    /// Group 11.
    pub text_midpoint: Point3,
    /// Group 1: text override, decoded; empty means the measured value.
    pub text_override: String,
    /// The override as written, if decoding changed it.
    pub raw_text_override: Option<String>,
    /// The flag bits of group 70 above the type (32, 64, 128).
    pub flags: i16,
    /// Group 71, if present.
    pub attachment: Option<i16>,
    /// Group 42: the measured value, if stored (R2000+).
    pub measurement: Option<f64>,
    /// Group 53, degrees.
    pub text_rotation: f64,
    /// Group 51, degrees.
    pub horizontal_direction: f64,
    pub extrusion: Point3,
    pub kind: DimensionKind,
}

/// A NURBS curve (`SPLINE`), read for display only.
#[derive(Debug, Clone, PartialEq)]
pub struct Spline {
    /// Group 70: 1 closed, 2 periodic, 4 rational, 8 planar, 16 linear.
    pub flags: i16,
    pub degree: i16,
    pub knots: Vec<f64>,
    /// Empty when every weight is 1.
    pub weights: Vec<f64>,
    pub control_points: Vec<Point3>,
    pub fit_points: Vec<Point3>,
    pub knot_tolerance: Option<f64>,
    pub control_tolerance: Option<f64>,
    pub fit_tolerance: Option<f64>,
    pub start_tangent: Option<Point3>,
    pub end_tangent: Option<Point3>,
    /// Group 210, if the spline is planar.
    pub normal: Option<Point3>,
}

/// A heavy polyline (`POLYLINE`): 2D and 3D polylines, polyface and polygon
/// meshes. Its vertices are the `VERTEX` entities that followed it.
#[derive(Debug, Clone, PartialEq)]
pub struct Polyline {
    /// Group 70: 1 closed, 8 3D polyline, 16 polygon mesh, 64 polyface, ...
    pub flags: i16,
    /// Groups 10/20/30: x and y are always 0; z is the elevation.
    pub location: Point3,
    pub default_start_width: f64,
    pub default_end_width: f64,
    pub thickness: f64,
    /// Groups 71/72: mesh M and N vertex counts (or polyface counts).
    pub mesh_counts: (i16, i16),
    /// Groups 73/74: smooth surface densities.
    pub smooth_densities: (i16, i16),
    /// Group 75: 0 none, 5 quadratic, 6 cubic, 8 Bezier.
    pub curve_type: i16,
    pub extrusion: Point3,
    /// Each with [`EntityKind::Vertex`] (or `Unknown` if one could not be
    /// parsed).
    pub vertices: Vec<Entity>,
    /// The closing `SEQEND`.
    pub seqend: Option<Box<Entity>>,
}

impl Polyline {
    pub fn is_closed(&self) -> bool {
        self.flags & 1 != 0
    }
}

/// A `VERTEX` of a [`Polyline`].
#[derive(Debug, Clone, PartialEq)]
pub struct Vertex {
    pub location: Point3,
    pub start_width: f64,
    pub end_width: f64,
    pub bulge: f64,
    /// Group 70: 1 extra (curve fit), 8 spline vertex, 32 3D polyline vertex,
    /// 64 mesh vertex, 128 polyface face, ...
    pub flags: i16,
    /// Group 50: curve-fit tangent direction, degrees.
    pub tangent: Option<f64>,
    /// Groups 71–74: polyface face vertex indices (0 if unused).
    pub indices: [i16; 4],
    /// Group 91 (R2010+), if present.
    pub id: Option<i32>,
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
    LwPolyline(LwPolyline),
    Ellipse(Ellipse),
    Point(Point),
    Text(Text),
    MText(MText),
    Insert(Insert),
    Dimension(Box<Dimension>),
    Spline(Box<Spline>),
    Polyline(Polyline),
    /// Only inside [`Insert::attributes`].
    Attrib(Attrib),
    /// Normally inside a block definition.
    AttDef(AttDef),
    /// Only inside [`Polyline::vertices`].
    Vertex(Vertex),
    Unknown(Unknown),
}

impl EntityKind {
    /// The DXF type name, e.g. `LINE`.
    pub fn type_name(&self) -> &str {
        match self {
            Self::Line(_) => "LINE",
            Self::Circle(_) => "CIRCLE",
            Self::Arc(_) => "ARC",
            Self::LwPolyline(_) => "LWPOLYLINE",
            Self::Ellipse(_) => "ELLIPSE",
            Self::Point(_) => "POINT",
            Self::Text(_) => "TEXT",
            Self::MText(_) => "MTEXT",
            Self::Insert(_) => "INSERT",
            Self::Dimension(_) => "DIMENSION",
            Self::Spline(_) => "SPLINE",
            Self::Polyline(_) => "POLYLINE",
            Self::Attrib(_) => "ATTRIB",
            Self::AttDef(_) => "ATTDEF",
            Self::Vertex(_) => "VERTEX",
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
    /// `102` groups, embedded objects, unknown codes), in file order. Empty
    /// for `Unknown`.
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
