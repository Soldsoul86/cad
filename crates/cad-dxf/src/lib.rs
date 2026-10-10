//! Reads ASCII DXF drawings into a plain Rust model.
//!
//! Anything the library does not understand is kept verbatim (as
//! [`Unknown`] entities, raw tables and raw sections) so a later writer can
//! put it back unchanged. Problems that affect only part of a file are
//! returned as [`Warning`]s rather than failing the load.
//!
//! ```
//! let text = "0\nSECTION\n2\nENTITIES\n0\nLINE\n8\nWALLS\n10\n0\n20\n0\n11\n5\n21\n0\n0\nENDSEC\n0\nEOF\n";
//! let (drawing, warnings) = cad_dxf::read_str(text)?;
//! assert!(warnings.is_empty());
//! let walls = drawing.entities.iter().filter(|e| e.layer == "WALLS").count();
//! assert_eq!(walls, 1);
//! # Ok::<(), cad_dxf::DxfError>(())
//! ```

mod encoding;
mod entity;
mod error;
mod model;
mod reader;
mod table;
pub mod tokenizer;
pub mod value;
mod writer;

pub use encoding::{CodePage, FileEncoding, decode_mtext, decode_text};
pub use entity::{
    Arc, AttDef, Attrib, Circle, Dimension, DimensionKind, Ellipse, Entity, EntityKind, Insert,
    Line, LwPolyline, LwVertex, MText, Point, Polyline, Spline, Text, Unknown, Vertex,
};
pub use error::{DxfError, Warning};
pub use model::{
    Block, Color, Drawing, Handle, Header, HeaderVariable, LineWeight, Point3, RawSection, Source,
    StrayRecord, Version, Z_AXIS,
};
pub use reader::{read_bytes, read_file, read_str};
pub use table::{
    BlockRecord, DimStyle, Layer, Linetype, LinetypeElement, Named, RawTable, Table, Tables,
    TextStyle, names_equal,
};
pub use tokenizer::Pair;
pub use value::Value;
