# Lightable UX audit

Walkthrough of https://lightable.dev (public build `0654581`, 11 Oct 2026)
as five first-time users, before any change. Screenshots in `img/`
(`before-*`; `after-*` once fixed). Browsers driven by
`tools/web_browser_test.mjs` (Chrome over CDP, Firefox over WebDriver
BiDi), with emulated phone sizes, touch, keyboard input and dropped files.

Severity: **B** blocks use · **C** causes confusion or costs trust ·
**P** polish. Effort: S (hours), M (a day), L (more).

## Walkthroughs

### 1. Mechanical engineer, laptop (1440 × 900)
`before-engineer-first-screen.png`. The overlay, "10 changes found between
Rev B and Rev C" and the list are clear; clicking a line zooms to it.
Hesitations: wants to try their own two revisions and finds no button for
it. The only hint is a tooltip on the example picker ("Drop two DXF files
on the window"). Unsure which of their files will count as "before". The
"Not compared yet" list, which decides whether they can trust "10 changes",
is only reachable through Details.

### 2. Fabrication shop owner, never used a compare tool (1366 × 768)
`before-shop-first-screen.png`. Understands "changes found" and the
coloured list. Misreads: "Polyline added", "Revision markup" are CAD words
but tolerable; the example picker reads "Example: Steel bracket…" and, once
they open their own files, "Example: Your drawings", which suggests they
are still in the example. Gets stuck on: opening files (no button), and on
errors (below), which they would not notice at all.

### 3. Architect, floor-plan example
`before-architect-plan.png`, `before-architect-details.png`. Headlines
mostly plain. Two lines are jargon: "Hatch pattern changed from SOLID to
ANSI37; solid fill changed from yes to no; pattern lines changed; other
hatch data changed" (four phrases for one change) and "Layer FURNITURE:
color changed from 8 to 6" (colour index numbers, meaningless outside CAD).
On first open of `?example=plan` the view was fitted before the list had its
width (fixed in `0654581`, which is live).

### 4. Phone, from a LinkedIn or Reddit link (390 × 844 portrait, 844 × 390 landscape)
`before-phone-portrait.png`, `before-phone-landscape.png`,
`before-phone-portrait-firefox.png`. Portrait: the change list, "Side by
side", "Details", Previous/Next and Close are off-screen; the summary is cut
mid-word; the hint "Click a change to zoom in" points at nothing.
Landscape: the list covers half the screen with every line cut off left and
right; the drawing is a thumbnail. Opening own files is impossible: phones
cannot drag and drop. Tooltips, the only place some actions are explained,
never appear on touch.

### 5. Keyboard only
`before-keyboard-focus.png`. Tab moves through the toolbar and buttons and
Enter activates them; N steps to the next change. The focus outline is a
faint grey line, easy to lose. There is no key for Previous, and nothing
tells the user that N exists (it is in a tooltip). The canvas is not
reachable by keyboard (pan/zoom need a mouse or touch).

### Edge and error states (dropped on the live page)
| Case | What happens | Screenshot |
|---|---|---|
| A DWG file | Small red text in the status bar: `part.dwg: line 1: invalid group code "AC1032\0\0…"`; the example stays on screen | `before-error-dwg.png` |
| A PDF | Same, `invalid group code "%PDF-1.7"` | `before-error-pdf.png` |
| Random bytes named .dxf | A status line full of garbage characters | `before-error-not-a-drawing.png` |
| Empty file | `not a DXF file (no SECTION found)` | `before-error-empty.png` |
| Truncated DXF + a good one | Compared without a word: "28 changes found", "Units ($INSUNITS) changed from (not set) to 4", "Layer 0 added" — every difference is an artefact of the cut-off file | `before-truncated-compared.png` |
| Same file twice | "No changes found between bracket-rev-c.dxf and bracket-rev-c.dxf"; correct, but does not say these are the same file | `before-same-file-twice.png` |
| Two unrelated drawings | "54 changes found", a long list of everything added and deleted; nothing suggests the files may not be revisions of one drawing | `before-unrelated-drawings.png` |
| One file | An older developer view: "87 items, 898 vertices · drawn: 34 chunks, 898 vertices · 897.8 ms/frame", a "Compare with…" toggle, layer panel | `before-one-file.png` |
| 50 MB DXF | No message while reading; the page is frozen for a few seconds, then the drawing appears (with "2206.4 ms/frame" in the toolbar) | `before-huge-file.png` |
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

(Filled in after the fixes: see below.)
