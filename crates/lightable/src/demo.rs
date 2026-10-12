//! The demo as a plan: what happens when, and what is said or shown. The
//! app plays it (`?demo`, `?demo&narration`); the same plan gives the
//! subtitle files and the voice-over cue sheet (`lightable demo-cues`), so
//! their timings cannot drift apart. Every number and headline on screen
//! comes from the comparison.

use crate::compare::Compare;

/// What the demo does at a moment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Act {
    /// Rev C alone, plain, filling the frame.
    Opening,
    /// Both revisions side by side, whole sheet.
    SideBySide,
    /// Side by side, zoomed close to the revision cloud.
    Cloud,
    /// The overlay, whole sheet, with the change list ("found N").
    Found,
    /// Select change `i` and zoom to it.
    Select(usize),
    /// The overlay, whole sheet, nothing selected.
    Overview,
    /// The report layout.
    Report,
    /// Back from the report, view unchanged.
    Scope,
    /// The end card.
    EndCard,
    /// Nothing (a caption only, or a step whose change does not exist).
    Nothing,
}

/// One moment of the demo.
#[derive(Debug, Clone, PartialEq)]
pub struct Shot {
    /// Seconds from the start of the demo.
    pub at: f64,
    pub act: Act,
    /// The on-screen caption from here on (`None`: no caption).
    pub caption: Option<String>,
    /// The caption is a change's headline (narration shows it only where
    /// the change list is hidden).
    pub change: bool,
}

/// A subtitle: text shown (or spoken) from `start` to `end` seconds.
#[derive(Debug, Clone, PartialEq)]
pub struct Cue {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub shots: Vec<Shot>,
    pub cues: Vec<Cue>,
    /// Total length, the end card held at the end.
    pub length: f64,
    /// The voice-over lines (narration mode), with their step names.
    pub narration: bool,
}

/// How long the end card is held in the normal demo.
const END_HOLD: f64 = 3.0;

/// The end card's text, as on screen.
pub const END_CARD: &str = "Free. Open source. Your files never leave your computer. lightable.dev";

/// How long a demo caption stays: reading time at 3 words a second, plus
/// a second for the zoom to settle; at least 3 seconds.
pub fn caption_seconds(caption: &str) -> f64 {
    (caption.split_whitespace().count() as f64 / 3.0 + 1.0).max(3.0)
}

/// The voice-over for the bracket example, one line per step
/// (`docs/demo/voiceover.md`). Every fact is checked against the
/// comparison by `the_voiceover_matches_the_comparison`.
pub const VOICEOVER: [(&str, &str); 7] = [
    ("Overview", "Rev C of this mounting bracket just came in."),
    (
        "Cloud zoom",
        "The revision cloud marks one change: the bolt note, M12 to M16.",
    ),
    ("Side by side", "Easy to check that and send it on."),
    (
        "Overlay",
        "But Lightable compares both revisions and finds ten changes.",
    ),
    (
        "Changes",
        "The four holes grew from 13.5 to 17.5 millimetres, and each moved 5 millimetres in from both edges. The edge and pitch dimensions changed too. None of them were clouded.",
    ),
    (
        "Report",
        "Click any change to see it, and export a review report.",
    ),
    (
        "End card",
        "Lightable. Free, open source, and your files never leave your browser. lightable.dev",
    ),
];

/// Speaking pace for the voice-over, words a second.
const SPEAK: f64 = 2.5;
/// Before a line starts: the view starts moving first.
const LEAD: f64 = 0.5;
/// After a line ends: the zoom settles before the next one.
const SETTLE: f64 = 0.7;
/// The end card held after the last line.
const NARRATION_HOLD: f64 = 1.0;

fn words(s: &str) -> usize {
    s.split_whitespace().count()
}

/// When the word starting `phrase` is spoken in `line`, from the line's
/// start (words before it at the speaking pace).
fn spoken_at(line: &str, phrase: &str) -> f64 {
    let before = line.find(phrase).map_or(0, |i| words(&line[..i]));
    LEAD + before as f64 / SPEAK
}

/// The demo for this comparison: narration timing when asked and the
/// comparison is the bracket example the voice-over is written for.
pub fn plan(c: &Compare, narration: bool) -> Plan {
    let picks = c.demo_picks();
    if narration && c.example == Some("bracket") && picks.len() == 3 {
        narration_plan(c, &picks)
    } else {
        normal_plan(c, &picks)
    }
}

/// A headline as captions of at most one clause each: "Circle diameter
/// changed from 13.5 mm to 17.5 mm; center moved 5 mm right and 5 mm up"
/// becomes two (the words unchanged, the second capitalised).
pub fn clauses(headline: &str) -> Vec<String> {
    headline
        .split("; ")
        .enumerate()
        .map(|(k, c)| if k == 0 { c.to_owned() } else { capital(c) })
        .collect()
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| {
        f.to_uppercase().collect::<String>() + c.as_str()
    })
}

fn normal_plan(c: &Compare, picks: &[usize]) -> Plan {
    let (a, b) = c.labels();
    let changes = |n: usize| match n {
        1 => "1 change".to_owned(),
        n => format!("{n} changes"),
    };
    // (act, caption, a change's headline)
    let mut steps: Vec<(Act, String, bool)> = vec![
        (
            Act::Opening,
            format!("{b} just came in. What changed?"),
            false,
        ),
        (Act::SideBySide, format!("Before: {a}. After: {b}."), false),
        match c.cloud_bounds() {
            Some(_) => (
                Act::Cloud,
                format!("The revision cloud marks {}.", changes(c.clouded_count())),
                false,
            ),
            None => (Act::Nothing, format!("{b} has no revision cloud."), false),
        },
        (
            Act::Found,
            format!("Lightable found {}.", changes(c.design_count())),
            false,
        ),
    ];
    // Two changes the cloud does not mark, one clause per caption.
    for k in 0..2 {
        match picks.get(k) {
            Some(&i) => {
                for (n, clause) in clauses(c.headline(i)).into_iter().enumerate() {
                    let act = if n == 0 { Act::Select(i) } else { Act::Nothing };
                    steps.push((act, clause, true));
                }
            }
            None => steps.push((Act::Nothing, String::new(), false)),
        }
    }
    steps.push((
        Act::Report,
        "Download the review report as PNG or JSON.".into(),
        false,
    ));
    steps.push((
        Act::Scope,
        "Whether each change is right is still your call.".into(),
        false,
    ));
    let mut t = 0.0;
    let mut shots = Vec::new();
    let mut cues = Vec::new();
    for (act, caption, change) in steps {
        // The opening has no zoom to wait for: reading time only, 3 s.
        let seconds = if act == Act::Opening {
            (words(&caption) as f64 / 3.0).max(3.0)
        } else {
            caption_seconds(&caption)
        };
        if !caption.is_empty() {
            cues.push(Cue {
                start: t,
                end: t + seconds,
                text: caption.clone(),
            });
        }
        shots.push(Shot {
            at: t,
            act,
            caption: Some(caption),
            change,
        });
        t += seconds;
    }
    shots.push(Shot {
        at: t,
        act: Act::EndCard,
        caption: None,
        change: false,
    });
    cues.push(Cue {
        start: t,
        end: t + END_HOLD,
        text: END_CARD.into(),
    });
    Plan {
        shots,
        cues,
        length: t + END_HOLD,
        narration: false,
    }
}

/// `picks`: a changed circle, a changed dimension, a changed text, none of
/// them clouded (`Compare::demo_picks`).
fn narration_plan(c: &Compare, picks: &[usize]) -> Plan {
    let (circle, dimension, text) = (picks[0], picks[1], picks[2]);
    let mut t = 0.0;
    let mut shots = Vec::new();
    let mut cues = Vec::new();
    for (k, (_, line)) in VOICEOVER.iter().enumerate() {
        let start = t;
        // (seconds into the step, act, change caption)
        let beats: Vec<(f64, Act, Option<String>)> = match k {
            0 => vec![(0.0, Act::Opening, None)],
            1 => vec![(0.0, Act::Cloud, None)],
            2 => vec![(0.0, Act::SideBySide, None)],
            3 => vec![(0.0, Act::Found, None)],
            4 => {
                let edge = spoken_at(line, "The edge");
                let hole = clauses(c.headline(circle));
                let mut v = vec![(0.0, Act::Select(circle), hole.first().cloned())];
                // The hole's second clause halfway to the dimension.
                if let Some(second) = hole.get(1) {
                    v.push((edge / 2.0, Act::Nothing, Some(second.clone())));
                }
                v.push((
                    edge,
                    Act::Select(dimension),
                    Some(c.headline(dimension).to_owned()),
                ));
                v.push((spoken_at(line, "None of them"), Act::Overview, None));
                v
            }
            5 => vec![
                (0.0, Act::Select(text), Some(c.headline(text).to_owned())),
                (spoken_at(line, "and export"), Act::Report, None),
            ],
            _ => vec![(0.0, Act::EndCard, None)],
        };
        for (offset, act, caption) in beats {
            shots.push(Shot {
                at: start + offset,
                act,
                change: caption.is_some(),
                caption,
            });
        }
        let spoken = words(line) as f64 / SPEAK;
        cues.push(Cue {
            start: start + LEAD,
            end: start + LEAD + spoken,
            text: (*line).to_owned(),
        });
        t = start + LEAD + spoken + SETTLE;
    }
    Plan {
        shots,
        cues,
        length: t + NARRATION_HOLD,
        narration: true,
    }
}

// Used by `lightable demo-cues` (native).
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn clock(seconds: f64, separator: char) -> String {
    let ms = (seconds * 1000.0).round() as u64;
    format!(
        "{:02}:{:02}:{:02}{separator}{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}

/// SubRip subtitles.
// Used by `lightable demo-cues` (native).
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn srt(cues: &[Cue]) -> String {
    let mut out = String::new();
    for (n, c) in cues.iter().enumerate() {
        out.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            n + 1,
            clock(c.start, ','),
            clock(c.end, ','),
            c.text
        ));
    }
    out
}

/// WebVTT subtitles.
// Used by `lightable demo-cues` (native).
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn vtt(cues: &[Cue]) -> String {
    let mut out = String::from("WEBVTT\n\n");
    for c in cues {
        out.push_str(&format!(
            "{} --> {}\n{}\n\n",
            clock(c.start, '.'),
            clock(c.end, '.'),
            c.text
        ));
    }
    out
}

/// The voice-over cue sheet: each line's start and end in the narration
/// video (Markdown table).
// Used by `lightable demo-cues` (native).
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn cue_sheet(plan: &Plan) -> String {
    let mut out = String::from("| # | Step | Start | End | Line |\n|---|---|---|---|---|\n");
    for (n, (cue, (step, _))) in plan.cues.iter().zip(VOICEOVER.iter()).enumerate() {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            n + 1,
            step,
            clock(cue.start, '.'),
            clock(cue.end, '.'),
            cue.text
        ));
    }
    out.push_str(&format!(
        "\nVideo length {} (the end card is held after the last line).\n",
        clock(plan.length, '.')
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn bracket() -> Compare {
        let e = crate::examples::find("bracket");
        let read = |b: &[u8]| Arc::new(lightable_dxf::read_bytes(b).unwrap().0);
        Compare::new(
            None,
            (e.a_name.into(), read(e.a)),
            (e.b_name.into(), read(e.b)),
            Some((e.a_label.into(), e.b_label.into())),
            Some(e.id),
        )
        .unwrap()
    }

    /// The short demo: the opening, side by side, the cloud, the count, two
    /// changes (one clause per caption), the report, the scope line, a 3 s
    /// end card; about 35 s.
    #[test]
    fn the_normal_demo_is_short_and_has_no_details_step() {
        let p = plan(&bracket(), false);
        let acts: Vec<Act> = p.shots.iter().map(|s| s.act).collect();
        assert!(matches!(
            acts[..4],
            [Act::Opening, Act::SideBySide, Act::Cloud, Act::Found]
        ));
        assert_eq!(p.cues[0].text, "Rev C just came in. What changed?");
        assert!((p.cues[0].end - 3.0).abs() < 1e-9, "the opening lasts 3 s");
        assert_eq!(p.cues[2].text, "The revision cloud marks 1 change.");
        assert_eq!(p.cues[3].text, "Lightable found 10 changes.");
        assert_eq!(
            p.cues[4].text,
            "Circle diameter changed from 13.5 mm to 17.5 mm"
        );
        assert_eq!(p.cues[5].text, "Center moved 5 mm right and 5 mm up");
        assert_eq!(p.cues[6].text, "Dimension changed from 20 mm to 25 mm");
        assert!(p.length >= 30.0 && p.length <= 35.0, "{}", p.length);
        let end = p.cues.last().unwrap();
        assert_eq!(end.text, END_CARD);
        assert!((end.end - end.start - 3.0).abs() < 1e-9);
        // Each caption is one clause, short enough for two lines in 9:16.
        assert!(
            p.cues
                .iter()
                .filter(|c| c.text != END_CARD)
                .all(|c| c.text.len() <= 72),
            "{:?}",
            p.cues
        );
    }

    /// Narration: each line at 2.5 words a second, half a second after its
    /// step starts, and 0.7 s to settle after it; no on-screen captions.
    #[test]
    fn narration_steps_leave_time_to_speak_each_line() {
        let p = plan(&bracket(), true);
        assert!(p.narration);
        assert_eq!(p.cues.len(), VOICEOVER.len());
        for (cue, (_, line)) in p.cues.iter().zip(VOICEOVER.iter()) {
            let spoken = cue.end - cue.start;
            assert!((spoken - words(line) as f64 / 2.5).abs() < 1e-9);
        }
        for pair in p.cues.windows(2) {
            assert!(pair[1].start - pair[0].end >= LEAD + SETTLE - 1e-9);
        }
        // Captions only for changes (shown where the list is hidden).
        assert!(p.shots.iter().all(|s| s.caption.is_some() == s.change));
        assert_eq!(p.shots.iter().filter(|s| s.change).count(), 4);
        // The changes line moves on to the dimension as it is mentioned.
        let dimension = p
            .shots
            .iter()
            .find(|s| matches!(s.act, Act::Select(_)) && s.at > p.cues[4].start + 1.0)
            .unwrap();
        assert!(dimension.at > p.cues[4].start && dimension.at < p.cues[4].end);
        assert!(p.length > p.cues.last().unwrap().end);
    }

    /// Other drawings have no voice-over: narration falls back to the
    /// normal demo.
    #[test]
    fn narration_only_for_the_bracket() {
        let e = crate::examples::find("plan");
        let read = |b: &[u8]| Arc::new(lightable_dxf::read_bytes(b).unwrap().0);
        let c = Compare::new(
            None,
            (e.a_name.into(), read(e.a)),
            (e.b_name.into(), read(e.b)),
            Some((e.a_label.into(), e.b_label.into())),
            Some(e.id),
        )
        .unwrap();
        assert!(!plan(&c, true).narration);
    }

    /// Every fact the voice-over states, checked against the comparison.
    #[test]
    fn the_voiceover_matches_the_comparison() {
        let c = bracket();
        let d = c.diff();
        let line = |k: usize| VOICEOVER[k].1;
        // "Rev C of this mounting bracket": the title block.
        let b = lightable_dxf::read_bytes(crate::examples::find("bracket").b)
            .unwrap()
            .0;
        let texts: Vec<String> = b
            .entities
            .iter()
            .filter_map(|e| match &e.kind {
                lightable_dxf::EntityKind::Text(t) => Some(t.value.clone()),
                _ => None,
            })
            .collect();
        assert!(
            texts.iter().any(|t| t == "STEEL MOUNTING BRACKET")
                && texts.iter().any(|t| t == "REV C")
        );
        assert!(line(0).contains("Rev C") && line(0).contains("mounting bracket"));
        // "The revision cloud marks one change: the bolt note, M12 to M16."
        assert_eq!(c.clouded_count(), 1);
        let clouded = c.clouded_headlines();
        assert_eq!(clouded.len(), 1);
        assert!(clouded[0].contains("BOLTS: 4x M12") && clouded[0].contains("BOLTS: 4x M16"));
        assert!(line(1).contains("one change") && line(1).contains("M12 to M16"));
        // "finds ten changes"
        assert_eq!(c.design_count(), 10);
        assert!(line(3).contains("ten changes"));
        // "The four holes grew from 13.5 to 17.5 millimetres, and each moved
        // 5 millimetres in from both edges."
        let extent = c.bounds().unwrap();
        let mid = [
            (extent.min[0] + extent.max[0]) / 2.0,
            (extent.min[1] + extent.max[1]) / 2.0,
        ];
        let holes: Vec<_> = d
            .design_changes()
            .filter(|x| x.entity_type == "CIRCLE")
            .collect();
        assert_eq!(holes.len(), 4);
        for h in &holes {
            assert!(
                h.headline
                    .contains("diameter changed from 13.5 mm to 17.5 mm"),
                "{}",
                h.headline
            );
            let at = h.bounds.unwrap();
            let (x, y) = ((at.min[0] + at.max[0]) / 2.0, (at.min[1] + at.max[1]) / 2.0);
            // Towards the middle in both directions: in from both edges.
            let across = if x < mid[0] {
                "5 mm right"
            } else {
                "5 mm left"
            };
            let up_down = if y < mid[1] { "5 mm up" } else { "5 mm down" };
            assert!(
                h.headline.contains(across) && h.headline.contains(up_down),
                "{}",
                h.headline
            );
        }
        assert!(line(4).contains("four holes") && line(4).contains("13.5 to 17.5 millimetres"));
        assert!(line(4).contains("5 millimetres in from both edges"));
        // "The edge and pitch dimensions changed too." Edge 20 -> 25 (two),
        // pitch 120 -> 110 and 60 -> 50.
        let dims: Vec<&str> = d
            .design_changes()
            .filter(|x| x.entity_type == "DIMENSION")
            .map(|x| x.headline.as_str())
            .collect();
        for want in [
            "from 20 mm to 25 mm",
            "from 120 mm to 110 mm",
            "from 60 mm to 50 mm",
        ] {
            assert!(dims.iter().any(|h| h.contains(want)), "{want}");
        }
        // "None of them were clouded": only the bolt note is.
        assert!(!clouded[0].starts_with("Circle") && !clouded[0].starts_with("Dimension"));
        // "Click any change to see it, and export a review report." (Two
        // clicks for the export: Details, then Export.)
        assert!(!line(5).contains("one click"));
    }

    #[test]
    fn subtitle_files_are_well_formed() {
        assert_eq!(
            clauses("Circle diameter changed from 1 mm to 2 mm; center moved 5 mm up"),
            [
                "Circle diameter changed from 1 mm to 2 mm",
                "Center moved 5 mm up"
            ]
        );
        let cues = [Cue {
            start: 3.5,
            end: 62.25,
            text: "Hello".into(),
        }];
        assert_eq!(srt(&cues), "1\n00:00:03,500 --> 00:01:02,250\nHello\n\n");
        assert_eq!(
            vtt(&cues),
            "WEBVTT\n\n00:00:03.500 --> 00:01:02.250\nHello\n\n"
        );
    }
}
