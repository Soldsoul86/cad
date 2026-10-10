# Changelog

## M1 — Tokenizer and model (unreleased)

- Cargo workspace with `cad-dxf`, `cad-geom`, `cad-core`, `cad-render` and `cad-app` (the last four are stubs).
- `cad-dxf` tokenizer: group-code/value pairs with LF or CRLF line endings, padded codes, empty values, a UTF-8 BOM and line numbers; binary DXF is rejected with a clear error.
- Reader: `HEADER` (all variables kept, typed accessors for version, code page, units, extents, current layer, text style and dim style), `TABLES` (LAYER parsed; other tables kept verbatim), `ENTITIES` (`LINE`, `CIRCLE`, `ARC` parsed; everything else kept verbatim as `Unknown`); `CLASSES`, `BLOCKS`, `OBJECTS` and unknown sections kept verbatim.
- Recoverable errors: a malformed entity becomes `Unknown` plus a warning; truncated files keep what was read; missing `ENDSEC`/`ENDTAB`/`EOF` are warnings.
- Corpus of 13 hand-written DXF files in `tests/corpus/`, corpus tests, and a truncation/corruption sweep that checks the reader never panics.
- GitHub Actions CI: `fmt --check`, `clippy -D warnings`, `test` on Linux, Windows and macOS.
