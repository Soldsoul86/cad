# Changelog

## M2 part B — nothing dropped, encodings, fuzzing (unreleased)

- Stray and duplicate records (e.g. an extra `ENDBLK`) are kept in `Drawing::stray` with their position; an unreadable tail is kept in `Drawing::unparsed_tail`. Warnings unchanged.
- `ATTDEF` parsed.
- `$DWGCODEPAGE` decoding for pre-2007 files (ANSI_874, 932, 936, 949, 950, 1250–1258) via `encoding_rs`; `Drawing::encoding` records the result.
- `\U+XXXX`, `\M+nXXXX` and `%%` codes decoded in TEXT, ATTRIB, ATTDEF, MTEXT and dimension text; the original string is kept when it differs. Public `decode_text` / `decode_mtext`.
- cargo-fuzz targets (`tokenizer`, `reader`, `text`) seeded from the corpus, `tools/fuzz.sh`, and replayed regression inputs. A 1-hour run of all three found no crashes (223 M executions in total).
- All corpus files load with zero warnings, except the 18 known LibreDWG files (duplicate records) and the two deliberately damaged fixtures.

## M2 part A — performance, entities, blocks, tables (unreleased)

- Streaming reader with borrowed tokens: the 50 MB benchmark file loads in about 0.8–1.05 s (was 3.1 s). New benchmark: `cargo bench -p cad-dxf --bench load`.
- R13 and R14 added to `Version`.
- New typed entities: `LWPOLYLINE` (bulges, widths, vertex ids), `ELLIPSE`, `POINT`, `TEXT`, `MTEXT`, `INSERT` (arrays, attributes), `ATTRIB`, `DIMENSION` (linear, aligned, angular, diameter, radius, 3-point angular, ordinate), `SPLINE`, and `POLYLINE`/`VERTEX`/`SEQEND` sequences.
- `BLOCKS` section parsed into `Drawing::blocks` (`Table<Block>`), including nested references and child sequences inside blocks.
- New typed tables: linetypes (with complex elements), text styles, dim styles, block records.
- `cad-geom`: `bulge::arc_from_bulge` and `bulge_from_sweep`.
- Corpus acceptance test: all real-writer files load with no warnings except encoding, apart from 18 LibreDWG files whose duplicate records are reported.

## Unreleased (pre-M2)

- Recorded M1 design decisions in `docs/DECISIONS.md`; dropped the `dxf` crate evaluation from the spec.
- Added `LICENSE-MIT` and `LICENSE-APACHE`.
- Committed the corpus generator as `tools/gen_corpus.py`.
- Corpus tests also load files from the git-ignored `tests/corpus-private/` when present.
- Real-writer corpus: 13 drawings generated with ezdxf (`tools/gen_sources.py`) and re-saved by ODA File Converter, FreeCAD and LibreDWG (`tools/convert_corpus.sh`) — 198 files across R12, R14 and R2000–R2018. Provenance in `tests/corpus/SOURCES.md`, coverage in `tests/corpus/COVERAGE.md` (`tools/corpus_coverage.py`).
- A ~50 MB mixed-entity speed-test file can be generated into `tests/corpus-private/`.
- The exhaustive truncation/corruption sweep now runs only on the small hand-written files.

## M1 — Tokenizer and model (unreleased)

- Cargo workspace with `cad-dxf`, `cad-geom`, `cad-core`, `cad-render` and `cad-app` (the last four are stubs).
- `cad-dxf` tokenizer: group-code/value pairs with LF or CRLF line endings, padded codes, empty values, a UTF-8 BOM and line numbers; binary DXF is rejected with a clear error.
- Reader: `HEADER` (all variables kept, typed accessors for version, code page, units, extents, current layer, text style and dim style), `TABLES` (LAYER parsed; other tables kept verbatim), `ENTITIES` (`LINE`, `CIRCLE`, `ARC` parsed; everything else kept verbatim as `Unknown`); `CLASSES`, `BLOCKS`, `OBJECTS` and unknown sections kept verbatim.
- Recoverable errors: a malformed entity becomes `Unknown` plus a warning; truncated files keep what was read; missing `ENDSEC`/`ENDTAB`/`EOF` are warnings.
- Corpus of 13 hand-written DXF files in `tests/corpus/`, corpus tests, and a truncation/corruption sweep that checks the reader never panics.
- GitHub Actions CI: `fmt --check`, `clippy -D warnings`, `test` on Linux, Windows and macOS.
