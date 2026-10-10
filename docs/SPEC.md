# 2D CAD Build Spec: DXF Core + Drafting App

## Overview

Build a free, open-source 2D CAD app in Rust that AutoCAD users can switch to without relearning: same command-line workflow, same core commands, and it reads and writes DXF. Step 1 is a standalone DXF library; Step 2 is the drafting app built on top of it.

**In scope for v0.1**

- DXF read/write (R12 and AC1015/2000 through AC1032/2018 ASCII)
- 2D entities: lines, polylines, circles, arcs, text, dimensions, blocks
- Layers, colors, linetypes, units
- 20 core drafting commands with a command line, snapping and undo
- Export to PDF and SVG

**Out of scope for v0.1**

- DWG (planned for v0.2 via LibreDWG bindings)
- 3D, solids, paper-space layouts beyond one default, xrefs, hatches with complex patterns, plugins/scripting

**Rules that apply everywhere**

- Clean-room: build from public DXF documentation and your own test files. Never use Autodesk code, icons, or the AutoCAD name in the product.
- Pick your own product name before the first public release. "Working name" in this spec is a placeholder.
- Hand each milestone to Claude Code as its own task, with the acceptance criteria from the Milestones section pasted in.

## Architecture

One Cargo workspace with five crates. `cad-dxf` and `cad-geom` have no UI dependencies, so other projects can use them on their own.

`cad-render` draws from the document held in `cad-core`; geometry maths lives only in `cad-geom`, so commands and rendering share one tested implementation.

```text
cad/
├── Cargo.toml          # workspace
├── crates/
│   ├── cad-dxf/        # Step 1
│   ├── cad-geom/
│   ├── cad-core/
│   ├── cad-render/
│   └── cad-app/        # binary
├── tests/corpus/       # DXF test files
├── docs/SPEC.md        # this spec
└── CLAUDE.md
```

## Step 1: DXF core library

The library (`cad-dxf`) turns a DXF file into a clean Rust data model and back, with no data loss for anything it doesn't understand. Before writing it, spend a day evaluating the existing `dxf` crate on crates.io: if its model and licence fit, wrap or fork it instead of starting from zero.

### Data model

- `Drawing` holds `Header`, `Tables`, `Blocks`, `Entities`, `Objects`.
- `Header`: units (`$INSUNITS`), extents, current layer, text style, dim style, codepage, version.
- `Tables`: layers, linetypes, text styles, dim styles, block records. Each keyed by name, case-insensitive.
- `Entity` = common fields (`handle`, `layer`, `color`, `linetype`, `lineweight`, `visible`) + `kind: EntityKind`.
- `EntityKind` enum for v0.1: `Line`, `LwPolyline` (with bulges), `Circle`, `Arc`, `Ellipse`, `Point`, `Text`, `MText`, `Insert` (block reference), `Dimension` (linear, aligned, radial, angular), `Spline` (read and display only).
- `Color`: ACI index (1–255), ByLayer, ByBlock, or 24-bit true colour.
- Anything unrecognised is kept as `Unknown { type_name, group_codes: Vec<(i16, Value)> }` and written back unchanged. This is the most important rule in the library.
- Use `f64` everywhere for coordinates; never `f32`.

### Reader

1. Tokenizer: read pairs of lines (group code, value). Handle CRLF and LF, leading spaces, and empty values.
2. Section parser for `HEADER`, `CLASSES`, `TABLES`, `BLOCKS`, `ENTITIES`, `OBJECTS`. Skip unknown sections without failing.
3. Text encoding: versions before 2007 use `$DWGCODEPAGE` (e.g. ANSI_1252); 2007 and later are UTF-8. Decode `\U+XXXX` escapes in strings.
4. Errors are recoverable where possible: one bad entity becomes `Unknown` plus a warning, not a failed load. Return `(Drawing, Vec<Warning>)`.
5. Binary DXF: detect its header and return a clear "not supported yet" error.

### Writer

- Write two targets: R12 (widest compatibility) and AC1032 (2018).
- Assign and preserve handles; keep owner/handle references consistent.
- Write `Unknown` items back byte-for-byte in their original position.
- Output must open in at least two other programs (LibreCAD, QCAD, or a free DWG/DXF viewer) without warnings.

### Public API (target shape)

```rust
let (drawing, warnings) = cad_dxf::read_file("plan.dxf")?;
for e in drawing.entities.iter().filter(|e| e.layer == "WALLS") { /* ... */ }
drawing.write_file("out.dxf", cad_dxf::Version::R2018)?;
```

### Testing

- Corpus: 50+ DXF files from different sources and versions, saved in `tests/corpus/` (use only files you are allowed to redistribute; otherwise keep a private corpus).
- Round-trip test: read → write → read gives an equal `Drawing` for every corpus file.
- Fuzzing with `cargo-fuzz` on the tokenizer and reader: no panics, ever.
- Golden renders: each corpus file rendered to SVG and compared against a checked-in snapshot, so regressions show up as image diffs.
- Performance: a 50 MB DXF loads in under 2 seconds on a mid-range laptop.

## Step 2: Drafting app

The app should feel familiar to an AutoCAD user within five minutes: type a command, answer the prompts, see the result. Everything is driven through the command line; toolbars are shortcuts to commands, never separate code paths.

### Canvas

- Infinite 2D canvas, dark background by default, world coordinates in `f64`, rendering in `f32` relative to the view centre to avoid precision jitter far from origin.
- Pan with middle mouse drag; zoom with wheel around the cursor; double-click middle = zoom extents.
- Crosshair cursor with a small pick box; coordinates shown live in the status bar.
- Spatial index (R-tree) over entity bounding boxes for picking, snapping and culling.
- Target: 100,000 entities at 60 fps pan/zoom.

### Command line

- Docked at the bottom. Type a command or alias, press Enter or Space to run.
- Enter or Space on an empty line repeats the last command; Esc cancels.
- Prompts show options in brackets, e.g. `Specify next point or [Undo/Close]:`. Typing the capital letter picks the option.
- Point input: click, absolute `10,20`, relative `@10,0`, polar `@50<45`, or direct distance (move the mouse in a direction and type `50`).
- Each command is a state machine implementing one `Command` trait: `on_point`, `on_text`, `on_option`, `preview`, `finish`, `cancel`.

### First 20 commands

| Command | Alias | What it must do in v0.1 |
| --- | --- | --- |
| LINE | L | Chained segments; Undo and Close options |
| PLINE | PL | Lightweight polyline; Arc and Line modes, width |
| CIRCLE | C | Center-radius, center-diameter, 2P, 3P |
| ARC | A | 3-point, start-center-end |
| RECTANG | REC | Two corners; result is a closed polyline |
| TEXT | T | Single-line text: insertion point, height, rotation |
| MOVE | M | Base point to second point, live preview |
| COPY | CO | Like MOVE, multiple copies until Enter |
| ROTATE | RO | Base point and angle, Reference option |
| SCALE | SC | Base point and factor, Reference option |
| MIRROR | MI | Two-point mirror line, keep or erase source |
| OFFSET | O | Distance and side; works on lines, arcs, circles, polylines |
| TRIM | TR | Pick objects to cut back to nearest boundaries |
| EXTEND | EX | Extend to nearest boundary |
| FILLET | F | Radius, including 0 for a sharp corner |
| ERASE | E | Delete the selection |
| EXPLODE | X | Polylines to segments, blocks to their contents |
| DIMLINEAR | DLI | Horizontal or vertical dimension with arrows and text |
| LAYER | LA | Panel to create, rename, colour, freeze, lock, set current |
| ZOOM | Z | Extents, Window, Previous |

UNDO (U) and REDO run outside this list: every command is one undo step, including its preview cancellations.

### Precision aids

- Object snap (F3): endpoint, midpoint, center, intersection, perpendicular, tangent, nearest; snap marker shapes shown at the cursor.
- Ortho (F8): lock to 0/90°. Polar tracking (F10) at 15° increments.
- Grid display (F7) and grid snap (F9).

### Selection

- Click picks one entity; Shift+click removes.
- Drag left-to-right = window (fully inside); right-to-left = crossing (touching), drawn with different box styles.
- Commands that need a selection accept a pre-selection or prompt `Select objects:`.

### Output

- Save and Save As to DXF (2018 default, R12 option).
- Plot to PDF and SVG: choose window or extents, paper size, scale (e.g. 1:100), and line weights by layer.

### Suggested crates

`egui` + `wgpu` (UI and canvas), `lyon` (path tessellation), `kurbo` (curve geometry), `rstar` (R-tree), `cosmic-text` or `rustybuzz` (text shaping), `printpdf` or `krilla` (PDF), `svg` (SVG export). Pin exact versions and check each licence is MIT/Apache-compatible.

## Milestones

Seven milestones, each small enough for one focused Claude Code task. Don't start the next until the current one passes its acceptance criteria.

1. **M1 — Tokenizer and model.** Workspace set up; `cad-dxf` reads group-code pairs and parses `HEADER`, `TABLES` (layers only) and `LINE`/`CIRCLE`/`ARC`.
   - Accept: 10 simple corpus files load with zero panics; unit tests per entity; CI runs `cargo test`, `clippy`, `fmt`.
2. **M2 — Full v0.1 reader.** All v0.1 entities, blocks, text encodings, `Unknown` preservation, warnings.
   - Accept: all 50+ corpus files load; fuzzing runs 1 hour with no crash; 50 MB file loads in under 2 s.
3. **M3 — Writer and round-trip.** R12 and 2018 output.
   - Accept: round-trip test passes for the whole corpus; written files open cleanly in two other CAD programs.
4. **M4 — Viewer.** `cad-app` opens a DXF and shows it with pan, zoom, layers on/off, and golden SVG snapshots.
   - Accept: 100k-entity file at 60 fps; snapshots match for the corpus.
5. **M5 — Command engine and drawing commands.** Command line, point input formats, LINE, PLINE, CIRCLE, ARC, RECTANG, TEXT, ERASE, undo/redo, object snap, ortho.
   - Accept: a scripted test draws a floor-plan outline using only typed commands and saves a valid DXF.
6. **M6 — Editing commands.** MOVE, COPY, ROTATE, SCALE, MIRROR, OFFSET, TRIM, EXTEND, FILLET, EXPLODE, selection windows.
   - Accept: each command has geometry tests including edge cases (parallel lines, zero radius, arcs crossing 0°).
7. **M7 — Dimensions, layers, plot, release.** DIMLINEAR, LAYER panel, PDF/SVG plot, installers for Windows, macOS, Linux.
   - Accept: a sample drawing plots to a correctly scaled A3 PDF at 1:100; README with screenshots and a side-by-side demo video; v0.1 tagged on GitHub.

## Working with Claude Code

Put this spec in the repo as `docs/SPEC.md` and the rules below in `CLAUDE.md`, so every session starts with the same context.

**Rules for `CLAUDE.md`**

- Read `docs/SPEC.md` before any task. Work on one milestone at a time.
- Clean-room only: use public DXF documentation and test files. Never paste or imitate Autodesk source code, icons or branding.
- Never panic on user input; return errors or warnings.
- Unknown DXF data must survive a round-trip unchanged.
- Every new entity or command ships with tests. Run `cargo test`, `cargo clippy -- -D warnings` and `cargo fmt --check` before finishing.
- Pin exact dependency versions; only MIT/Apache-2.0/BSD/zlib licences.
- Keep a `CHANGELOG.md` entry per milestone.

**Starter prompt for M1**

```text
Read docs/SPEC.md and CLAUDE.md. Set up the Cargo workspace described in the Architecture section. Then implement milestone M1 only: the DXF tokenizer, the Drawing/Header/Tables/Entity model, and parsing of LINE, CIRCLE and ARC. Add unit tests and a GitHub Actions CI workflow. Stop when M1's acceptance criteria pass and summarise what's left for M2.
```

**Prompt pattern for every later milestone**

```text
Read docs/SPEC.md and CLAUDE.md. Implement milestone M<N> only. Acceptance criteria: <paste from the spec>. Write tests first for the trickiest cases, then the code. Run the full test suite and fix failures before finishing. List any spec questions you had to decide yourself.
```

**Tips**

- Ask Claude Code to write a tiny DXF by hand for each new entity before parsing it; small fixtures make bugs obvious.
- For geometry commands (TRIM, OFFSET, FILLET), ask for the maths as a pure function in a `geom` module first, with tests, before wiring the UI.
- When a corpus file renders wrong, give Claude Code the file, the snapshot diff, and a screenshot from another viewer.
