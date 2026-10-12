# Lightable UX audit

Walkthrough of https://lightable.dev (public build `0654581`, 11 Oct 2026)
as five first-time users, before any change. Screenshots in `img/`
(`before-*`, `after-*` once fixed). Browsers driven by
`tools/web_browser_test.mjs` (Chrome over CDP, Firefox over WebDriver
BiDi), with emulated phone sizes, touch, keyboard input and dropped files.

Severity: **B** blocks use · **C** causes confusion or costs trust ·
**P** polish. Effort: S (hours), M (a day), L (more).

## Walkthroughs

### 1. Mechanical engineer, laptop (1440 × 900)
`before-engineer-first-screen.jpg`. The overlay, "10 changes found between
Rev B and Rev C" and the list are clear; clicking a line zooms to it.
Hesitations: wants to try their own two revisions and finds no button for
it. The only hint is a tooltip on the example picker ("Drop two DXF files
on the window"). Unsure which of their files will count as "before". The
"Not compared yet" list, which decides whether they can trust "10 changes",
is only reachable through Details.

### 2. Fabrication shop owner, never used a compare tool (1366 × 768)
`before-shop-first-screen.jpg`. Understands "changes found" and the
coloured list. Misreads: "Polyline added", "Revision markup" are CAD words
but tolerable; the example picker reads "Example: Steel bracket…" and, once
they open their own files, "Example: Your drawings", which suggests they
are still in the example. Gets stuck on: opening files (no button), and on
errors (below), which they would not notice at all.

### 3. Architect, floor-plan example
`before-architect-plan.jpg`, `before-architect-details.jpg`. Headlines
mostly plain. Two lines are jargon: "Hatch pattern changed from SOLID to
ANSI37; solid fill changed from yes to no; pattern lines changed; other
hatch data changed" (four phrases for one change) and "Layer FURNITURE:
color changed from 8 to 6" (colour index numbers, meaningless outside CAD).
On first open of `?example=plan` the view was fitted before the list had its
width (fixed in `0654581`, which is live).

### 4. Phone, from a LinkedIn or Reddit link (390 × 844 portrait, 844 × 390 landscape)
`before-phone-portrait.jpg`, `before-phone-landscape.jpg`. (Phone views are
emulated at a device pixel ratio of 2: on this Mac, emulating 3 makes the
browser size the canvas at 2× while reporting 3×, which magnifies the UI
1.5× and shifts pointer input. The CSS layout is the same as on a 3× phone.) Portrait: the change list, "Side by
side", "Details", Previous/Next and Close are off-screen; the summary is cut
mid-word; the hint "Click a change to zoom in" points at nothing.
Landscape: the list covers half the screen with every line cut off left and
right; the drawing is a thumbnail. Opening own files is impossible: phones
cannot drag and drop. Tooltips, the only place some actions are explained,
never appear on touch.

### 5. Keyboard only
`before-keyboard-focus.jpg`. Tab moves through the toolbar and buttons and
Enter activates them; N steps to the next change. The focus outline is a
faint grey line, easy to lose. There is no key for Previous, and nothing
tells the user that N exists (it is in a tooltip). The canvas is not
reachable by keyboard (pan/zoom need a mouse or touch).

### Edge and error states (dropped on the live page)
| Case | What happens | Screenshot |
|---|---|---|
| A DWG file | Small red text in the status bar: `part.dwg: line 1: invalid group code "AC1032\0\0…"`; the example stays on screen | `before-error-dwg.jpg` |
| A PDF | Same, `invalid group code "%PDF-1.7"` | `before-error-pdf.jpg` |
| Random bytes named .dxf | A status line full of garbage characters | `before-error-not-a-drawing.jpg` |
| Empty file | `not a DXF file (no SECTION found)` | `before-error-empty.jpg` |
| Truncated DXF + a good one | Compared without a word: "28 changes found", "Units ($INSUNITS) changed from (not set) to 4", "Layer 0 added" — every difference is an artefact of the cut-off file | `before-truncated-compared.jpg` |
| Same file twice | "No changes found between bracket-rev-c.dxf and bracket-rev-c.dxf"; correct, but does not say these are the same file | `before-same-file-twice.jpg` |
| Two unrelated drawings | "54 changes found", a long list of everything added and deleted; nothing suggests the files may not be revisions of one drawing | `before-unrelated-drawings.jpg` |
| One file | An older developer view: "87 items, 898 vertices · drawn: 34 chunks, 898 vertices · 897.8 ms/frame", a "Compare with…" toggle, layer panel | `before-one-file.jpg` |
| 50 MB DXF | No message while reading; the page is frozen for a few seconds, then the drawing appears (with "2206.4 ms/frame" in the toolbar) | `before-huge-file.jpg` |
| WebGL2 unavailable | (from the code) "Could not start: " followed by a JavaScript error | — |

### Colour and contrast (computed with `tools/palette_check.py`: WCAG 2 contrast; Machado 2009 colour-vision simulation, CIE76 ΔE)
| Colour on #212121 | Contrast |
|---|---|
| added #3CD05A | 7.95 |
| deleted #F04848 | 4.39 (just under 4.5) |
| changed #F5D02A | 10.69 |
| grey text 140–200 | 4.8–10.3 |

Added vs deleted, ΔE: normal 128, **deuteranopia 11.7** (barely
distinguishable; about 5 % of men), protanopia 45. The list and legend say
"added"/"deleted" in words, but in the overlay a moved part is a red and a
green outline that a deuteranope sees as two near-identical olive lines.

## Issues, ranked

| # | Sev | Effort | Issue | Plan |
|---|---|---|---|---|
| U1 | B | M | No way to open own files except drag and drop; impossible on phones and not discoverable on desktop | "Choose files…" button (web) that opens the system file dialog for two files |
| U2 | B | M | Phone portrait: list and most buttons off-screen, summary cut | Narrow / portrait layout: list under the drawing, toolbar and summary wrap |
| U3 | B | S | Errors are tiny red status text in parser jargon or garbage; easy to miss, no next step | Plain-language error window: what happened, what to do (DWG: save as DXF, how) |
| U4 | B | S | A truncated DXF is compared silently and produces false changes | Refuse files that end mid-section, with a plain message; warn on a missing end marker |
| U5 | C | S | Which file is "before" is not shown or changeable | "Before: a.dxf → After: b.dxf" with a Swap button; default order by file date, then name |
| U6 | C | S | One file opens a developer view with render statistics | Plain viewer: file name, "Choose a second file to compare"; statistics only in a tooltip |
| U7 | C | S | Picker says "Example: Your drawings" for the user's own files | Label "Your drawings" without "Example:" |
| U8 | C | S | Same file twice not recognised | "You chose the same file twice" note |
| U9 | C | S | Unrelated drawings give a long add/delete list with no hint | Note when almost nothing matches: check these are two revisions of one drawing |
| U10 | C | S | Red/green coding fails for deuteranopia; deleted red below 4.5:1 | Added → blue (#4CB0FF): ΔE 95 deutan, 71 protan; legend unchanged in words |
| U11 | C | S | Focus outline faint; no Previous key; N undiscoverable | Bright focus outline; P for previous; keys shown on the buttons' line |
| U12 | C | S | No status while a big file is read | "Reading big.dxf (50 MB)…" shown before the work starts |
| U13 | C | S | "Not compared yet" only under Details | "What isn't compared?" link under the list |
| U14 | C | S | Privacy promise not visible in the app | Short line by "Choose files…": files are read in this browser, never uploaded |
| U15 | C | M | Jargon in some headlines: four phrases for one hatch change; colour index numbers | Hatch: pattern change only (rest in details); colour names for index 1–9 ("grey (8) to magenta (6)") |
| U16 | C | S | Hints point at hidden things on small screens; tooltips never show on touch | Hints only when their target is visible; essential help on screen, not in tooltips |
| U17 | P | S | Landscape phone: list lines cut off both sides | Wrapping headlines; list width a share of the screen |
| U18 | P | S | Empty state (after Close) says "or pass one on the command line" on the web | Web wording: choose two files or try the example |
| U19 | P | S | Start-up failure shows a raw JavaScript error | Plain message (needs WebGL2; which browsers) with the error as detail |
| U20 | P | S | Status-bar messages coloured red/green only | Covered by U3 (errors move to a window with a heading) |
| D1 | — | M | Native app: no file dialog either | Deferred: needs a new dependency (rfd); the web app is the audience here |
| D2 | — | L | No screen-reader support in the web build (egui draws to a canvas) | Deferred; stated in limits |
| D3 | — | M | Big files are read on the main thread (page frozen meanwhile) | Deferred (worker); U12 at least says what is happening |
| D4 | — | M | Pan and zoom need a pointer or touch | Deferred; Next/Previous and the list zoom by keyboard |

## Demo (`?demo`) timing

At 3 words per second the captions need: overview 11 words → 3.7 s; cloud
6 → 2 s; count 4 → 1.3 s; circle headline 18 words → 6 s; dimension 6 →
2 s; callout 9 → 3 s; Details 9 → 3 s; report 9 → 3 s; scope 12 → 4 s. The
fixed 3–3.5 s steps leave the circle headline on screen for half the time
needed. Selection highlight is a 1 px white box: invisible in a small video.

## Results

Re-checked on a local build (`tools/build_web.sh`) in Chrome and Firefox,
desktop (1440 × 900) and emulated phones (390 × 844, 844 × 390), with the
same harness. 259 tests pass (new: `intake.rs` 7, file batches 1, layout 1,
demo timing 2; changed: plan headlines, cloud/no-coordinates checks).

| # | Status | What changed | After |
|---|---|---|---|
| U1 | Fixed (web) | "Choose files…" opens the system dialog (several files); works by tap on phones | `after-engineer-first-screen.jpg`, `after-phone-portrait.jpg` |
| U2 | Fixed | Portrait: list under the drawing (35 %), toolbar in rows, summary wraps | `after-phone-portrait.jpg`, `after-phone-portrait-next.jpg`, `after-phone-portrait-firefox.jpg` |
| U3 | Fixed | A window says what the file is and what to do; reader message under "Details", cleaned | `after-error-dwg.jpg`, `after-error-pdf.jpg`, `after-error-not-a-drawing.jpg`, `after-error-empty.jpg` |
| U4 | Fixed | Cut-off DXF refused with an explanation | `after-truncated-refused.jpg` |
| U5 | Fixed | Revision markers in names first (Rev B, _C, r2, v3), then dates; if they disagree or say nothing, the user chooses; "Before: … → After: …" with Swap | `after-order-by-name.jpg` (part_C chosen first, put second), `after-order-ask.jpg` (no clue) |
| U6 | Fixed | One file: name, "To compare, choose (or drop) the other revision."; statistics in a tooltip; the next file is compared with it | `after-one-file.jpg`, `after-one-file-then-other.jpg` |
| U7 | Fixed | Picker reads "Examples" for the user's own files | `after-order-and-swap.jpg` |
| U8 | Fixed | "You chose the same file twice (both files have identical content)…" | `after-same-file-twice.jpg` |
| U9 | Fixed | "These drawings have almost nothing in common…" | `after-unrelated-drawings.jpg` |
| U10 | Fixed | Added is blue; deuteranopia ΔE 11.7 → 95 | all `after-*` |
| U11 | Fixed | Blue 2-point focus outline; P for previous; "N / P keys step" in the list header | `after-keyboard-focus.jpg`, `after-keyboard-previous-key.jpg` |
| U12 | Fixed | "Reading big_save.dxf (50 MB)…" in the middle of the screen while the page is busy (≈ 4.5 s) | `after-huge-file-reading.jpg` (a compositor frame: screenshots cannot be taken while the page is busy) |
| U13 | Fixed | "What isn't compared?" under the list opens Details at that list | `after-not-compared-link.jpg` |
| U14 | Fixed | "Files are read in this browser, never uploaded." beside "Choose files…" and on the loading page | `after-engineer-first-screen.jpg` |
| U15 | Fixed | "Hatch pattern changed from SOLID to ANSI37"; "colour changed from dark grey (8) to magenta (6)" | `after-architect-plan.jpg` |
| U16 | Fixed | Hints only beside visible targets; on phones only the list hint | `after-phone-portrait.jpg` |
| U17 | Fixed | Landscape phone: list beside the drawing, compact summary, wrapped rows | `after-phone-landscape.jpg`, `after-phone-landscape-firefox.jpg` |
| U18 | Fixed | Empty state: "Compare two revisions of a drawing", "Choose two DXF files…", "Or try the example", privacy line (no command-line wording on the web) | `after-empty-state.jpg` (after Close) |
| U19 | Fixed | Plain WebGL2 message with the error as a detail | — (not reproducible here) |
| U20 | Fixed | Status bar is neutral; problems are in the window | — |
| D1–D4 | Deferred | See "Remaining limitations" | |

Also found and fixed while checking: with Details on at 960 points wide
the toolbar's right-hand buttons overlapped the exports (now inline); long
headlines in the list did not wrap.

**Network:** in every run (picks, drops, errors, demo, phones; Chrome and
Firefox) the page made only its own 5 requests (Firefox also lists the two
`data:` icons); none after files were chosen or dropped, none to another
origin, none with a body.

## Demo

Timing now follows the captions (≥ 3 words per second, plus 1 s for the
zoom, minimum 3 s); about 37 s plus the end card. Captions scale with the
screen and wrap; the selection frame is 2–6 points with a dark halo.
`?demo&format=landscape` / `?demo&format=vertical` give 16:9 / 9:16 frames
(`demo-16x9-step0…9.jpg`, `demo-9x16-step0…9.jpg`, captured at exactly
1920 × 1080 and 1080 × 1920). Every number and headline still comes from
the comparison. Script: `docs/demo/demo-script.md`.

## Remaining known UX limitations

- **Native app has no file dialog** (D1): drag and drop or the command line.
- **No screen-reader support** (D2): the web app draws to a canvas; egui's
  accessibility tree is not available in the browser build. Keyboard use
  works (Tab, Enter, N/P), but pan and zoom need a pointer or touch (D4).
- **Big files block the page while they are read** (D3): the message says
  what is happening, but the page does not respond for those seconds
  (≈ 4.5 s for 50 MB here; longer on phones).
- **Phone layout is usable, not designed for phones:** the drawing area in
  portrait is about a third of the screen; tooltips (which hold some
  secondary help) never show on touch; the list's resize handle is small.
- **Ordering trusts names and dates:** markers are read from common
  patterns only; copied or downloaded files may all carry the copy time.
  When the clues disagree or are missing the user is asked; Swap is always
  one tap away.
- **Errors are detected from the first bytes and the reader's result**: a
  DWG renamed to .dxf is still recognised; a corrupt DXF that parses (but
  is wrong) is not.
- **"Almost nothing in common"** is a threshold (fewer than 1 in 10 matched),
  not an understanding of the drawings.
- **Safari not tested** (no automation access on this Mac); phone checks are
  emulated, not on real devices.
