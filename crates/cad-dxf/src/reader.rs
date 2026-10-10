//! Turns DXF text into a [`Drawing`].

use std::borrow::Cow;
use std::path::Path;

use crate::error::{DxfError, Warning};
use crate::model::{
    Arc, Circle, Color, Drawing, Entity, EntityKind, Handle, Header, HeaderVariable, Layer, Line,
    LineWeight, Point3, RawSection, RawTable, Table, Tables, Unknown, Z_AXIS,
};
use crate::tokenizer::{BINARY_SENTINEL, Pair, Token, Tokenizer};
use crate::value::{parse_f64, parse_int};

/// Reads a DXF file from disk.
pub fn read_file(path: impl AsRef<Path>) -> Result<(Drawing, Vec<Warning>), DxfError> {
    read_bytes(&std::fs::read(path)?)
}

/// Reads a DXF document from bytes.
///
/// UTF-8 is assumed; input that is not valid UTF-8 is decoded as Latin-1
/// with a warning (code page support arrives in M2).
pub fn read_bytes(bytes: &[u8]) -> Result<(Drawing, Vec<Warning>), DxfError> {
    if bytes.starts_with(BINARY_SENTINEL) {
        return Err(DxfError::BinaryNotSupported);
    }
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let mut warnings = Vec::new();
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => Cow::Borrowed(text),
        Err(_) => {
            warnings.push(Warning::new(
                None,
                "file is not valid UTF-8; decoded as Latin-1",
            ));
            Cow::Owned(bytes.iter().map(|&b| char::from(b)).collect())
        }
    };
    let (drawing, more) = read_str(&text)?;
    warnings.extend(more);
    Ok((drawing, warnings))
}

/// Reads a DXF document from text.
pub fn read_str(text: &str) -> Result<(Drawing, Vec<Warning>), DxfError> {
    let mut warnings = Vec::new();
    let mut tokens = Vec::new();
    let mut seen_section = false;
    for result in Tokenizer::new(text) {
        match result {
            Ok(token) => {
                seen_section |= token.is_marker("SECTION");
                tokens.push(token);
            }
            // Once the file has started like a DXF, keep what was read so far.
            Err(err) if seen_section => {
                warnings.push(Warning::new(
                    err.line(),
                    format!("{err}; the rest of the file was ignored"),
                ));
                break;
            }
            Err(err) => return Err(err),
        }
    }
    if !seen_section {
        return Err(DxfError::NotDxf);
    }
    let mut reader = Reader {
        tokens: &tokens,
        pos: 0,
        drawing: Drawing::default(),
        warnings,
    };
    reader.read();
    Ok((reader.drawing, reader.warnings))
}

struct Reader<'t> {
    tokens: &'t [Token],
    pos: usize,
    drawing: Drawing,
    warnings: Vec<Warning>,
}

impl<'t> Reader<'t> {
    fn warn(&mut self, line: usize, message: impl Into<String>) {
        self.warnings.push(Warning::new(Some(line), message));
    }

    /// Advances past tokens until the next `0` pair.
    fn skip_to_next_record(&mut self) {
        while self.tokens.get(self.pos).is_some_and(|t| t.pair.code != 0) {
            self.pos += 1;
        }
    }

    fn read(&mut self) {
        let mut saw_eof = false;
        while let Some(token) = self.tokens.get(self.pos) {
            self.pos += 1;
            if token.is_marker("SECTION") {
                self.read_section(token.line);
            } else if token.is_marker("EOF") {
                saw_eof = true;
                break;
            } else if token.pair.code == 999 {
                // Comments are allowed anywhere; they carry no drawing data.
            } else {
                self.warn(
                    token.line,
                    format!(
                        "unexpected group {} {:?} outside a section; skipped",
                        token.pair.code, token.pair.raw
                    ),
                );
                self.skip_to_next_record();
            }
        }
        if !saw_eof {
            let line = self.tokens.last().map_or(1, |t| t.line);
            self.warn(line, "missing EOF marker");
        } else if let Some(extra) = self.tokens.get(self.pos) {
            self.warn(extra.line, "data after EOF ignored");
        }
    }

    fn read_section(&mut self, line: usize) {
        let tokens = self.tokens;
        let name = match tokens.get(self.pos) {
            Some(t) if t.pair.code == 2 => {
                self.pos += 1;
                t.pair.raw.trim().to_owned()
            }
            _ => {
                self.warn(line, "SECTION without a name");
                String::new()
            }
        };
        let start = self.pos;
        let mut closed = false;
        while let Some(t) = tokens.get(self.pos) {
            if t.is_marker("ENDSEC") {
                closed = true;
                break;
            }
            if t.is_marker("SECTION") || t.is_marker("EOF") {
                break;
            }
            self.pos += 1;
        }
        let body = &tokens[start..self.pos];
        if closed {
            self.pos += 1;
        } else {
            self.warn(line, format!("section {name:?} has no ENDSEC"));
        }

        match name.to_ascii_uppercase().as_str() {
            "HEADER" => self.read_header(body),
            "CLASSES" => self.drawing.classes.extend(pairs_of(body)),
            "TABLES" => self.read_tables(body),
            "BLOCKS" => self.drawing.blocks.extend(pairs_of(body)),
            "ENTITIES" => self.read_entities(body),
            "OBJECTS" => self.drawing.objects.extend(pairs_of(body)),
            _ => self.drawing.other_sections.push(RawSection {
                name,
                pairs: pairs_of(body).collect(),
            }),
        }
    }

    fn read_header(&mut self, body: &[Token]) {
        for token in body {
            if token.pair.code == 9 {
                self.drawing.header.variables.push(HeaderVariable {
                    name: token.pair.raw.trim().to_owned(),
                    pairs: Vec::new(),
                });
            } else if let Some(var) = self.drawing.header.variables.last_mut() {
                var.pairs.push(token.pair.clone());
            } else {
                self.warn(
                    token.line,
                    format!(
                        "group {} before the first header variable; ignored",
                        token.pair.code
                    ),
                );
            }
        }
        check_header(&self.drawing.header, &mut self.warnings);
    }

    fn read_tables(&mut self, body: &[Token]) {
        let mut i = 0;
        while i < body.len() {
            let token = &body[i];
            i += 1;
            if !token.is_marker("TABLE") {
                self.warn(
                    token.line,
                    format!(
                        "unexpected group {} {:?} in TABLES; skipped",
                        token.pair.code, token.pair.raw
                    ),
                );
                while body.get(i).is_some_and(|t| t.pair.code != 0) {
                    i += 1;
                }
                continue;
            }
            let name = match body.get(i) {
                Some(t) if t.pair.code == 2 => {
                    i += 1;
                    t.pair.raw.trim().to_owned()
                }
                _ => {
                    self.warn(token.line, "TABLE without a name");
                    String::new()
                }
            };
            let start = i;
            while body
                .get(i)
                .is_some_and(|t| !t.is_marker("ENDTAB") && !t.is_marker("TABLE"))
            {
                i += 1;
            }
            let content = &body[start..i];
            if body.get(i).is_some_and(|t| t.is_marker("ENDTAB")) {
                i += 1;
            } else {
                self.warn(token.line, format!("table {name:?} has no ENDTAB"));
            }

            if name.eq_ignore_ascii_case("LAYER") {
                read_layer_table(content, &mut self.drawing.tables, &mut self.warnings);
            } else {
                self.drawing.tables.other.push(RawTable {
                    name,
                    pairs: pairs_of(content).collect(),
                });
            }
        }
    }

    fn read_entities(&mut self, body: &[Token]) {
        let (head, records) = split_records(body);
        if let Some(first) = head.first() {
            self.warn(
                first.line,
                format!("{} pairs before the first entity; ignored", head.len()),
            );
        }
        for record in records {
            let entity = parse_entity(record, &mut self.warnings);
            self.drawing.entities.push(entity);
        }
    }
}

fn pairs_of(tokens: &[Token]) -> impl Iterator<Item = Pair> + '_ {
    tokens.iter().map(|t| t.pair.clone())
}

/// Splits tokens into the pairs before the first `0` pair and the records
/// that each start with a `0` pair.
fn split_records(tokens: &[Token]) -> (&[Token], Vec<&[Token]>) {
    let first = tokens
        .iter()
        .position(|t| t.pair.code == 0)
        .unwrap_or(tokens.len());
    let (head, mut rest) = tokens.split_at(first);
    let mut records = Vec::new();
    while !rest.is_empty() {
        let len = rest[1..]
            .iter()
            .position(|t| t.pair.code == 0)
            .map_or(rest.len(), |p| p + 1);
        let (record, tail) = rest.split_at(len);
        records.push(record);
        rest = tail;
    }
    (head, records)
}

fn check_header(header: &Header, warnings: &mut Vec<Warning>) {
    if let Some(ver) = header.acadver()
        && header.version().is_none()
    {
        warnings.push(Warning::new(
            None,
            format!("$ACADVER {ver:?} is not a supported version; reading anyway"),
        ));
    }
}

fn read_layer_table(content: &[Token], tables: &mut Tables, warnings: &mut Vec<Warning>) {
    let (head, records) = split_records(content);
    let layers: &mut Table<Layer> = &mut tables.layers;
    layers.header.extend(pairs_of(head));
    for record in records {
        let line = record[0].line;
        let raw = || pairs_of(record).collect::<Vec<_>>();
        if !record[0].is_marker("LAYER") {
            warnings.push(Warning::new(
                Some(line),
                format!(
                    "unexpected {:?} entry in LAYER table; kept as is",
                    record[0].pair.raw
                ),
            ));
            layers.unparsed.push(raw());
            continue;
        }
        match parse_layer(&record[1..]) {
            Ok(layer) if layers.contains(&layer.name) => {
                warnings.push(Warning::new(
                    Some(line),
                    format!("duplicate layer {:?}; kept as is", layer.name),
                ));
                layers.unparsed.push(raw());
            }
            Ok(layer) => layers.entries.push(layer),
            Err(message) => {
                warnings.push(Warning::new(
                    Some(line),
                    format!("LAYER entry kept as is: {message}"),
                ));
                layers.unparsed.push(raw());
            }
        }
    }
}

/// Walks an object's pairs: pairs inside `102 {...}` groups and extended data
/// (codes 1000+) go straight to `extra`; every other pair is offered to
/// `field`, and the ones it does not take (`Ok(false)`) go to `extra` too.
/// Returns the first error `field` reported, after visiting every pair.
fn walk<'p>(
    pairs: impl IntoIterator<Item = &'p Pair>,
    extra: &mut Vec<Pair>,
    mut field: impl FnMut(&'p Pair) -> Result<bool, String>,
) -> Result<(), String> {
    let mut depth = 0usize;
    let mut xdata = false;
    let mut first_error = None;
    for pair in pairs {
        xdata |= pair.code >= 1000;
        if pair.code == 102 {
            let value = pair.raw.trim();
            if value.starts_with('{') {
                depth += 1;
            } else if value == "}" {
                depth = depth.saturating_sub(1);
            }
        }
        if xdata || depth > 0 || pair.code == 102 {
            extra.push(pair.clone());
            continue;
        }
        match field(pair) {
            Ok(true) => {}
            Ok(false) => extra.push(pair.clone()),
            Err(message) => {
                first_error.get_or_insert(message);
                extra.push(pair.clone());
            }
        }
    }
    first_error.map_or(Ok(()), Err)
}

fn invalid(pair: &Pair) -> String {
    format!("invalid value {:?} for group code {}", pair.raw, pair.code)
}

fn int_of(pair: &Pair) -> Result<i64, String> {
    parse_int(&pair.raw).ok_or_else(|| invalid(pair))
}

fn float_of(pair: &Pair) -> Result<f64, String> {
    parse_f64(&pair.raw).ok_or_else(|| invalid(pair))
}

fn handle_of(pair: &Pair) -> Result<Handle, String> {
    Handle::parse(&pair.raw).ok_or_else(|| invalid(pair))
}

fn flag_of(pair: &Pair) -> Result<bool, String> {
    match int_of(pair)? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(invalid(pair)),
    }
}

fn true_color_of(pair: &Pair) -> Result<Color, String> {
    Color::from_true_color(int_of(pair)?).ok_or_else(|| invalid(pair))
}

fn lineweight_of(pair: &Pair) -> Result<LineWeight, String> {
    LineWeight::from_code(int_of(pair)?).ok_or_else(|| invalid(pair))
}

fn parse_layer(pairs: &[Token]) -> Result<Layer, String> {
    let mut layer = Layer::new("");
    let mut name = None;
    let mut extra = Vec::new();
    walk(pairs.iter().map(|t| &t.pair), &mut extra, |p| {
        match p.code {
            2 if name.is_none() => name = Some(p.raw.trim().to_owned()),
            5 if layer.handle.is_none() => layer.handle = Some(handle_of(p)?),
            330 if layer.owner.is_none() => layer.owner = Some(handle_of(p)?),
            70 => layer.flags = i16::try_from(int_of(p)?).map_err(|_| invalid(p))?,
            62 => {
                let value = int_of(p)?;
                let color = Color::from_aci(value.abs()).ok_or_else(|| invalid(p))?;
                layer.on = value >= 0;
                if !matches!(layer.color, Color::Rgb(..)) {
                    layer.color = color;
                }
            }
            420 => layer.color = true_color_of(p)?,
            6 => layer.linetype = p.raw.trim().to_owned(),
            370 => layer.lineweight = lineweight_of(p)?,
            290 => layer.plot = flag_of(p)?,
            // Subclass markers are regenerated by the writer.
            100 => {}
            _ => return Ok(false),
        }
        Ok(true)
    })?;
    match name {
        Some(name) if !name.is_empty() => {
            layer.name = name;
            layer.extra = extra;
            Ok(layer)
        }
        _ => Err("missing layer name (group code 2)".into()),
    }
}

/// Applies a pair to the entity's common fields. `Ok(false)` if the pair is
/// not a common field.
fn common_field(entity: &mut Entity, pair: &Pair) -> Result<bool, String> {
    match pair.code {
        5 if entity.handle.is_none() => entity.handle = Some(handle_of(pair)?),
        330 if entity.owner.is_none() => entity.owner = Some(handle_of(pair)?),
        8 => entity.layer = pair.raw.trim().to_owned(),
        6 => entity.linetype = pair.raw.trim().to_owned(),
        62 => {
            let color = Color::from_aci(int_of(pair)?).ok_or_else(|| invalid(pair))?;
            // A true colour (420) takes precedence over the index colour.
            if !matches!(entity.color, Color::Rgb(..)) {
                entity.color = color;
            }
        }
        420 => entity.color = true_color_of(pair)?,
        370 => entity.lineweight = lineweight_of(pair)?,
        60 => entity.visible = !flag_of(pair)?,
        67 => entity.paper_space = flag_of(pair)?,
        // Subclass markers are regenerated by the writer.
        100 => {}
        _ => return Ok(false),
    }
    Ok(true)
}

/// The geometry group codes each supported entity type reads.
fn geometry_codes(type_name: &str) -> Option<&'static [i16]> {
    Some(match type_name {
        "LINE" => &[10, 20, 30, 11, 21, 31, 39, 210, 220, 230],
        "CIRCLE" => &[10, 20, 30, 40, 39, 210, 220, 230],
        "ARC" => &[10, 20, 30, 40, 50, 51, 39, 210, 220, 230],
        _ => return None,
    })
}

/// Parses one entity record (starting with its `0 <type>` pair).
///
/// Never fails: anything that cannot be represented becomes
/// [`EntityKind::Unknown`] with all its pairs, plus a warning.
fn parse_entity(record: &[Token], warnings: &mut Vec<Warning>) -> Entity {
    let type_name = record[0].pair.raw.trim().to_owned();
    let line = record[0].line;
    let pairs: Vec<Pair> = pairs_of(&record[1..]).collect();
    let mut entity = Entity::new(EntityKind::Unknown(Unknown {
        type_name: type_name.clone(),
        pairs: Vec::new(),
    }));
    let mut extra = Vec::new();

    let Some(codes) = geometry_codes(&type_name) else {
        // Best-effort common fields; the raw pairs stay authoritative.
        let _ = walk(&pairs, &mut extra, |p| common_field(&mut entity, p));
        entity.kind = EntityKind::Unknown(Unknown { type_name, pairs });
        return entity;
    };

    let mut geometry = Geometry::default();
    let walked = walk(&pairs, &mut extra, |p| {
        if common_field(&mut entity, p)? {
            return Ok(true);
        }
        if codes.contains(&p.code) && geometry.get(p.code).is_none() {
            geometry.values.push((p.code, float_of(p)?));
            return Ok(true);
        }
        Ok(false)
    });
    match walked.and_then(|()| geometry.build(&type_name)) {
        Ok(kind) => {
            entity.kind = kind;
            entity.extra = extra;
        }
        Err(message) => {
            warnings.push(Warning::new(
                Some(line),
                format!("{type_name} kept as unknown entity: {message}"),
            ));
            entity.kind = EntityKind::Unknown(Unknown { type_name, pairs });
        }
    }
    entity
}

/// Geometry values collected from an entity, first occurrence of each code.
#[derive(Default)]
struct Geometry {
    values: Vec<(i16, f64)>,
}

impl Geometry {
    fn get(&self, code: i16) -> Option<f64> {
        self.values
            .iter()
            .find(|(c, _)| *c == code)
            .map(|&(_, v)| v)
    }

    fn require(&self, code: i16, what: &str) -> Result<f64, String> {
        self.get(code)
            .ok_or_else(|| format!("missing {what} (group code {code})"))
    }

    fn point(&self, x: i16, what: &str) -> Result<Point3, String> {
        Ok(Point3::new(
            self.require(x, what)?,
            self.require(x + 10, what)?,
            self.get(x + 20).unwrap_or(0.0),
        ))
    }

    fn radius(&self) -> Result<f64, String> {
        let radius = self.require(40, "radius")?;
        if radius < 0.0 {
            return Err(format!("negative radius {radius}"));
        }
        Ok(radius)
    }

    fn build(&self, type_name: &str) -> Result<EntityKind, String> {
        let thickness = self.get(39).unwrap_or(0.0);
        let extrusion = Point3::new(
            self.get(210).unwrap_or(Z_AXIS.x),
            self.get(220).unwrap_or(Z_AXIS.y),
            self.get(230).unwrap_or(Z_AXIS.z),
        );
        Ok(match type_name {
            "LINE" => EntityKind::Line(Line {
                start: self.point(10, "start point")?,
                end: self.point(11, "end point")?,
                thickness,
                extrusion,
            }),
            "CIRCLE" => EntityKind::Circle(Circle {
                center: self.point(10, "center")?,
                radius: self.radius()?,
                thickness,
                extrusion,
            }),
            "ARC" => EntityKind::Arc(Arc {
                center: self.point(10, "center")?,
                radius: self.radius()?,
                start_angle: self.require(50, "start angle")?,
                end_angle: self.require(51, "end angle")?,
                thickness,
                extrusion,
            }),
            other => return Err(format!("no geometry parser for {other}")),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a DXF document from `(code, value)` pairs.
    fn dxf(pairs: &[(i16, &str)]) -> String {
        pairs.iter().map(|(c, v)| format!("{c}\n{v}\n")).collect()
    }

    /// Wraps entity pairs in an ENTITIES section.
    fn entities_doc(body: &[(i16, &str)]) -> String {
        let mut pairs = vec![(0, "SECTION"), (2, "ENTITIES")];
        pairs.extend_from_slice(body);
        pairs.extend_from_slice(&[(0, "ENDSEC"), (0, "EOF")]);
        dxf(&pairs)
    }

    fn read_ok(text: &str) -> (Drawing, Vec<Warning>) {
        read_str(text).expect("document should load")
    }

    fn only_entity(text: &str) -> (Entity, Vec<Warning>) {
        let (drawing, warnings) = read_ok(text);
        assert_eq!(drawing.entities.len(), 1);
        (drawing.entities.into_iter().next().unwrap(), warnings)
    }

    #[test]
    fn line_with_common_fields() {
        let (e, warnings) = only_entity(&entities_doc(&[
            (0, "LINE"),
            (5, "2F"),
            (330, "1F"),
            (100, "AcDbEntity"),
            (8, "WALLS"),
            (6, "DASHED"),
            (62, "1"),
            (370, "25"),
            (100, "AcDbLine"),
            (10, "1.5"),
            (20, "-2"),
            (30, "0"),
            (11, "10"),
            (21, "20"),
            (31, "3"),
        ]));
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(e.handle, Some(Handle(0x2F)));
        assert_eq!(e.owner, Some(Handle(0x1F)));
        assert_eq!(e.layer, "WALLS");
        assert_eq!(e.linetype, "DASHED");
        assert_eq!(e.color, Color::Index(1));
        assert_eq!(e.lineweight, LineWeight::Hundredths(25));
        assert!(e.visible);
        assert!(e.extra.is_empty());
        let EntityKind::Line(line) = e.kind else {
            panic!("expected a line, got {:?}", e.kind)
        };
        assert_eq!(line.start, Point3::new(1.5, -2.0, 0.0));
        assert_eq!(line.end, Point3::new(10.0, 20.0, 3.0));
        assert_eq!(line.thickness, 0.0);
        assert_eq!(line.extrusion, Z_AXIS);
    }

    #[test]
    fn line_defaults_without_z_or_layer() {
        let (e, _) = only_entity(&entities_doc(&[
            (0, "LINE"),
            (10, "0"),
            (20, "0"),
            (11, "1"),
            (21, "1"),
        ]));
        assert_eq!(e.layer, "0");
        assert_eq!(e.color, Color::ByLayer);
        let EntityKind::Line(line) = e.kind else {
            panic!()
        };
        assert_eq!(line.end, Point3::new(1.0, 1.0, 0.0));
    }

    #[test]
    fn line_missing_end_point_becomes_unknown_with_warning() {
        let (e, warnings) = only_entity(&entities_doc(&[
            (0, "LINE"),
            (8, "A"),
            (10, "0"),
            (20, "0"),
        ]));
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("end point"));
        assert_eq!(warnings[0].line, Some(5));
        let EntityKind::Unknown(u) = e.kind else {
            panic!()
        };
        assert_eq!(u.type_name, "LINE");
        assert_eq!(u.pairs.len(), 3);
        assert_eq!(e.layer, "A");
    }

    #[test]
    fn bad_number_keeps_raw_pairs() {
        let (e, warnings) = only_entity(&entities_doc(&[
            (0, "LINE"),
            (10, "abc"),
            (20, "0"),
            (11, "1"),
            (21, "1"),
        ]));
        assert_eq!(warnings.len(), 1);
        let EntityKind::Unknown(u) = e.kind else {
            panic!()
        };
        assert_eq!(u.pairs[0], Pair::new(10, "abc"));
    }

    #[test]
    fn circle() {
        let (e, warnings) = only_entity(&entities_doc(&[
            (0, "CIRCLE"),
            (8, "0"),
            (10, "5"),
            (20, "6"),
            (40, "2.5"),
            (39, "1"),
        ]));
        assert!(warnings.is_empty());
        let EntityKind::Circle(c) = e.kind else {
            panic!()
        };
        assert_eq!(c.center, Point3::new(5.0, 6.0, 0.0));
        assert_eq!(c.radius, 2.5);
        assert_eq!(c.thickness, 1.0);
    }

    #[test]
    fn circle_without_radius_or_with_negative_radius_is_unknown() {
        for radius in [None, Some("-1")] {
            let mut body = vec![(0, "CIRCLE"), (10, "0"), (20, "0")];
            if let Some(r) = radius {
                body.push((40, r));
            }
            let (e, warnings) = only_entity(&entities_doc(&body));
            assert!(matches!(e.kind, EntityKind::Unknown(_)));
            assert_eq!(warnings.len(), 1);
        }
    }

    #[test]
    fn arc_including_one_crossing_zero_degrees() {
        let (e, warnings) = only_entity(&entities_doc(&[
            (0, "ARC"),
            (10, "1"),
            (20, "2"),
            (40, "3"),
            (50, "350"),
            (51, "10"),
            (210, "0"),
            (220, "0"),
            (230, "-1"),
        ]));
        assert!(warnings.is_empty());
        let EntityKind::Arc(a) = e.kind else { panic!() };
        assert_eq!(a.center, Point3::new(1.0, 2.0, 0.0));
        assert_eq!(a.radius, 3.0);
        assert_eq!(a.start_angle, 350.0);
        assert_eq!(a.end_angle, 10.0);
        assert_eq!(a.extrusion, Point3::new(0.0, 0.0, -1.0));
    }

    #[test]
    fn arc_missing_angle_is_unknown() {
        let (e, warnings) = only_entity(&entities_doc(&[
            (0, "ARC"),
            (10, "1"),
            (20, "2"),
            (40, "3"),
            (50, "0"),
        ]));
        assert!(matches!(e.kind, EntityKind::Unknown(_)));
        assert!(warnings[0].message.contains("end angle"));
    }

    #[test]
    fn true_color_wins_over_index_and_xdata_is_extra() {
        let (e, warnings) = only_entity(&entities_doc(&[
            (0, "CIRCLE"),
            (102, "{ACAD_REACTORS"),
            (330, "AB"),
            (102, "}"),
            (330, "1F"),
            (62, "5"),
            (420, "16711680"),
            (60, "1"),
            (10, "0"),
            (20, "0"),
            (40, "1"),
            (999, "a comment"),
            (1001, "MYAPP"),
            (1000, "hello"),
            (1040, "2.0"),
        ]));
        assert!(warnings.is_empty());
        assert_eq!(e.owner, Some(Handle(0x1F)));
        assert_eq!(e.color, Color::Rgb(255, 0, 0));
        assert!(!e.visible);
        let codes: Vec<i16> = e.extra.iter().map(|p| p.code).collect();
        assert_eq!(codes, vec![102, 330, 102, 999, 1001, 1000, 1040]);
    }

    #[test]
    fn invalid_color_makes_entity_unknown() {
        let (e, warnings) = only_entity(&entities_doc(&[
            (0, "CIRCLE"),
            (62, "300"),
            (10, "0"),
            (20, "0"),
            (40, "1"),
        ]));
        assert!(matches!(e.kind, EntityKind::Unknown(_)));
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn unknown_entity_type_is_preserved_without_warning() {
        let body = [
            (0, "LWPOLYLINE"),
            (8, "PLAN"),
            (90, "2"),
            (10, "0"),
            (20, "0"),
            (10, "1"),
            (20, "1"),
        ];
        let (e, warnings) = only_entity(&entities_doc(&body));
        assert!(warnings.is_empty());
        assert_eq!(e.layer, "PLAN");
        let EntityKind::Unknown(u) = &e.kind else {
            panic!()
        };
        assert_eq!(u.type_name, "LWPOLYLINE");
        let expected: Vec<Pair> = body[1..].iter().map(|&(c, v)| Pair::new(c, v)).collect();
        assert_eq!(u.pairs, expected);
        assert!(e.extra.is_empty());
        assert_eq!(e.kind.type_name(), "LWPOLYLINE");
    }

    #[test]
    fn header_variables_and_layers() {
        let text = dxf(&[
            (0, "SECTION"),
            (2, "HEADER"),
            (9, "$ACADVER"),
            (1, "AC1015"),
            (9, "$INSUNITS"),
            (70, "6"),
            (9, "$CLAYER"),
            (8, "Walls"),
            (0, "ENDSEC"),
            (0, "SECTION"),
            (2, "TABLES"),
            (0, "TABLE"),
            (2, "LTYPE"),
            (70, "1"),
            (0, "LTYPE"),
            (2, "CONTINUOUS"),
            (0, "ENDTAB"),
            (0, "TABLE"),
            (2, "LAYER"),
            (5, "2"),
            (70, "3"),
            (0, "LAYER"),
            (5, "10"),
            (2, "0"),
            (70, "0"),
            (62, "7"),
            (6, "CONTINUOUS"),
            (0, "LAYER"),
            (2, "Walls"),
            (70, "5"),
            (62, "-1"),
            (6, "DASHED"),
            (370, "50"),
            (290, "0"),
            (0, "LAYER"),
            (2, "WALLS"),
            (62, "2"),
            (0, "ENDTAB"),
            (0, "ENDSEC"),
            (0, "EOF"),
        ]);
        let (d, warnings) = read_ok(&text);
        assert_eq!(d.header.version(), Some(crate::Version::R2000));
        assert_eq!(d.header.insunits(), Some(6));
        assert_eq!(d.header.current_layer(), Some("Walls"));

        let layers = &d.tables.layers;
        assert_eq!(layers.len(), 2);
        assert_eq!(layers.header.len(), 2);
        let zero = layers.get("0").unwrap();
        assert_eq!(zero.handle, Some(Handle(0x10)));
        assert!(zero.on);
        let walls = layers.get("walls").unwrap();
        assert_eq!(walls.name, "Walls");
        assert!(!walls.on);
        assert_eq!(walls.color, Color::Index(1));
        assert!(walls.frozen());
        assert!(walls.locked());
        assert!(!walls.plot);
        assert_eq!(walls.linetype, "DASHED");
        assert_eq!(walls.lineweight, LineWeight::Hundredths(50));

        // The case-insensitive duplicate is kept verbatim with a warning.
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].message.contains("duplicate"));
        assert_eq!(layers.unparsed.len(), 1);

        assert_eq!(d.tables.other.len(), 1);
        assert_eq!(d.tables.other[0].name, "LTYPE");
        assert_eq!(d.tables.other[0].pairs.len(), 3);
    }

    #[test]
    fn layer_without_name_is_kept_unparsed() {
        let text = dxf(&[
            (0, "SECTION"),
            (2, "TABLES"),
            (0, "TABLE"),
            (2, "LAYER"),
            (0, "LAYER"),
            (70, "0"),
            (0, "ENDTAB"),
            (0, "ENDSEC"),
            (0, "EOF"),
        ]);
        let (d, warnings) = read_ok(&text);
        assert!(d.tables.layers.is_empty());
        assert_eq!(d.tables.layers.unparsed.len(), 1);
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn unknown_and_raw_sections_are_kept() {
        let text = dxf(&[
            (0, "SECTION"),
            (2, "CLASSES"),
            (0, "CLASS"),
            (1, "X"),
            (0, "ENDSEC"),
            (0, "SECTION"),
            (2, "BLOCKS"),
            (0, "BLOCK"),
            (2, "B"),
            (0, "ENDBLK"),
            (0, "ENDSEC"),
            (0, "SECTION"),
            (2, "THUMBNAILIMAGE"),
            (90, "0"),
            (0, "ENDSEC"),
            (0, "SECTION"),
            (2, "OBJECTS"),
            (0, "DICTIONARY"),
            (0, "ENDSEC"),
            (0, "EOF"),
        ]);
        let (d, warnings) = read_ok(&text);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(d.classes.len(), 2);
        assert_eq!(d.blocks.len(), 3);
        assert_eq!(d.objects.len(), 1);
        assert_eq!(d.other_sections.len(), 1);
        assert_eq!(d.other_sections[0].name, "THUMBNAILIMAGE");
        assert_eq!(d.other_sections[0].pairs, vec![Pair::new(90, "0")]);
    }

    #[test]
    fn missing_eof_and_endsec_are_warnings() {
        let text = dxf(&[
            (0, "SECTION"),
            (2, "ENTITIES"),
            (0, "CIRCLE"),
            (10, "0"),
            (20, "0"),
            (40, "1"),
        ]);
        let (d, warnings) = read_ok(&text);
        assert_eq!(d.entities.len(), 1);
        assert_eq!(warnings.len(), 2, "{warnings:?}");
    }

    #[test]
    fn truncated_file_keeps_what_was_read() {
        let mut text = entities_doc(&[(0, "CIRCLE"), (10, "0"), (20, "0"), (40, "1")]);
        text = text.replace("0\nENDSEC\n0\nEOF\n", "0\nENDSEC\n0");
        let (d, warnings) = read_ok(&text);
        assert_eq!(d.entities.len(), 1);
        assert!(
            warnings
                .iter()
                .any(|w| w.message.contains("rest of the file"))
        );
    }

    #[test]
    fn stray_pairs_outside_sections_are_skipped() {
        let text = dxf(&[
            (999, "comments are fine"),
            (5, "stray"),
            (0, "SECTION"),
            (2, "ENTITIES"),
            (0, "ENDSEC"),
            (0, "EOF"),
            (0, "JUNK"),
        ]);
        let (_, warnings) = read_ok(&text);
        assert_eq!(warnings.len(), 2, "{warnings:?}");
    }

    #[test]
    fn non_dxf_input_is_an_error() {
        assert!(matches!(read_str(""), Err(DxfError::NotDxf)));
        assert!(matches!(
            read_str("0\nLINE\n0\nEOF\n"),
            Err(DxfError::NotDxf)
        ));
        assert!(read_str("hello world\n").is_err());
    }

    #[test]
    fn binary_dxf_is_rejected_clearly() {
        let mut bytes = b"AutoCAD Binary DXF\r\n\x1a\0".to_vec();
        bytes.extend_from_slice(&[0, 0, 1, 2, 3]);
        assert!(matches!(
            read_bytes(&bytes),
            Err(DxfError::BinaryNotSupported)
        ));
    }

    #[test]
    fn latin1_input_loads_with_warning() {
        let mut bytes = entities_doc(&[(0, "LINE"), (8, "W\u{e4}nde")]).into_bytes();
        // Replace the UTF-8 "ä" (C3 A4) with Latin-1 (E4).
        let at = bytes.windows(2).position(|w| w == [0xC3, 0xA4]).unwrap();
        bytes.splice(at..at + 2, [0xE4]);
        let (d, warnings) = read_bytes(&bytes).unwrap();
        assert_eq!(d.entities[0].layer, "Wände");
        assert!(warnings.iter().any(|w| w.message.contains("Latin-1")));
    }

    #[test]
    fn bom_is_ignored() {
        let mut bytes = b"\xEF\xBB\xBF".to_vec();
        bytes.extend_from_slice(entities_doc(&[]).as_bytes());
        let (_, warnings) = read_bytes(&bytes).unwrap();
        assert!(warnings.is_empty());
    }
}
