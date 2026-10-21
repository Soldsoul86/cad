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

## Use-case videos (the demo page)

One voice-over line per step, for the branded videos (tools/stage/). Each
video tells hook, action, reveal, proof, payoff, end card; the captions
stay in the picture in every version, and the voice says the same facts.
Record each line within its times (at most 2.8 words a second; the first
line starts after the 1.2 s sting). The videos come with a quiet music bed
and UI sounds, without music, and as separate stems
(`out/demo/audio/*-music.wav`, `*-sfx.wav`), so a recorded voice can be laid
on its own track. Made by `lightable demo-cues out/demo`
(`feature-ID-voice.md`, and `feature-ID-voice.srt` as a guide to record to);
a test keeps the lines here equal to the plans.

### 1. The changes the cloud leaves out (gearbox cover)

| # | Scene | Start | End | On screen | Say |
|---|---|---|---|---|---|
| 1 | hook | 00:00:01.300 | 00:00:04.500 | The cloud marks 1 change. Is that everything? | The cloud marks 1 change. Is that everything? |
| 2 | action | 00:00:04.900 | 00:00:08.500 | Drop in both revisions. | Drop in both revisions. |
| 3 | reveal | 00:00:08.900 | 00:00:12.500 | 14 changes found between Rev B and Rev C. | 14 changes found between Rev B and Rev C. |
| 4 | proof | 00:00:12.900 | 00:00:17.500 | The cloud marks 1 change; 13 others weren't clouded. | The cloud marks 1 change; 13 others weren't clouded. |
| 5 | proof | 00:00:17.900 | 00:00:22.500 | Circle diameter changed from 11 mm to 13.5 mm | Circle diameter changed from 11 mm to 13.5 mm |
| 6 | proof | 00:00:22.900 | 00:00:26.500 | On layer SECTION: line moved 1.25 mm up. | On layer SECTION: line moved 1.25 mm up. |
| 7 | payoff | 00:00:26.900 | 00:00:29.500 | Every change listed, clouded or not. | Every change listed, clouded or not. |

Video length 00:00:32.700 with the stage's 1.2 s sting; the end card is held after the last line.

### 2. Tiny changes, exact values (locating plate)

| # | Scene | Start | End | On screen | Say |
|---|---|---|---|---|---|
| 1 | hook | 00:00:01.300 | 00:00:04.500 | Tiny changes are easy to miss by eye. | Tiny changes are easy to miss by eye. |
| 2 | action | 00:00:04.900 | 00:00:08.500 | Drop in both revisions. | Drop in both revisions. |
| 3 | reveal | 00:00:08.900 | 00:00:12.500 | 3 changes found between Rev B and Rev C. | 3 changes found between Rev B and Rev C. |
| 4 | proof | 00:00:12.900 | 00:00:16.000 | Circle Ø8 moved 0.5 mm right | Circle Ø8 moved 0.5 mm right |
| 5 | proof | 00:00:16.400 | 00:00:19.500 | Zoomed in close on that hole. | Zoomed in close on that hole. |
| 6 | proof | 00:00:19.900 | 00:00:24.000 | Details: moved 0.5 mm right. | Details: moved 0.5 mm right. |
| 7 | proof | 00:00:24.400 | 00:00:28.500 | Tolerance note: ±0.1 → ±0.05. | Tolerance note: ±0.1 → ±0.05. |
| 8 | payoff | 00:00:28.900 | 00:00:31.500 | Small changes, exact values. | Small changes, exact values. |

Video length 00:00:34.700 with the stage's 1.2 s sting; the end card is held after the last line.

### 3. Re-export noise ignored (drive shaft, simulated re-export)

| # | Scene | Start | End | On screen | Say |
|---|---|---|---|---|---|
| 1 | hook | 00:00:01.300 | 00:00:04.500 | Simulated re-export of Rev D as Rev E. | A simulated re-export of Rev D. What changed? |
| 2 | action | 00:00:04.900 | 00:00:08.500 | Drop in Rev D and Rev E. | Drop in Rev D and Rev E. |
| 3 | reveal | 00:00:08.900 | 00:00:13.500 | 1 change found. | 1 change found. New handles, shuffled order, older DXF version. |
| 4 | proof | 00:00:13.900 | 00:00:18.500 | The one real change: Ra 1.6 → Ra 0.8. | The one real change: Ra 1.6 → Ra 0.8. |
| 5 | proof | 00:00:18.900 | 00:00:23.500 | 17 objects redrawn the same: not changes. | 17 objects redrawn the same: not changes. |
| 6 | payoff | 00:00:23.900 | 00:00:26.500 | Real changes only, not file noise. | Real changes only, not file noise. |

Video length 00:00:29.700 with the stage's 1.2 s sting; the end card is held after the last line.

### 4. One block edit, every door (apartment plan)

| # | Scene | Start | End | On screen | Say |
|---|---|---|---|---|---|
| 1 | hook | 00:00:01.300 | 00:00:04.500 | A block edit changes every copy of it. | A block edit changes every copy of it. |
| 2 | action | 00:00:04.900 | 00:00:08.500 | Drop in both revisions. | Drop in both revisions. |
| 3 | reveal | 00:00:08.900 | 00:00:12.500 | 4 changes found between Rev A and Rev B. | 4 changes found between Rev A and Rev B. |
| 4 | proof | 00:00:12.900 | 00:00:18.000 | Door block edited: shown at all 7 doors. | Door block edited: shown at all 7 doors. |
| 5 | proof | 00:00:18.400 | 00:00:23.000 | On layer WALLS: line moved 300 mm right. | On layer WALLS: line moved 300 mm right. |
| 6 | proof | 00:00:23.400 | 00:00:28.000 | Before and after, side by side. | Before and after, side by side. |
| 7 | payoff | 00:00:28.400 | 00:00:31.000 | One block edit, shown wherever it's used. | One block edit, shown wherever it's used. |

Video length 00:00:34.200 with the stage's 1.2 s sting; the end card is held after the last line.

### 5. Big drawings, compared in your browser (warehouse layout)

| # | Scene | Start | End | On screen | Say |
|---|---|---|---|---|---|
| 1 | hook | 00:00:01.300 | 00:00:04.500 | A large layout, two revisions. What changed? | A large layout, two revisions. What changed? |
| 2 | action | 00:00:04.900 | 00:00:09.500 | Drop in both. Read and compared in this browser. | Drop in both. Read and compared in this browser. |
| 3 | reveal | 00:00:09.900 | 00:00:14.500 | 3 changes found among 23,182 objects. | 3 changes found among 23,182 objects. |
| 4 | proof | 00:00:14.900 | 00:00:18.500 | Read and compared in seconds. | Read and compared in seconds, on the machine shown. |
| 5 | proof | 00:00:18.900 | 00:00:23.000 | Requests logged since the files opened: [as shown on screen]. | Requests the browser logged after the files opened: [as shown on screen]. |
| 6 | proof | 00:00:23.400 | 00:00:27.500 | Text changed: R1-065 → QC-01. | Text changed: R1-065 → QC-01. |
| 7 | payoff | 00:00:27.900 | 00:00:30.500 | Big drawings, compared in your browser. | Big drawings, compared in your browser. |

Video length 00:00:33.700 with the stage's 1.2 s sting; the end card is held after the last line.

The large layout's request count is measured as the video plays: say the number on screen.
