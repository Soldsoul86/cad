//! Turns DXF text into a [`Drawing`].
//!
//! The reader streams over the tokenizer: values stay borrowed from the
//! input and are copied only when the model keeps them.

mod entity;
mod fields;
mod table;
#[cfg(test)]
mod tests;

use std::borrow::Cow;
use std::path::Path;

use crate::entity::{Entity, EntityKind};
use crate::error::{DxfError, Warning};
use crate::model::{Block, Drawing, HeaderVariable, RawSection};
use crate::table::{Named, RawTable, Table};
use crate::tokenizer::{BINARY_SENTINEL, Pair, Token, Tokenizer};

use entity::parse_entity;

/// Reads a DXF file from disk.
pub fn read_file(path: impl AsRef<Path>) -> Result<(Drawing, Vec<Warning>), DxfError> {
    read_bytes(&std::fs::read(path)?)
}

/// Reads a DXF document from bytes.
///
/// UTF-8 is assumed; input that is not valid UTF-8 is decoded as Latin-1
/// with a warning (code page support arrives later in M2).
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
    let mut reader = Reader {
        tokens: Tokenizer::new(text),
        peeked: None,
        seen_section: false,
        fatal: None,
        last_line: 1,
        body: Vec::new(),
        drawing: Drawing::default(),
        warnings: Vec::new(),
    };
    reader.read()?;
    Ok((reader.drawing, reader.warnings))
}

/// Markers that end a section's content.
const SECTION_END: [&str; 3] = ["ENDSEC", "SECTION", "EOF"];

struct Reader<'a> {
    tokens: Tokenizer<'a>,
    peeked: Option<Token<'a>>,
    seen_section: bool,
    /// A tokenizer error before the file looked like a DXF.
    fatal: Option<DxfError>,
    last_line: usize,
    /// Reused buffer for one record's pairs.
    body: Vec<Token<'a>>,
    drawing: Drawing,
    warnings: Vec<Warning>,
}

impl<'a> Reader<'a> {
    fn warn(&mut self, line: usize, message: impl Into<String>) {
        self.warnings.push(Warning::new(Some(line), message));
    }

    fn peek(&mut self) -> Option<&Token<'a>> {
        if self.peeked.is_none() {
            match self.tokens.next() {
                Some(Ok(token)) => {
                    self.seen_section |= token.is_marker("SECTION");
                    self.last_line = token.line;
                    self.peeked = Some(token);
                }
                // Once the file has started like a DXF, keep what was read.
                Some(Err(err)) if self.seen_section => {
                    let line = err.line().unwrap_or(self.last_line);
                    self.warn(line, format!("{err}; the rest of the file was ignored"));
                }
                Some(Err(err)) => self.fatal = Some(err),
                None => {}
            }
        }
        self.peeked.as_ref()
    }

    fn next(&mut self) -> Option<Token<'a>> {
        self.peek();
        self.peeked.take()
    }

    /// The value of the next pair if it is a `0` pair.
    fn peek_marker(&mut self) -> Option<&'a str> {
        match self.peek() {
            Some(t) if t.code == 0 => Some(t.raw.trim()),
            _ => None,
        }
    }

    /// True at the end of input or at a `0` pair named in `markers`.
    fn at(&mut self, markers: &[&str]) -> bool {
        match self.peek() {
            None => true,
            Some(t) => t.code == 0 && markers.contains(&t.raw.trim()),
        }
    }

    /// Reads the pairs up to the next `0` pair into `body`.
    fn read_body(&mut self, body: &mut Vec<Token<'a>>) {
        body.clear();
        while self.peek().is_some_and(|t| t.code != 0) {
            if let Some(t) = self.next() {
                body.push(t);
            }
        }
    }

    fn skip_body(&mut self) {
        while self.peek().is_some_and(|t| t.code != 0) {
            self.next();
        }
    }

    /// Reads pairs until a section end, as owned pairs.
    fn read_raw(&mut self) -> Vec<Pair> {
        let mut pairs = Vec::new();
        while !self.at(&SECTION_END) {
            if let Some(t) = self.next() {
                pairs.push(t.to_pair());
            }
        }
        pairs
    }

    fn read(&mut self) -> Result<(), DxfError> {
        let mut saw_eof = false;
        while let Some(token) = self.next() {
            if token.is_marker("SECTION") {
                self.read_section(token.line);
            } else if token.is_marker("EOF") {
                saw_eof = true;
                break;
            } else if token.code == 999 {
                // Comments are allowed anywhere; they carry no drawing data.
            } else {
                self.warn(
                    token.line,
                    format!(
                        "unexpected group {} {:?} outside a section; skipped",
                        token.code, token.raw
                    ),
                );
                self.skip_body();
            }
        }
        if let Some(err) = self.fatal.take() {
            return Err(err);
        }
        if !self.seen_section {
            return Err(DxfError::NotDxf);
        }
        if !saw_eof {
            let line = self.last_line;
            self.warn(line, "missing EOF marker");
        } else if let Some(extra) = self.peek() {
            let line = extra.line;
            self.warn(line, "data after EOF ignored");
        }
        Ok(())
    }

    fn read_section(&mut self, line: usize) {
        let name = match self.peek() {
            Some(t) if t.code == 2 => {
                let name = t.raw.trim();
                self.next();
                name
            }
            _ => {
                self.warn(line, "SECTION without a name");
                ""
            }
        };
        match name.to_ascii_uppercase().as_str() {
            "HEADER" => self.read_header(),
            "CLASSES" => {
                let pairs = self.read_raw();
                self.drawing.classes.extend(pairs);
            }
            "TABLES" => self.read_tables(),
            "BLOCKS" => self.read_blocks(),
            "ENTITIES" => {
                let mut entities = std::mem::take(&mut self.drawing.entities);
                self.read_entities(&mut entities, &SECTION_END);
                self.drawing.entities = entities;
            }
            "OBJECTS" => {
                let pairs = self.read_raw();
                self.drawing.objects.extend(pairs);
            }
            _ => {
                let pairs = self.read_raw();
                self.drawing.other_sections.push(RawSection {
                    name: name.to_owned(),
                    pairs,
                });
            }
        }
        if self.peek_marker() == Some("ENDSEC") {
            self.next();
        } else {
            self.warn(line, format!("section {name:?} has no ENDSEC"));
        }
    }

    fn read_header(&mut self) {
        while !self.at(&SECTION_END) {
            let Some(token) = self.next() else { break };
            if token.code == 9 {
                self.drawing.header.variables.push(HeaderVariable {
                    name: token.raw.trim().to_owned(),
                    pairs: Vec::new(),
                });
            } else if let Some(var) = self.drawing.header.variables.last_mut() {
                var.pairs.push(token.to_pair());
            } else {
                self.warn(
                    token.line,
                    format!(
                        "group {} before the first header variable; ignored",
                        token.code
                    ),
                );
            }
        }
        if let Some(ver) = self.drawing.header.acadver()
            && self.drawing.header.version().is_none()
        {
            let message = format!("$ACADVER {ver:?} is not a supported version; reading anyway");
            self.warnings.push(Warning::new(None, message));
        }
    }

    fn read_tables(&mut self) {
        while !self.at(&SECTION_END) {
            let Some(token) = self.next() else { break };
            if token.is_marker("TABLE") {
                self.read_table(token.line);
            } else {
                self.warn(
                    token.line,
                    format!(
                        "unexpected group {} {:?} in TABLES; skipped",
                        token.code, token.raw
                    ),
                );
                self.skip_body();
            }
        }
    }

    fn read_table(&mut self, line: usize) {
        let name = match self.peek() {
            Some(t) if t.code == 2 => {
                let name = t.raw.trim();
                self.next();
                name
            }
            _ => {
                self.warn(line, "TABLE without a name");
                ""
            }
        };
        let mut body = std::mem::take(&mut self.body);
        self.read_body(&mut body);
        let header: Vec<Pair> = body.iter().map(Token::to_pair).collect();
        let kind = name.to_ascii_uppercase();
        let mut raw = RawTable {
            name: name.to_owned(),
            pairs: header.clone(),
        };
        let tables = &mut self.drawing.tables;
        match kind.as_str() {
            "LAYER" => tables.layers.header.extend(header),
            "LTYPE" => tables.linetypes.header.extend(header),
            "STYLE" => tables.text_styles.header.extend(header),
            "DIMSTYLE" => tables.dim_styles.header.extend(header),
            "BLOCK_RECORD" => tables.block_records.header.extend(header),
            _ => {}
        }
        loop {
            match self.peek_marker() {
                Some("ENDTAB") => {
                    self.next();
                    break;
                }
                Some("TABLE" | "ENDSEC" | "SECTION" | "EOF") | None => {
                    self.warn(line, format!("table {name:?} has no ENDTAB"));
                    break;
                }
                Some(_) => {}
            }
            let Some(head) = self.next() else { break };
            self.read_body(&mut body);
            let tables = &mut self.drawing.tables;
            let warnings = &mut self.warnings;
            match kind.as_str() {
                "LAYER" => add_entry(
                    &mut tables.layers,
                    "LAYER",
                    &head,
                    &body,
                    table::layer,
                    warnings,
                ),
                "LTYPE" => add_entry(
                    &mut tables.linetypes,
                    "LTYPE",
                    &head,
                    &body,
                    table::linetype,
                    warnings,
                ),
                "STYLE" => add_entry(
                    &mut tables.text_styles,
                    "STYLE",
                    &head,
                    &body,
                    table::text_style,
                    warnings,
                ),
                "DIMSTYLE" => add_entry(
                    &mut tables.dim_styles,
                    "DIMSTYLE",
                    &head,
                    &body,
                    table::dim_style,
                    warnings,
                ),
                "BLOCK_RECORD" => add_entry(
                    &mut tables.block_records,
                    "BLOCK_RECORD",
                    &head,
                    &body,
                    table::block_record,
                    warnings,
                ),
                _ => {
                    raw.pairs.push(head.to_pair());
                    raw.pairs.extend(body.iter().map(Token::to_pair));
                }
            }
        }
        self.body = body;
        if !matches!(
            kind.as_str(),
            "LAYER" | "LTYPE" | "STYLE" | "DIMSTYLE" | "BLOCK_RECORD"
        ) {
            self.drawing.tables.other.push(raw);
        }
    }

    fn read_blocks(&mut self) {
        while !self.at(&SECTION_END) {
            let Some(token) = self.next() else { break };
            if token.is_marker("BLOCK") {
                self.read_block(&token);
            } else {
                self.warn(
                    token.line,
                    format!(
                        "unexpected group {} {:?} in BLOCKS; skipped",
                        token.code, token.raw
                    ),
                );
                self.skip_body();
            }
        }
    }

    fn read_block(&mut self, head: &Token<'a>) {
        const BLOCK_END: [&str; 5] = ["ENDBLK", "BLOCK", "ENDSEC", "SECTION", "EOF"];
        let mut body = std::mem::take(&mut self.body);
        self.read_body(&mut body);
        let parsed = table::block_header(&body);
        let problem = match &parsed {
            Err(message) => Some(format!("BLOCK kept as is: {message}")),
            Ok(block) if block_exists(&self.drawing.blocks, block) => {
                Some(format!("duplicate block {:?}; kept as is", block.name))
            }
            Ok(_) => None,
        };
        if let Some(message) = problem {
            // Keep the whole definition verbatim, through its ENDBLK record.
            self.warn(head.line, message);
            let mut raw = vec![head.to_pair()];
            raw.extend(body.iter().map(Token::to_pair));
            self.body = body;
            while !self.at(&BLOCK_END) {
                if let Some(t) = self.next() {
                    raw.push(t.to_pair());
                }
            }
            if self.peek_marker() == Some("ENDBLK") {
                while let Some(t) = self.next() {
                    raw.push(t.to_pair());
                    if self.peek().is_none_or(|t| t.code == 0) {
                        break;
                    }
                }
            } else {
                self.warn(head.line, "block has no ENDBLK");
            }
            self.drawing.blocks.unparsed.push(raw);
            return;
        }
        self.body = body;
        let Ok(mut block) = parsed else { return };

        self.read_entities(&mut block.entities, &BLOCK_END);
        if self.peek_marker() == Some("ENDBLK") {
            self.next();
            let mut body = std::mem::take(&mut self.body);
            self.read_body(&mut body);
            block.end = body.iter().map(Token::to_pair).collect();
            self.body = body;
        } else {
            self.warn(head.line, "block has no ENDBLK");
        }
        self.drawing.blocks.entries.push(block);
    }

    /// Reads entity records until a `0` marker in `stop` (not consumed),
    /// grouping `POLYLINE`/`VERTEX`/`SEQEND` and `INSERT`/`ATTRIB`/`SEQEND`.
    fn read_entities(&mut self, out: &mut Vec<Entity>, stop: &[&str]) {
        loop {
            match self.peek() {
                None => return,
                Some(t) if t.code == 0 => {
                    if stop.contains(&t.raw.trim()) {
                        return;
                    }
                }
                Some(t) => {
                    let line = t.line;
                    self.warn(line, "pairs before the first entity; ignored");
                    self.skip_body();
                    continue;
                }
            }
            let fallback = self.last_line;
            let line = self.peek().map_or(fallback, |t| t.line);
            if let Some(entity) = self.read_entity() {
                // The parent is the last entity that is not itself an orphan.
                let parent = out
                    .iter()
                    .rev()
                    .find(|e| !matches!(e.kind.type_name(), "VERTEX" | "ATTRIB" | "SEQEND"));
                if let Some(message) = orphan_message(&entity, parent) {
                    self.warn(line, message);
                }
                out.push(entity);
            }
        }
    }

    /// Reads one entity and, for polylines and inserts, its children.
    fn read_entity(&mut self) -> Option<Entity> {
        let head = self.next()?;
        let mut entity = self.parse_record(&head);
        match &mut entity.kind {
            EntityKind::Polyline(poly) => {
                while self.peek_marker() == Some("VERTEX") {
                    let child = self.next()?;
                    poly.vertices.push(self.parse_record(&child));
                }
                poly.seqend = self.read_seqend(head.line, "POLYLINE");
            }
            EntityKind::Insert(insert) => {
                while self.peek_marker() == Some("ATTRIB") {
                    let child = self.next()?;
                    insert.attributes.push(self.parse_record(&child));
                }
                if !insert.attributes.is_empty() || self.peek_marker() == Some("SEQEND") {
                    insert.seqend = self.read_seqend(head.line, "INSERT");
                }
            }
            _ => {}
        }
        Some(entity)
    }

    fn read_seqend(&mut self, line: usize, parent: &str) -> Option<Box<Entity>> {
        if self.peek_marker() != Some("SEQEND") {
            self.warn(line, format!("{parent} has no SEQEND"));
            return None;
        }
        let head = self.next()?;
        Some(Box::new(self.parse_record(&head)))
    }

    /// Reads the body of the record opened by `head` and parses it.
    fn parse_record(&mut self, head: &Token<'a>) -> Entity {
        let mut body = std::mem::take(&mut self.body);
        self.read_body(&mut body);
        let entity = parse_entity(head, &body, &mut self.warnings);
        self.body = body;
        entity
    }
}

/// The warning for a `VERTEX`, `ATTRIB` or `SEQEND` outside its parent, or
/// `None` for any other entity.
fn orphan_message(entity: &Entity, previous: Option<&Entity>) -> Option<String> {
    let type_name = entity.kind.type_name();
    if !matches!(type_name, "VERTEX" | "ATTRIB" | "SEQEND") {
        return None;
    }
    let siblings: &[Entity] = match previous.map(|e| &e.kind) {
        Some(EntityKind::Polyline(p)) => &p.vertices,
        Some(EntityKind::Insert(i)) => &i.attributes,
        _ => &[],
    };
    let duplicate = entity.handle.is_some() && siblings.iter().any(|s| s.handle == entity.handle);
    Some(match (duplicate, entity.handle) {
        (true, Some(handle)) => format!(
            "{type_name} repeats handle {handle} of the preceding {}'s {type_name}; kept as is",
            previous.map_or("", |p| p.kind.type_name())
        ),
        _ => format!("{type_name} without a parent entity; kept as is"),
    })
}

fn block_exists(blocks: &Table<Block>, block: &Block) -> bool {
    !block.name.is_empty() && blocks.contains(block.name())
}

/// Parses a table entry and adds it, or keeps it verbatim with a warning.
fn add_entry<T: Named>(
    table: &mut Table<T>,
    expected: &str,
    head: &Token<'_>,
    body: &[Token<'_>],
    parse: fn(&[Token<'_>]) -> Result<T, String>,
    warnings: &mut Vec<Warning>,
) {
    let raw = || {
        let mut pairs = vec![head.to_pair()];
        pairs.extend(body.iter().map(Token::to_pair));
        pairs
    };
    let line = Some(head.line);
    if head.raw.trim() != expected {
        warnings.push(Warning::new(
            line,
            format!(
                "unexpected {:?} entry in {expected} table; kept as is",
                head.raw.trim()
            ),
        ));
        table.unparsed.push(raw());
        return;
    }
    match parse(body) {
        Ok(entry) if !entry.name().is_empty() && table.contains(entry.name()) => {
            warnings.push(Warning::new(
                line,
                format!("duplicate {expected} {:?}; kept as is", entry.name()),
            ));
            table.unparsed.push(raw());
        }
        Ok(entry) => table.entries.push(entry),
        Err(message) => {
            warnings.push(Warning::new(
                line,
                format!("{expected} entry kept as is: {message}"),
            ));
            table.unparsed.push(raw());
        }
    }
}
