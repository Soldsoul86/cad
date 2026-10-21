# Voice-over for the demo

For the bracket example (Rev B → Rev C). Play it with `?demo&narration`
(add `&format=landscape` or `&format=vertical` for a 16:9 or 9:16 frame).
The steps and gestures are those of the normal demo
(`docs/demo/demo-script.md`); each step lasts long enough to say its line
at about 2.5 words a second, with half a second before the line and 0.7 s
after it, and never less than the normal step. No captions are drawn; the
voice and the subtitle files carry the words. Everything on screen still
comes from the comparison, and every action is a real input.

The lines live in `crates/lightable/src/demo.rs` (`VOICEOVER`), where the
timing is computed; the test `the_voiceover_matches_the_comparison` checks
every fact below against the comparison of the two example files.

## Script

| Step | Line | On screen |
|---|---|---|
| Opening | "Rev C of this mounting bracket just came in. What changed?" | Rev C open on its own |
| Drop | "Drop both revisions on the page. Nothing to install, and nothing is uploaded." | The pointer drags the two files in and drops them; the names appear |
| Overlay | "Lightable finds ten changes. Blue is added, red deleted, yellow changed." | Overlay, change list, the legend highlighted |
| Cloud | "The revision cloud marks one of them, the bolt note. In this example, the other nine weren't clouded." | Pinch onto the cloud: M16 readable |
| Changes | "Every change is in plain words: the holes grew from 13.5 to 17.5 millimetres, and the edge dimensions from 20 to 25. Tap one to zoom to it." | Tap a hole in the list; at "and the edge…" tap the 20 → 25 mm dimension (the list scrolled first in 9:16) |
| Side by side | "See before and after, side by side." | Click Side by side |
| Report | "Then share a review report." | Click Details, then Export PNG report; the report on screen |
| Phone | "It works on your phone too." | The comparison in a phone outline |
| End card | "Lightable. Free, open source, and your files are read in your browser, never uploaded. lightable.dev" | End card |

## Fact check against the comparison

| Statement | Comparison | Result |
|---|---|---|
| "this mounting bracket", "Rev C" | Title block: STEEL MOUNTING BRACKET, REV C | Correct |
| "finds ten changes" | 10 design changes (plus 8 revision-markup items, listed separately) | Correct |
| "Blue is added, red deleted, yellow changed" | Legend colours and words: added (blue), deleted (red), changed (yellow) | Correct |
| "marks one of them, the bolt note" | 1 design change inside the cloud: the BOLTS note, M12 → M16 | Correct |
| "In this example, the other nine weren't clouded" | 10 − 1; the example was drawn with only the bolt note clouded (`cloud_by_design`) | Correct, and said of this example only |
| "the holes grew from 13.5 to 17.5 millimetres" | 4 circles, each "diameter changed from 13.5 mm to 17.5 mm" | Correct |
| "the edge dimensions from 20 to 25" | Two dimensions "changed from 20 mm to 25 mm" | Correct |
| "Tap one to zoom to it" | A click on a row selects it and zooms there | Correct |
| "share a review report" | Details, then Export PNG report (also JSON) | Correct (two clicks, not one) |
| "your files are read in your browser, never uploaded" | The page reads files in the browser and sends nothing (the recorder checks the network) | Correct |

## Cue sheet (narration video)

Times from the first frame of `demo-*-narration.mp4`. Record each line,
then drop the clip at its start time; each slot is the line at 2.5 words a
second, so a clip can run a little long into the gap before the next.

| # | Step | Start | End | Line |
|---|---|---|---|---|
| 1 | Opening | 00:00:00.500 | 00:00:04.900 | Rev C of this mounting bracket just came in. What changed? |
| 2 | Drop | 00:00:06.100 | 00:00:11.300 | Drop both revisions on the page. Nothing to install, and nothing is uploaded. |
| 3 | Overlay | 00:00:12.500 | 00:00:16.900 | Lightable finds ten changes. Blue is added, red deleted, yellow changed. |
| 4 | Cloud | 00:00:18.100 | 00:00:25.300 | The revision cloud marks one of them, the bolt note. In this example, the other nine weren't clouded. |
| 5 | Changes | 00:00:26.500 | 00:00:37.700 | Every change is in plain words: the holes grew from 13.5 to 17.5 millimetres, and the edge dimensions from 20 to 25. Tap one to zoom to it. |
| 6 | Side by side | 00:00:38.900 | 00:00:41.700 | See before and after, side by side. |
| 7 | Report | 00:00:42.900 | 00:00:44.900 | Then share a review report. |
| 8 | Phone | 00:00:46.900 | 00:00:49.300 | It works on your phone too. |
| 9 | End card | 00:00:50.900 | 00:00:55.700 | Lightable. Free, open source, and your files are read in your browser, never uploaded. lightable.dev |

Video length 00:00:56.400 (the end card is held after the last line).

The same cues are in `out/demo/demo-narration.srt` and `.vtt` (made by
`lightable demo-cues out/demo`).
