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

mod error;
mod model;
mod reader;
pub mod tokenizer;
pub mod value;

pub use error::{DxfError, Warning};
pub use model::{
    Arc, Circle, Color, Drawing, Entity, EntityKind, Handle, Header, HeaderVariable, Layer, Line,
    LineWeight, Named, Point3, RawSection, RawTable, Table, Tables, Unknown, Version, Z_AXIS,
    names_equal,
};
pub use reader::{read_bytes, read_file, read_str};
pub use tokenizer::Pair;
pub use value::Value;
