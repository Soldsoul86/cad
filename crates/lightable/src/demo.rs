//! The demo as a plan: what happens when, and what is said or shown. The
//! app plays it (`?demo`, `?demo&narration`); the same plan gives the
//! subtitle files and the voice-over cue sheet (`lightable demo-cues`), so
//! their timings cannot drift apart. Every number and headline on screen
//! comes from the comparison.

use crate::compare::Compare;

/// What the demo does at a moment. The app sets the scene; everything a
/// viewer sees a person do is a [`Gesture`], performed through the app's
/// normal input (by `tools/demo.mjs` with browser input events, or, playing
/// live, by the app feeding the same events to its own input).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Act {
    /// Rev C open on its own, as it came in.
    Opening,
    /// The example's two revisions compared (as the app opens), for the
    /// feature videos that start there.
    Compared,
    /// Something on screen highlighted for 2.7 s (a target's name).
    Highlight(&'static str),
    /// The two files are dragged in and dropped (a gesture).
    Drop,
    /// A pinch-zoom on the revision cloud (a gesture).
    Cloud,
    /// A tap on change `i` in the list (a gesture).
    Tap(usize),
    /// A click on "Side by side" (a gesture).
    SideBySide,
    /// Clicks on "Details", then "Export PNG report" (gestures); the
    /// exported report stays on screen.
    Report,
    /// The same comparison in a phone outline, laid out as on a phone.
    Phone,
    /// The end card.
    EndCard,
    /// A pinch-zoom close onto change `i` (a gesture), to see a small one.
    Close(usize),
    /// A caption only.
    Nothing,
}

/// What a gesture is aimed at. The app publishes where each target is on
/// screen ([`Target::name`]); the pointer moves there before it acts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Target {
    /// Drag files in from the edge and drop them on the drawing.
    Drop(Files),
    /// Pinch-zoom (ctrl + wheel) on the revision cloud, by the factor that
    /// fits it on screen.
    Pinch,
    /// Click change `i` in the list.
    Row(usize),
    SideBySide,
    Details,
    ExportPng,
}

/// The files a drop gesture drops.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Files {
    /// The example's two files.
    Example,
    /// The large layout, from disk (out/demo/usecases, made by
    /// tools/gen_usecases.py; too big to build into the app).
    Big,
}

impl Files {
    /// The files: name, bytes, and the example file they copy (for the
    /// recorder, which drops files from disk).
    pub fn files(
        &self,
        e: &crate::examples::Example,
    ) -> Vec<(&'static str, &'static [u8], Option<&'static str>)> {
        match self {
            Files::Example => vec![
                (e.a_name, e.a, Some(e.a_name)),
                (e.b_name, e.b, Some(e.b_name)),
            ],
            Files::Big => vec![
                ("depot-rev-a.dxf", &[], Some("usecases/depot-rev-a.dxf")),
                ("depot-rev-b.dxf", &[], Some("usecases/depot-rev-b.dxf")),
            ],
        }
    }
}

impl Target {
    /// Where the app publishes this target ("drop" for any drop).
    pub fn on_screen(&self) -> String {
        match self {
            Target::Drop(_) => "drop".into(),
            t => t.name(),
        }
    }

    pub fn name(&self) -> String {
        match self {
            Target::Drop(Files::Example) => "drop:example".into(),
            Target::Drop(Files::Big) => "drop:big".into(),
            Target::Pinch => "pinch".into(),
            Target::Row(i) => format!("row:{i}"),
            Target::SideBySide => "side_by_side".into(),
            Target::Details => "details".into(),
            Target::ExportPng => "export_png".into(),
        }
    }
}

/// A person's action from `from`: acts at `at` (click, drop, or the pinch
/// starts) and, for a pinch, zooms until `until`. The pointer moves to the
/// target in the last 0.8 s before a click (first scrolling the list where
/// a row is out of view), and over the whole time for a drag or a pinch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gesture {
    pub from: f64,
    pub at: f64,
    pub until: f64,
    pub target: Target,
}

/// What the screen shows at the end of a step: its caption must be about
/// this (the recorder checks it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// The start screen, nothing open.
    Start,
    /// One drawing open on its own.
    Single,
    /// The overlay and the change list.
    Overlay,
    /// Both revisions side by side.
    SideBySide,
    /// Details open (exact values).
    Details,
    /// The PNG report.
    Report,
    /// The phone outline.
    Phone,
    /// The end card.
    End,
}

impl View {
    pub fn name(&self) -> &'static str {
        match self {
            View::Start => "start",
            View::Single => "single",
            View::Overlay => "overlay",
            View::SideBySide => "side_by_side",
            View::Details => "details",
            View::Report => "report",
            View::Phone => "phone",
            View::End => "end",
        }
    }
}

/// One moment of the demo.
#[derive(Debug, Clone, PartialEq)]
pub struct Shot {
    /// Seconds from the start of the demo.
    pub at: f64,
    pub act: Act,
    /// The on-screen caption from here on (`None`: unchanged).
    pub caption: Option<String>,
    /// What the screen shows at the end of the step (a step's first shot).
    pub view: Option<View>,
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
    pub gestures: Vec<Gesture>,
    pub cues: Vec<Cue>,
    /// Total length, the end card held at the end.
    pub length: f64,
    /// Voice-over timing: no captions in the picture, the voice carries
    /// the words (the cues are the voice-over lines).
    pub narration: bool,
}

/// The end card's text, as on screen.
pub const END_CARD: &str = "Free. Open source. Your files never leave your computer. lightable.dev";

/// How long a caption needs to be read: 3 words a second, plus a second;
/// at least 3 seconds (used by the app's tests of caption lengths).
#[cfg_attr(not(test), allow(dead_code))]
pub fn caption_seconds(caption: &str) -> f64 {
    (caption.split_whitespace().count() as f64 / 3.0 + 1.0).max(3.0)
}

/// The voice-over for the bracket example, one line per step
/// (`docs/demo/voiceover.md`). Every fact is checked against the
/// comparison by `the_voiceover_matches_the_comparison`.
pub const VOICEOVER: [(&str, &str); 9] = [
    (
        "Opening",
        "Rev C of this mounting bracket just came in. What changed?",
    ),
    (
        "Drop",
        "Drop both revisions on the page. Nothing to install, and nothing is uploaded.",
    ),
    (
        "Overlay",
        "Lightable finds ten changes. Blue is added, red deleted, yellow changed.",
    ),
    (
        "Cloud",
        "The revision cloud marks one of them, the bolt note. In this example, the other nine weren't clouded.",
    ),
    (
        "Changes",
        "Every change is in plain words: the holes grew from 13.5 to 17.5 millimetres, and the edge dimensions from 20 to 25. Tap one to zoom to it.",
    ),
    ("Side by side", "See before and after, side by side."),
    ("Report", "Then share a review report."),
    ("Phone", "It works on your phone too."),
    (
        "End card",
        "Lightable. Free, open source, and your files never leave your computer. lightable.dev",
    ),
];

/// Speaking pace for the voice-over, words a second.
const SPEAK: f64 = 2.5;
/// Before a line starts.
const LEAD: f64 = 0.5;
/// After a line ends, before the next step.
const SETTLE: f64 = 0.7;

fn words(s: &str) -> usize {
    s.split_whitespace().count()
}

/// When the word starting `phrase` is spoken in `line`, from the line's
/// start (words before it at the speaking pace).
fn spoken_at(line: &str, phrase: &str) -> f64 {
    let before = line.find(phrase).map_or(0, |i| words(&line[..i]));
    LEAD + before as f64 / SPEAK
}

/// A step of the demo: what happens, for how long (normal timing), the
/// caption, and its gestures (`from`, `at`, `until` in seconds into the
/// step).
struct Step {
    act: Act,
    seconds: f64,
    caption: String,
    gestures: Vec<(f64, f64, f64, Target)>,
    /// Later acts within the step (seconds into it), with a new caption
    /// or none.
    then: Vec<(f64, Act, Option<String>)>,
    /// What the screen shows at the step's end, when not what its act
    /// implies (a click inside the step changes the view).
    view: Option<View>,
}

impl Step {
    fn shows(mut self, view: View) -> Step {
        self.view = Some(view);
        self
    }
}

/// What a step's act leaves on screen.
fn view_after(act: Act) -> View {
    match act {
        Act::Opening => View::Single,
        Act::SideBySide => View::SideBySide,
        Act::Report => View::Report,
        Act::Phone => View::Phone,
        Act::EndCard => View::End,
        _ => View::Overlay,
    }
}

fn step(act: Act, seconds: f64, caption: String) -> Step {
    Step {
        act,
        seconds,
        caption,
        gestures: Vec::new(),
        then: Vec::new(),
        view: None,
    }
}

fn click(from: f64, at: f64, target: Target) -> (f64, f64, f64, Target) {
    (from, at, at, target)
}

/// The demo for this comparison (the built-in example the demo drops in):
/// about 40 seconds; narration timing when asked and the comparison is the
/// bracket example the voice-over is written for.
pub fn plan(c: &Compare, narration: bool) -> Plan {
    let narration = narration && c.example == Some("bracket") && c.demo_picks().len() >= 2;
    let (_, b) = c.labels();
    // Labels kept on one line in captions ("Rev C", not "Rev" / "C").
    let b = b.replace(' ', "\u{a0}");
    let changes = |n: usize| match n {
        1 => "1 change".to_owned(),
        n => format!("{n} changes"),
    };
    let n = c.design_count();
    let picks = c.demo_picks();
    let mut steps = vec![
        step(
            Act::Opening,
            3.0,
            format!("{b} just came in. What changed?"),
        ),
        Step {
            gestures: vec![(0.3, 1.8, 1.8, Target::Drop(Files::Example))],
            ..step(
                Act::Drop,
                4.0,
                "Drop in two revisions. No install, no upload.".into(),
            )
        },
        step(
            Act::Highlight("legend"),
            4.0,
            // "found" is in the summary line right above.
            format!(
                "{}. Blue added, red deleted, yellow changed.",
                capital(&changes(n))
            ),
        ),
    ];
    steps.push(match c.cloud_bounds() {
        Some(_) => Step {
            gestures: vec![(0.0, 0.8, 2.0, Target::Pinch)],
            ..step(Act::Cloud, 5.0, cloud_caption(c))
        },
        None => step(Act::Nothing, 5.0, format!("{b} has no revision cloud.")),
    });
    let tap = "Every change in plain words. Tap to zoom.".to_owned();
    steps.push(match picks.as_slice() {
        [first, second, ..] => Step {
            gestures: vec![
                click(0.0, 0.8, Target::Row(*first)),
                // Time to scroll the list first where the row is out of
                // view (a phone's short list).
                click(3.0, 4.8, Target::Row(*second)),
            ],
            then: vec![(3.0, Act::Tap(*second), None)],
            ..step(Act::Tap(*first), 8.0, tap)
        },
        [only] => Step {
            gestures: vec![click(0.0, 0.8, Target::Row(*only))],
            ..step(Act::Tap(*only), 8.0, tap)
        },
        [] => step(Act::Nothing, 8.0, tap),
    });
    steps.push(Step {
        gestures: vec![click(0.0, 0.7, Target::SideBySide)],
        ..step(
            Act::SideBySide,
            4.0,
            "Before and after, side by side.".into(),
        )
    });
    steps.push(Step {
        gestures: vec![
            click(0.0, 0.6, Target::Details),
            click(0.8, 1.5, Target::ExportPng),
        ],
        ..step(Act::Report, 4.0, "Share a review report.".into())
    });
    steps.push(step(Act::Phone, 4.0, "Works on your phone too.".into()));
    steps.push(step(Act::EndCard, 4.0, END_CARD.into()));
    assemble(steps, narration)
}

/// Times the steps into a plan: normal timing, or voice-over timing (each
/// step long enough for its `VOICEOVER` line).
fn assemble(steps: Vec<Step>, narration: bool) -> Plan {
    let mut t = 0.0;
    let mut plan = Plan {
        shots: Vec::new(),
        gestures: Vec::new(),
        cues: Vec::new(),
        length: 0.0,
        narration,
    };
    for (k, mut s) in steps.into_iter().enumerate() {
        let mut seconds = s.seconds;
        if narration {
            let line = VOICEOVER[k].1;
            seconds = seconds.max(LEAD + words(line) as f64 / SPEAK + SETTLE);
            // The second tap as the dimension is mentioned.
            if s.gestures.len() == 2
                && let Act::Tap(_) = s.act
            {
                let at = spoken_at(line, "and the edge").max(s.gestures[0].1 + 3.0);
                s.gestures[1] = click(at - 1.8, at, s.gestures[1].3);
                s.then[0].0 = at - 1.8;
            }
            plan.cues.push(Cue {
                start: t + LEAD,
                end: t + LEAD + words(line) as f64 / SPEAK,
                text: line.to_owned(),
            });
        } else {
            // One cue per caption: the step's, then any later ones.
            let mut said: Vec<(f64, &str)> = vec![(0.0, s.caption.as_str())];
            said.extend(
                s.then
                    .iter()
                    .filter_map(|(at, _, c)| c.as_deref().map(|c| (*at, c))),
            );
            for (k, (at, text)) in said.iter().enumerate() {
                let end = said.get(k + 1).map_or(seconds, |n| n.0);
                plan.cues.push(Cue {
                    start: t + at,
                    end: t + end,
                    // Plain spaces in subtitle files.
                    text: text.replace('\u{a0}', " "),
                });
            }
        }
        let caption = if narration || s.act == Act::EndCard {
            None
        } else {
            Some(s.caption)
        };
        plan.shots.push(Shot {
            at: t,
            act: s.act,
            caption: caption.or(Some(String::new())),
            view: Some(s.view.unwrap_or(view_after(s.act))),
        });
        for (offset, act, caption) in s.then {
            plan.shots.push(Shot {
                at: t + offset,
                act,
                caption: caption.filter(|_| !narration),
                view: None,
            });
        }
        for (from, at, until, target) in s.gestures {
            plan.gestures.push(Gesture {
                from: t + from,
                at: t + at,
                until: t + until,
                target,
            });
        }
        t += seconds;
    }
    plan.length = t;
    plan
}

/// The demo page's use-case videos, each 20–30 s on its own example
/// drawing, each proving one capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Feature {
    /// Finds the changes the revision cloud leaves out (gearbox cover).
    Unclouded,
    /// Shows tiny changes with their exact values (locating plate).
    Tiny,
    /// Ignores re-export noise: one real change (drive shaft Rev E, saved by
    /// another program).
    Resave,
    /// A block definition edit shown at every reference (floor plan).
    Blocks,
    /// A large drawing compared in the browser, nothing uploaded (depot).
    Big,
}

impl Feature {
    pub const ALL: [Feature; 5] = [
        Feature::Unclouded,
        Feature::Tiny,
        Feature::Resave,
        Feature::Blocks,
        Feature::Big,
    ];

    /// The name in `?feature=` and in file names.
    pub fn id(&self) -> &'static str {
        match self {
            Feature::Unclouded => "unclouded",
            Feature::Tiny => "tiny",
            Feature::Resave => "resave",
            Feature::Blocks => "blocks",
            Feature::Big => "big",
        }
    }

    pub fn from_id(id: &str) -> Option<Feature> {
        Feature::ALL.into_iter().find(|f| f.id() == id)
    }

    /// The built-in example it uses (the large layout is not built in: its
    /// files are dropped from disk, `Files::Big`).
    pub fn example(&self) -> &'static str {
        match self {
            Feature::Unclouded => "cover",
            Feature::Tiny => "plate",
            Feature::Resave => "shaft",
            Feature::Blocks => "flat",
            Feature::Big => "",
        }
    }

    /// Videos that start on the example's comparison, as the app opens.
    pub fn starts_compared(&self) -> bool {
        matches!(self, Feature::Tiny | Feature::Blocks)
    }

    /// Videos that start on the empty start screen.
    pub fn starts_empty(&self) -> bool {
        matches!(self, Feature::Big)
    }
}

/// What changed in a text, in its own words: the differing words of the
/// old and new values, with the word before a changed number ("Ra 1.6 →
/// Ra 0.8", "±0.1 → ±0.05").
pub fn text_change(c: &lightable_diff::Change) -> Option<String> {
    let d = c
        .details
        .iter()
        .find(|d| d.starts_with("text changed from "))?;
    let rest = d.strip_prefix("text changed from \"")?;
    let (old, new) = rest.split_once("\" to \"")?;
    let new = new.strip_suffix('"')?;
    let (a, b): (Vec<&str>, Vec<&str>) = (old.split(' ').collect(), new.split(' ').collect());
    let front = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let back = a[front..]
        .iter()
        .rev()
        .zip(b[front..].iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let mut from = front;
    // A bare number reads better with the word before it.
    if from > 0 && a[from].parse::<f64>().is_ok() {
        from -= 1;
    }
    Some(format!(
        "{} → {}",
        a[from..a.len() - back].join(" "),
        b[from..b.len() - back].join(" ")
    ))
}

/// The change's headline, said for its layer: "On layer SECTION: line
/// moved 1.25 mm up."
fn on_layer(c: &Compare, i: usize) -> String {
    let ch = &c.diff().changes[i];
    let mut h = ch.headline.clone();
    if let Some(first) = h.get(..1) {
        h = first.to_lowercase() + &h[1..];
    }
    format!("On layer {}: {h}.", ch.layer())
}

/// The first design change matching `test`.
fn find(c: &Compare, test: impl Fn(&lightable_diff::Change) -> bool) -> Option<usize> {
    c.diff()
        .changes
        .iter()
        .position(|ch| !ch.markup && test(ch))
}

/// Thousands with a thin space: "23 185".
pub fn count(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (k, ch) in s.chars().enumerate() {
        if k > 0 && (s.len() - k).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// A use-case video for this comparison (its example). `c` is `None` for
/// the large layout until its files have been dropped and compared; its
/// numbers come in then (`{objects}`, `{seconds}`, `{requests}` are filled
/// in as the captions are shown, from what was measured).
pub fn feature_plan(c: Option<&Compare>, f: Feature) -> Plan {
    let end = step(Act::EndCard, 3.0, END_CARD.into());
    let changes = |n: usize| match n {
        1 => "1 change".to_owned(),
        n => format!("{n} changes"),
    };
    let steps = match (f, c) {
        (Feature::Big, c) => {
            // The first change in the list (the same before the files are
            // compared, so the plan's gestures do not change).
            let first = c.map_or(Some(0), |c| find(c, |_| true));
            vec![
                step(
                    Act::Opening,
                    3.0,
                    "Two revisions of a large warehouse layout.".into(),
                )
                .shows(View::Start),
                Step {
                    gestures: vec![(0.3, 1.8, 1.8, Target::Drop(Files::Big))],
                    ..step(
                        Act::Drop,
                        6.0,
                        "Drop in both. Read and compared in this browser.".into(),
                    )
                },
                step(
                    Act::Highlight("timing"),
                    5.0,
                    "{objects} objects read and compared in seconds.".into(),
                ),
                step(
                    Act::Highlight("network"),
                    5.0,
                    "Requests logged since the files opened: {requests}.".into(),
                ),
                match first {
                    Some(i) => Step {
                        gestures: vec![click(0.0, 0.8, Target::Row(i))],
                        ..step(
                            Act::Tap(i),
                            5.0,
                            c.map_or(String::new(), |c| {
                                text_change(&c.diff().changes[i]).map_or_else(
                                    || on_layer(c, i),
                                    |w| format!("Text changed: {w}."),
                                )
                            }),
                        )
                    },
                    None => step(Act::Nothing, 5.0, "Each change, in plain words.".into()),
                },
                end,
            ]
        }
        (_, None) => vec![end],
        (Feature::Unclouded, Some(c)) => {
            let (_, b) = c.labels();
            let (a, b2) = c.labels();
            let n = c.design_count();
            let mut v = vec![
                step(
                    Act::Opening,
                    3.0,
                    format!("{b} of the gearbox cover. What changed?"),
                ),
                Step {
                    gestures: vec![(0.3, 1.8, 1.8, Target::Drop(Files::Example))],
                    ..step(Act::Drop, 4.0, "Drop in both revisions.".into())
                },
                step(
                    Act::Highlight("summary"),
                    4.0,
                    format!("{} found between {a} and {b2}.", changes(n)),
                ),
            ];
            if c.cloud_bounds().is_some() {
                let k = c.clouded_count();
                v.push(Step {
                    gestures: vec![(0.0, 0.8, 2.0, Target::Pinch)],
                    ..step(
                        Act::Cloud,
                        5.0,
                        format!(
                            "The cloud marks {}; {} others weren't clouded.",
                            changes(k),
                            n - k
                        ),
                    )
                });
            }
            let picks = c.demo_picks();
            if let Some(&hole) = picks.first() {
                v.push(Step {
                    gestures: vec![click(0.0, 0.8, Target::Row(hole))],
                    ..step(
                        Act::Tap(hole),
                        5.0,
                        clauses(c.headline(hole)).swap_remove(0),
                    )
                });
            }
            if let Some(edge) = find(c, |ch| ch.layer() == "SECTION") {
                v.push(Step {
                    gestures: vec![click(0.0, 0.8, Target::Row(edge))],
                    ..step(Act::Tap(edge), 4.0, on_layer(c, edge))
                });
            }
            v.push(end);
            v
        }
        (Feature::Tiny, Some(c)) => {
            let mut v = vec![step(
                Act::Compared,
                4.0,
                format!("Locating plate: {}, all tiny.", changes(c.design_count())),
            )];
            if let Some(hole) = find(c, |ch| ch.entity_type == "CIRCLE") {
                let ch = &c.diff().changes[hole];
                let (_, exact) = lightable_diff::words::exact(ch, c.diff().units.as_deref());
                v.push(Step {
                    gestures: vec![click(0.0, 0.8, Target::Row(hole))],
                    ..step(Act::Tap(hole), 4.0, c.headline(hole).to_owned())
                });
                v.push(Step {
                    // (After the step has set what to zoom onto.)
                    gestures: vec![(0.3, 1.1, 2.3, Target::Pinch)],
                    ..step(
                        Act::Close(hole),
                        4.0,
                        "Zoomed in close on that hole.".into(),
                    )
                });
                v.push(
                    Step {
                        gestures: vec![click(0.0, 0.7, Target::Details)],
                        ..step(Act::Nothing, 5.0, format!("Details: {}.", lower(&exact)))
                    }
                    .shows(View::Details),
                );
            }
            if let Some(note) = find(c, |ch| ch.entity_type == "TEXT")
                && let Some(words) = text_change(&c.diff().changes[note])
            {
                v.push(
                    Step {
                        gestures: vec![click(0.0, 0.8, Target::Row(note))],
                        ..step(Act::Tap(note), 5.0, format!("Tolerance note: {words}."))
                    }
                    .shows(View::Details),
                );
            }
            v.push(end);
            v
        }
        (Feature::Resave, Some(c)) => {
            let d = c.diff();
            let mut v = vec![
                step(
                    Act::Opening,
                    3.0,
                    "Rev E was saved by another program.".into(),
                ),
                Step {
                    gestures: vec![(0.3, 1.8, 1.8, Target::Drop(Files::Example))],
                    ..step(Act::Drop, 4.0, "Drop in Rev D and Rev E.".into())
                },
                Step {
                    then: vec![(
                        2.5,
                        Act::Nothing,
                        Some("New handles, new order, R2000: not changes.".into()),
                    )],
                    ..step(
                        Act::Highlight("summary"),
                        5.0,
                        format!("{} found.", capital(&changes(c.design_count()))),
                    )
                },
            ];
            if let Some(i) = find(c, |_| true) {
                let words = text_change(&d.changes[i]).unwrap_or_else(|| c.headline(i).into());
                v.push(Step {
                    gestures: vec![click(0.0, 0.8, Target::Row(i))],
                    ..step(Act::Tap(i), 5.0, format!("The one real change: {words}."))
                });
            }
            v.push(
                Step {
                    gestures: vec![click(0.0, 0.7, Target::Details)],
                    ..step(
                        Act::Highlight("redrawn"),
                        5.0,
                        format!("{} objects redrawn the same: not changes.", d.redrawn.len()),
                    )
                }
                .shows(View::Details),
            );
            v.push(end);
            v
        }
        (Feature::Blocks, Some(c)) => {
            let d = c.diff();
            let mut v = vec![step(
                Act::Compared,
                4.0,
                format!(
                    "Apartment plan, Rev A to Rev B: {}.",
                    changes(c.design_count())
                ),
            )];
            if let Some(i) = find(c, |ch| matches!(ch.place, lightable_diff::Place::Block(_))) {
                let refs = d.changes[i].used_by_new.len();
                v.push(Step {
                    gestures: vec![click(0.0, 0.8, Target::Row(i))],
                    ..step(
                        Act::Tap(i),
                        6.0,
                        format!("Door block edited: shown at all {refs} doors."),
                    )
                });
            }
            if let Some(i) = find(c, |ch| ch.layer() == "WALLS") {
                v.push(Step {
                    gestures: vec![click(0.0, 0.8, Target::Row(i))],
                    ..step(Act::Tap(i), 5.0, on_layer(c, i))
                });
            }
            v.push(Step {
                gestures: vec![click(0.0, 0.7, Target::SideBySide)],
                ..step(
                    Act::SideBySide,
                    5.0,
                    "Before and after, side by side.".into(),
                )
            });
            v.push(end);
            v
        }
    };
    assemble(steps, false)
}

fn lower(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| {
        f.to_lowercase().collect::<String>() + c.as_str()
    })
}

/// A headline as captions of at most one clause each: "Circle diameter
/// changed from 13.5 mm to 17.5 mm; centre moved 5 mm right and 5 mm up"
/// becomes two (the words unchanged, the second capitalised).
pub fn clauses(headline: &str) -> Vec<String> {
    headline
        .split("; ")
        .enumerate()
        .map(|(k, c)| if k == 0 { c.to_owned() } else { capital(c) })
        .collect()
}

/// The caption at the cloud. Only the built-in example whose cloud was
/// drawn around one change on purpose may say the others weren't clouded:
/// Lightable does not judge what a cloud is meant to cover.
fn cloud_caption(c: &Compare) -> String {
    let by_design = c
        .example
        .is_some_and(|id| crate::examples::find(id).cloud_by_design);
    if !by_design {
        return "The revision cloud, up close.".into();
    }
    let (clouded, n) = (c.clouded_count(), c.design_count());
    let marks = match clouded {
        1 => "1 change".to_owned(),
        k => format!("{k} changes"),
    };
    match n.saturating_sub(clouded) {
        0 => format!("The cloud marks {marks}."),
        1 => format!("The cloud marks {marks}. The other 1 wasn't clouded."),
        rest => format!("The cloud marks {marks}. The other {rest} weren't clouded."),
    }
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| {
        f.to_uppercase().collect::<String>() + c.as_str()
    })
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

    fn compare(id: &str, as_example: bool) -> Compare {
        let e = crate::examples::find(id);
        let read = |b: &[u8]| Arc::new(lightable_dxf::read_bytes(b).unwrap().0);
        Compare::new(
            None,
            (e.a_name.into(), read(e.a)),
            (e.b_name.into(), read(e.b)),
            Some((e.a_label.into(), e.b_label.into())),
            as_example.then_some(e.id),
        )
        .unwrap()
    }

    fn bracket() -> Compare {
        compare("bracket", true)
    }

    /// The 40-second demo: every step at its time, with its caption, and a
    /// person's gesture wherever something is done.
    #[test]
    fn the_demo_follows_the_timeline() {
        let c = bracket();
        let p = plan(&c, false);
        let starts: Vec<(f64, &str)> = p
            .cues
            .iter()
            .map(|cue| (cue.start, cue.text.as_str()))
            .collect();
        assert_eq!(
            starts,
            [
                (0.0, "Rev C just came in. What changed?"),
                (3.0, "Drop in two revisions. No install, no upload."),
                (7.0, "10 changes. Blue added, red deleted, yellow changed."),
                (
                    11.0,
                    "The cloud marks 1 change. The other 9 weren't clouded."
                ),
                (16.0, "Every change in plain words. Tap to zoom."),
                (24.0, "Before and after, side by side."),
                (28.0, "Share a review report."),
                (32.0, "Works on your phone too."),
                (36.0, END_CARD),
            ]
        );
        assert!((p.length - 40.0).abs() < 1e-9);
        let targets: Vec<Target> = p.gestures.iter().map(|g| g.target).collect();
        let picks = c.demo_picks();
        assert_eq!(
            targets,
            [
                Target::Drop(Files::Example),
                Target::Pinch,
                Target::Row(picks[0]),
                Target::Row(picks[1]),
                Target::SideBySide,
                Target::Details,
                Target::ExportPng,
            ]
        );
        // Each gesture inside its step, the pointer moving before it acts,
        // and one at a time.
        for g in &p.gestures {
            assert!(g.from < g.at && g.at <= g.until, "{g:?}");
        }
        for pair in p.gestures.windows(2) {
            assert!(pair[0].until < pair[1].from, "{pair:?}");
        }
        // The two taps are the circle and the dimension, neither clouded.
        assert!(c.headline(picks[0]).starts_with("Circle diameter changed"));
        assert!(c.headline(picks[1]).starts_with("Dimension changed"));
        // Captions fit two lines in 9:16.
        assert!(
            p.cues
                .iter()
                .filter(|c| c.text != END_CARD)
                .all(|c| c.text.len() <= 72),
            "{:?}",
            p.cues
        );
    }

    /// "The other N weren't clouded" is said only of the built-in example
    /// drawn that way, never of other files (Lightable does not check
    /// clouds).
    #[test]
    fn only_the_example_built_that_way_says_the_others_were_not_clouded() {
        let p = plan(&bracket(), false);
        assert!(p.cues.iter().any(|c| c.text.contains("weren't clouded")));
        // The same drawings as the user's own files.
        let p = plan(&compare("bracket", false), false);
        assert!(
            p.cues.iter().all(|c| !c.text.contains("clouded")),
            "{:?}",
            p.cues
        );
        assert!(
            p.cues
                .iter()
                .any(|c| c.text == "The revision cloud, up close.")
        );
        // The floor plan has no cloud: no zoom to one.
        let p = plan(&compare("plan", true), false);
        assert!(
            p.cues
                .iter()
                .any(|c| c.text == "Rev B has no revision cloud.")
        );
        assert!(p.gestures.iter().all(|g| g.target != Target::Pinch));
    }

    /// Narration: each line at 2.5 words a second, half a second into its
    /// step, the step long enough to finish it; no captions in the picture.
    #[test]
    fn narration_steps_leave_time_to_speak_each_line() {
        let p = plan(&bracket(), true);
        assert!(p.narration);
        assert_eq!(p.cues.len(), VOICEOVER.len());
        for (cue, (_, line)) in p.cues.iter().zip(VOICEOVER.iter()) {
            assert_eq!(cue.text, *line);
            let spoken = cue.end - cue.start;
            assert!((spoken - words(line) as f64 / 2.5).abs() < 1e-9);
        }
        for pair in p.cues.windows(2) {
            assert!(pair[1].start - pair[0].end >= LEAD + SETTLE - 1e-9);
        }
        assert!(
            p.shots
                .iter()
                .all(|s| s.caption.as_deref().is_none_or(str::is_empty))
        );
        // The second tap as the dimensions are mentioned.
        let changes = &p.cues[4];
        let second = p.gestures[3];
        assert!(second.at > changes.start && second.at < changes.end);
        for pair in p.gestures.windows(2) {
            assert!(pair[0].until < pair[1].from, "{pair:?}");
        }
        assert!(p.length > p.cues.last().unwrap().end);
    }

    /// Other drawings have no voice-over: narration falls back to the
    /// normal demo.
    #[test]
    fn narration_only_for_the_bracket() {
        assert!(!plan(&compare("plan", true), true).narration);
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
        // "finds ten changes. Blue is added, red deleted, yellow changed."
        assert_eq!(c.design_count(), 10);
        assert!(line(2).contains("ten changes"));
        use lightable_diff::view as v;
        const {
            assert!(v::ADDED.2 > v::ADDED.0 && v::ADDED.2 > v::ADDED.1, "blue");
            assert!(
                v::DELETED.0 > v::DELETED.1 && v::DELETED.0 > v::DELETED.2,
                "red"
            );
            assert!(
                v::CHANGED.0 > 0xe0 && v::CHANGED.1 > 0xc0 && v::CHANGED.2 < 0x40,
                "yellow"
            );
        }
        // "The revision cloud marks one of them, the bolt note. In this
        // example, the other nine weren't clouded."
        assert_eq!(c.clouded_count(), 1);
        let clouded = c.clouded_headlines();
        assert!(clouded[0].contains("BOLTS: 4x M12") && clouded[0].contains("BOLTS: 4x M16"));
        assert!(crate::examples::find("bracket").cloud_by_design);
        assert!(line(3).contains("one of them") && line(3).contains("other nine"));
        assert!(line(3).contains("In this example"));
        // "the holes grew from 13.5 to 17.5 millimetres"
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
        }
        assert!(line(4).contains("holes grew from 13.5 to 17.5 millimetres"));
        // "and the edge dimensions from 20 to 25": two of them.
        let edges = d
            .design_changes()
            .filter(|x| x.entity_type == "DIMENSION")
            .filter(|x| x.headline.contains("from 20 mm to 25 mm"))
            .count();
        assert_eq!(edges, 2);
        assert!(line(4).contains("edge dimensions from 20 to 25"));
        assert!(line(8).contains("never leave your computer"));
    }

    /// The use-case videos: 20–30 s, short captions (two lines in 9:16),
    /// gestures one at a time, numbers from each example's comparison.
    #[test]
    fn use_case_videos_say_what_their_comparisons_found() {
        let mut said = std::collections::HashMap::new();
        for f in Feature::ALL {
            let p = if f == Feature::Big {
                feature_plan(None, f)
            } else {
                feature_plan(Some(&compare(f.example(), true)), f)
            };
            assert!((20.0..=30.0).contains(&p.length), "{f:?}: {}", p.length);
            for cue in p.cues.iter().filter(|c| c.text != END_CARD) {
                assert!(cue.text.len() <= 55, "{f:?}: {}", cue.text);
            }
            for pair in p.gestures.windows(2) {
                assert!(pair[0].until < pair[1].from, "{f:?} {pair:?}");
            }
            // Each step says what the screen shows at its end.
            assert!(p.shots.iter().any(|s| s.view.is_some()));
            said.insert(f, p.cues.into_iter().map(|c| c.text).collect::<Vec<_>>());
        }
        let has = |f: Feature, t: &str| said[&f].iter().any(|c| c == t);
        assert!(has(
            Feature::Unclouded,
            "14 changes found between Rev B and Rev C."
        ));
        assert!(has(
            Feature::Unclouded,
            "The cloud marks 1 change; 13 others weren't clouded."
        ));
        assert!(has(
            Feature::Unclouded,
            "Circle diameter changed from 11 mm to 13.5 mm"
        ));
        assert!(has(
            Feature::Unclouded,
            "On layer SECTION: line moved 1.25 mm up."
        ));
        assert!(has(Feature::Tiny, "Circle Ø8 moved 0.5 mm right"));
        assert!(has(Feature::Tiny, "Details: moved 0.5 mm right."));
        assert!(has(Feature::Tiny, "Tolerance note: ±0.1 → ±0.05."));
        assert!(has(Feature::Resave, "1 change found."));
        assert!(has(
            Feature::Resave,
            "The one real change: Ra 1.6 → Ra 0.8."
        ));
        assert!(has(
            Feature::Resave,
            "17 objects redrawn the same: not changes."
        ));
        assert!(has(
            Feature::Blocks,
            "Door block edited: shown at all 7 doors."
        ));
        assert!(has(
            Feature::Blocks,
            "On layer WALLS: line moved 300 mm right."
        ));
        assert!(has(
            Feature::Big,
            "{objects} objects read and compared in seconds."
        ));
    }

    /// The demo page (`web/demo/index.html`) states the comparison's own
    /// numbers and words, and its "not compared" list.
    #[test]
    fn the_demo_page_matches_the_comparison() {
        let page = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/demo/index.html"),
        )
        .unwrap();
        let c = bracket();
        let d = c.diff();
        assert!(
            page.contains(&format!("\"{}\"", c.summary())),
            "{}",
            c.summary()
        );
        assert!(page.contains(&format!("({} items)", d.markup_count())));
        assert!(page.contains(&format!(
            "That is 4 holes, 4 dimensions, the callout and the note: {} changes",
            c.design_count()
        )));
        let headlines: Vec<&str> = d.design_changes().map(|x| x.headline.as_str()).collect();
        assert!(
            headlines
                .iter()
                .any(|h| h.starts_with("Circle diameter changed from 13.5 mm to 17.5 mm"))
        );
        assert!(page.contains("from Ø13.5 to Ø17.5 mm"));
        for (a, b) in [
            ("4x Ø13.5 THRU", "4x Ø17.5 THRU"),
            ("BOLTS: 4x M12", "BOLTS: 4x M16"),
        ] {
            assert!(
                headlines.iter().any(|h| h.contains(a) && h.contains(b)),
                "{a}"
            );
            assert!(page.contains(a) && page.contains(b));
        }
        for want in [
            "from 20 mm to 25 mm",
            "from 120 mm to 110 mm",
            "from 60 mm to 50 mm",
        ] {
            assert!(headlines.iter().any(|h| h.contains(want)), "{want}");
        }
        // The not-compared list, without its per-file notes.
        for item in &d.not_compared {
            let general = item
                .replace(" (the same in both files)", "")
                .replace("other tables: ", "other tables (")
                .replace("APPID, UCS, VIEW, VPORT", "APPID, UCS, VIEW, VPORT)");
            assert!(page.contains(&general), "missing: {general}");
        }
        // DXF versions: the oldest and newest the reader takes.
        use lightable_dxf::Version;
        assert!(
            Version::from_acadver("AC1009").is_some() && Version::from_acadver("AC1032").is_some()
        );
        assert!(Version::from_acadver("AC1006").is_none());
        assert!(page.contains("<code>AC1009</code> to <code>AC1032</code>"));
        // No scripts on the page.
        assert!(!page.contains("<script"));
        // The large layout claims no more than its counter shows, and says
        // how to check the rest; a time names its machine.
        assert!(page.contains("Check it yourself") && page.contains("Resource Timing"));
        assert!(!page.contains("nothing is sent anywhere"));
        assert!(page.contains("MacBook Air with a 2-core Intel Core i3"));
        // The use cases' numbers, from their comparisons.
        let cover = compare("cover", true);
        assert_eq!((cover.design_count(), cover.clouded_count()), (14, 1));
        assert!(page.contains("made 14 changes") && page.contains("13 others weren't clouded"));
        let holes = cover
            .diff()
            .design_changes()
            .filter(|c| c.headline == "Circle diameter changed from 11 mm to 13.5 mm")
            .count();
        assert_eq!(holes, 8);
        assert!(page.contains("8 holes Ø11 → Ø13.5 mm"));
        let plate = compare("plate", true);
        assert_eq!(plate.design_count(), 3);
        assert!(page.contains("3 changes: a dowel hole moved 0.5 mm right"));
        let shaft = compare("shaft", true);
        assert_eq!((shaft.design_count(), shaft.diff().redrawn.len()), (1, 17));
        assert!(page.contains("The 17 objects drawn the same"));
        let flat = compare("flat", true);
        assert_eq!(flat.design_count(), 4);
        assert!(
            flat.diff()
                .changes
                .iter()
                .filter(|c| c.headline.starts_with("In block DOOR-800"))
                .all(|c| c.used_by_new.len() == 7)
        );
        assert!(page.contains("4 changes: the door block") && page.contains("all 7 doors"));
        // The large layout's size, as its generator recorded it.
        let depot = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/depot-inventory.json"),
        )
        .unwrap();
        assert!(depot.contains("\"23182 objects in Rev B"));
        assert_eq!(depot.matches("\"id\": \"E").count(), 3);
        assert!(page.contains("23,181 objects in Rev A, 23,182 in Rev B"));
    }

    #[test]
    fn subtitle_files_are_well_formed() {
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
