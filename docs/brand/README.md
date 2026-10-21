# Lightable brand kit

One kit for every video and page: the five use-case videos use all of it,
the same way. Made by `tools/gen_brand.py` (the SVGs; Lato is copied in
unmodified). The stage that applies it to the videos is `tools/stage/`.

## Logo

| File | Use |
|---|---|
| `logo-mark.svg` | The mark alone (icons, the intro sting). |
| `wordmark.svg` | "lightable" in Lato Regular, outlined (no font needed). |
| `lockup.svg` | Mark and wordmark together (end card, video footer). |

The mark is a light table: two sheets laid over each other, the earlier
revision in blue, the later in yellow, and where they overlap the light
shines through in white. It is the app's own idea (compare by overlaying)
in the app's own colours. Keep it on dark backgrounds; leave clear space of
half the mark's width around it; never recolour, rotate or outline it.

## Colours

The app's change colours, on a light-table night.

| Name | Hex | Use |
|---|---|---|
| Ink | `#0b0f14` | Backgrounds, text on light cards. |
| Night | `#101924` | Background gradient top. |
| Deep | `#143049` | The glow under the window. |
| Paper | `#eef2f6` | Text, file cards. |
| Mist | `#9aa7b4` | Secondary text. |
| Signal blue | `#4cb0ff` | Brand accent; the app's "added". Pointer ripples, the drop's landing pulse, "lightable.dev". |
| Marker yellow | `#f5d02a` | The app's "changed". Key numbers in captions; the pulse around a highlighted change. |
| Red | `#f04848` | The app's "deleted". Never decorative. |
| Grey | `#6a6a6a` | The app's "unchanged". |

## Type

**Lato** (Regular 400 and Light 300, version 1.105, © Łukasz Dziedzic),
under the SIL Open Font Licence 1.1: `fonts/` with `fonts/OFL.txt`. Used
for titles and captions in the videos only; the app keeps its own fonts.
Lato has no "→": the stage draws the arrow as a small stroke icon.

## Captions

- Lato Regular, at least 4 % of the frame's height (4.5 % when it fits),
  at most two lines, one clause each.
- Paper on an ink pill (95 % opaque), radius 0.32 em; key numbers
  (measurements, counts, ±, Ø) in marker yellow.
- Placed where the app keeps the drawing clear, above the frame's bottom
  15 %; the frame's top-left 10 % holds no control.
- In: fade and slide up 0.55 em over 0.36 s when the step starts. The words
  are always the app's own (numbers as measured).

## Motion

- **Easing:** out-cubic for things arriving, in-out-cubic for the camera.
- **Pointer:** eased by the recorder, a 0.3 s pause on a control; lifts 12 %
  with a shadow while hovering, dips while pressed; a blue ripple (0.45 s)
  where it presses.
- **Camera:** the window settles in after the sting (96 % → 100 %), pushes
  in 3.5 % over 1.6 s for the reveal and the proof, eases back over 1 s for
  the payoff. The app's own zooms stay the app's.
- **Highlights:** a yellow double pulse (2 × 0.6 s) around a change the
  moment the app selects it.
- **Transitions:** sting → window: fade and scale (0.33 s). Window → end
  card: fade (0.42 s), the lines rising in one after another.

## Intro sting (1.2 s)

The two sheets slide in from either side (0.42 s), the overlap lights up
(0.33 s in), the wordmark rises in (0.42 s in), and the whole fades and
grows slightly into the window (from 0.87 s). A soft three-note chime.

## End card (3 s)

The lockup, then "Free. Open source.", "Your files are read in your
browser, never uploaded." and "lightable.dev" in signal blue: the app's own
end-card words, three lines rising in. A warm four-note chord.

## Sound

Our own (`tools/gen_sound.py`): a quiet bed (Cmaj7, Am7, Fmaj7, G6 at 84
bpm), and UI sounds for the sting, the pickup, the drop, a click, the
reveal and the end card, placed where the recording really did each. Every
video comes with music and without, plus separate stems for a voice-over.
