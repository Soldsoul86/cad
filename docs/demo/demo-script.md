# The 30-second demo (`?demo`, `--demo`)

Situation: a manufacturing engineer checks Rev C of a steel bracket drawing
before it goes to the supplier. The revision cloud suggests a small change;
the drawing changed in more places.

First screen (before the demo starts, and for any pair of files):

> **10 changes found between Rev B and Rev C** · plus revision markup (8 items, including 1 revision cloud)
> *Example: the drafter clouded only the bolt note.*

The first line comes from the comparison for any files (the cloud part is
left out when no revision cloud changed). The italic line belongs to the
built-in bracket example only: it describes how the example was drawn and
is never shown for your own files. The 8 markup items are drawing objects:
the revision letter, the revision-table row (a polyline, a line and two
texts), the cloud, and the revision triangle with its letter.

Every number and headline on screen comes from the comparison of the two
DXF files at the moment the demo runs (`crates/lightable/src/app.rs`,
`run_demo`). Nothing is typed in by hand except the connecting words.

Each step stays on screen for its caption's reading time: 3 words a
second plus one second for the zoom to settle, at least 3 seconds
(`caption_seconds` in `demo.rs`); the opening has no zoom and lasts 3 s.
Long headlines are shown one clause per caption (split at "; ", words
unchanged). With the bracket the demo runs about 31 s, then holds the end
card for 3 s (34.3 s in all).

| Start | Length | On screen | Caption | Where the words come from |
|---|---|---|---|---|
| 0 s | 3.0 s | Rev C alone, plain colours, filling the frame | "Rev C just came in. What changed?" | Example label |
| 3.0 s | 3.0 s | Rev B and Rev C side by side (stacked in 9:16), fitted tightly | "Before: Rev B. After: Rev C." | Example labels |
| 6.0 s | 3.0 s | Side by side, close on the cloud and the bolt note (M12 / M16 readable) | "The revision cloud marks 1 change." | Design changes whose centre is inside the added cloud (narration only) |
| 9.0 s | 3.0 s | Overlay, whole sheet, the change list appears | "Lightable found 10 changes." | `Diff::design_changes().count()` |
| 12.0 s | 4.0 s | A hole, framed | "Circle diameter changed from 13.5 mm to 17.5 mm" | First clause of the first changed circle's headline |
| 16.0 s | 4.0 s | (same) | "Center moved 5 mm right and 5 mm up" | Second clause of the same headline |
| 20.0 s | 3.7 s | A dimension, framed | "Dimension changed from 20 mm to 25 mm" | The first changed dimension's headline |
| 23.7 s | 3.7 s | Report layout, whole sheet | "Download the review report as PNG or JSON." | The PNG report layout (shown, not saved) |
| 27.3 s | 4.0 s | Overlay, whole sheet | "Whether each change is right is still your call." | Scope statement |
| 31.3 s | 3.0 s | End card | "Free. Open source. / Your files never leave your computer. / lightable.dev" | |

Captions are 4.5 % of the screen height (smaller only to fit two lines),
white on near-black, at the bottom of the drawing; the demo's zooms fit the
change into the area above them, so a caption never covers it. The change
list appears at "found 10 changes"; in 9:16 only for that step (the caption
shows the current change instead). The Details step was taken out of the
short demo (it suits a longer explainer).

The three clicks are chosen from the real results by `Compare::demo_picks`:
one change of each kind (circle, dimension, text) that the cloud does not
surround. The test `the_demo_narrates_the_bracket_from_real_results`
(`crates/lightable/src/compare.rs`) checks the captions against the
comparison; `the_bracket_cloud_surrounds_exactly_one_design_change`
(`crates/lightable-diff/tests/examples.rs`) checks the cloud sentence.

## What the demo does not claim

- Lightable does not check whether a revision cloud covers every change.
  The cloud sentence is narration about this example, computed for the
  demo only.
- It does not say whether a drawing is ready for manufacturing, whether a
  design is safe, or whether a change is correct.
- It compares drawing data (entities, their values, layers, blocks, units).
  It does not understand the part. "Not compared yet" lists what it skips.

## Other examples

`?demo&example=plan` plays the same script on the floor plan: there is no
revision cloud, so the second caption says so, and the clicks are three of the plan's
changes (a circle, dimension or text first where there is one).

## Voice-over mode and video files

`?demo&narration` plays the voice-over version for the bracket example: the
seven steps of `docs/demo/voiceover.md`, each long enough to speak its line
at about 2.5 words a second plus settling time, with no on-screen captions
(about 46 s). Other drawings fall back to the normal demo. The normal
timing above is unchanged.

Subtitles and the cue sheet come from the same plan the app plays
(`crates/lightable/src/demo.rs`):

```bash
cargo run --release -p lightable -- demo-cues out/demo
```

writes `demo.srt`/`.vtt` (normal demo: the on-screen captions and the end
card), `demo-narration.srt`/`.vtt` (the voice-over lines) and
`cue-sheet.md`. Times count from the first frame of the matching video.

Videos are recorded frame by frame from the web build: `?demo&record` makes
demo time follow a frame counter the recorder sets (30 frames a second), so
the result is smooth however slowly the page renders:

```bash
node tools/record_demo.mjs "http://localhost:8765/target/web/index.html?demo&record&format=landscape" out/demo/demo-landscape.mp4 960 540
```

(960 × 540 CSS pixels at a pixel ratio of 2 = 1920 × 1080; use 540 960 and
`format=vertical` for 1080 × 1920, and add `&narration` for the voice-over
timing.) `out/` is git-ignored.
