# Voice-over for the demo

For the bracket example (Rev B → Rev C). Play it with `?demo&narration`
(add `&format=landscape` or `&format=vertical` for a 16:9 or 9:16 frame).
In narration mode each step lasts long enough to say its line at about
2.5 words a second, with half a second before the line and 0.7 s after it
for the view to settle; no captions are drawn on screen, so the voice and
the subtitle files carry the words. Everything on screen (counts, headlines,
the change list) still comes from the comparison.

The lines live in `crates/lightable/src/demo.rs` (`VOICEOVER`), where the
timing is computed; the test `the_voiceover_matches_the_comparison` checks
every fact below against the comparison of the two example files.

## Script

| Step | Line | On screen |
|---|---|---|
| Overview | "Rev C of this mounting bracket just came in." | Rev B and Rev C side by side, whole sheet |
| Cloud zoom | "The revision cloud marks one change: the bolt note, M12 to M16." | Both revisions zoomed to the cloud, the bolt note and the revision triangle |
| Side by side | "Easy to check that and send it on." | Side by side, whole sheet |
| Overlay | "But Lightable compares both revisions and finds ten changes." | Overlay; "10 changes found between Rev B and Rev C" |
| Changes | "The four holes grew from 13.5 to 17.5 millimetres, and each moved 5 millimetres in from both edges. The edge and pitch dimensions changed too. None of them were clouded." | A hole selected and framed; at "The edge…" a dimension (20 → 25); at "None of them…" the whole overlay again |
| Report | "Click any change to see it, and export a review report." | The hole callout selected; at "and export…" the report layout (whole sheet) |
| End card | "Lightable. Free, open source, and your files never leave your browser. lightable.dev" | End card |

## Fact check against the comparison

| Statement | Comparison | Result |
|---|---|---|
| "mounting plate" (draft) | Title block: STEEL MOUNTING BRACKET (made from 10 mm plate) | **Corrected** to "mounting bracket" |
| "marks one change: the bolt note, M12 to M16" | 1 design change inside the cloud: "Text changed from BOLTS: 4x M12 ISO 4017 - 8.8 to BOLTS: 4x M16 ISO 4017 - 8.8" | Correct |
| "finds ten changes" | 10 design changes (plus 8 revision-markup items, listed separately) | Correct |
| "The four holes grew from 13.5 to 17.5 millimetres" | 4 circles, each "diameter changed from 13.5 mm to 17.5 mm" | Correct |
| "each one moved 5 millimetres" (draft) | Each centre moved 5 mm across **and** 5 mm up or down (about 7.1 mm diagonally), towards the middle: the edge distances went from 20 to 25 mm | **Corrected** to "each moved 5 millimetres in from both edges" |
| "The edge and pitch dimensions changed too" | Edge 20 → 25 mm (two), pitch 120 → 110 mm and 60 → 50 mm | Correct |
| "None of them were clouded" | Only the bolt note is inside the cloud | Correct |
| "export a report in one click" (draft) | The exports are under Details: Details, then Export PNG report (or JSON) | **Corrected** to "and export a review report" |
| "your files never leave your browser" | The page reads files in the browser and sends nothing (network checks) | Correct for the web version (the end card on screen says "computer") |

## Cue sheet (narration video)

Times from the first frame of `demo-*-narration.mp4`. Record each line,
then drop the clip at its start time; each slot is the line at 2.5 words a
second, so a clip can run a little long into the 1.2 s gap before the next.

| # | Step | Start | End | Line |
|---|---|---|---|---|
| 1 | Overview | 00:00:00.500 | 00:00:04.100 | Rev C of this mounting bracket just came in. |
| 2 | Cloud zoom | 00:00:05.300 | 00:00:10.100 | The revision cloud marks one change: the bolt note, M12 to M16. |
| 3 | Side by side | 00:00:11.300 | 00:00:14.500 | Easy to check that and send it on. |
| 4 | Overlay | 00:00:15.700 | 00:00:19.300 | But Lightable compares both revisions and finds ten changes. |
| 5 | Changes | 00:00:20.500 | 00:00:32.500 | The four holes grew from 13.5 to 17.5 millimetres, and each moved 5 millimetres in from both edges. The edge and pitch dimensions changed too. None of them were clouded. |
| 6 | Report | 00:00:33.700 | 00:00:38.100 | Click any change to see it, and export a review report. |
| 7 | End card | 00:00:39.300 | 00:00:44.100 | Lightable. Free, open source, and your files never leave your browser. lightable.dev |

Video length 00:00:45.800 (the end card is held after the last line).

The same cues are in `out/demo/demo-narration.srt` and `.vtt` (made by
`lightable demo-cues out/demo`).
