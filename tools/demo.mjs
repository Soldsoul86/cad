// The demo video pipeline: stills with automatic checks, draft videos and
// final videos, each published to the private review page (out/review/).
//
//   node tools/demo.mjs stills [--modes normal,narration] [--note "..."]
//   node tools/demo.mjs draft  [--modes normal,narration] [--note "..."]
//   node tools/demo.mjs final  [--modes normal,narration] [--note "..."]
//   node tools/demo.mjs approve --mode normal|narration
//
// `approve` copies the current FINAL videos of that mode (never drafts),
// their captions and two posters into web/demo/, where tools/build_web.sh
// picks them up for lightable.dev/demo/. Run it only for a final you have
// approved.
//
// Options: --formats landscape,vertical (default both), --base URL of the
// web build (default http://localhost:8765/target/web/index.html, served
// by tools/web_test_server.py), --no-review (do not publish).
//
// The app plays the demo on a frame clock the tool sets (`?demo&record`):
// demo time = frame / 30, and every zoom starts at its step's scheduled
// time, so any frame looks the same however it is reached. Stills jump
// straight to the moments that matter (each step settled, and mid-zoom), so
// a still set takes seconds. Videos are cut into segments at the demo's
// steps. A segment is cached under a hash of its timeline and settings (the
// steps up to it, format, quality); before it is reused, the current build
// draws three probe frames (start, middle, end) and they must match the
// cached ones exactly, so only steps that look different are recorded
// again. Segments are joined with ffmpeg. While the picture holds still (no
// animation, nothing changed) a frame is drawn once and repeated.
//
// Needs Chrome, ffmpeg (h264_videotoolbox for drafts if available, libx264
// for finals) and `target/release/lightable` (for the subtitles).
// Development only.

import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  copyFileSync, existsSync, linkSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";

const ROOT = resolve(new URL("..", import.meta.url).pathname);
const OUT = join(ROOT, "out", "demo");
const REVIEW = join(ROOT, "out", "review");
const TOOL_VERSION = 3;
const CLOCK = 30; // the app's frame clock

const args = process.argv.slice(2);
const command = args[0];
const opt = (name, fallback) => {
  const i = args.indexOf(`--${name}`);
  return i >= 0 ? args[i + 1] : fallback;
};
const formats = opt("formats", "landscape,vertical").split(",");
const modes = opt("modes", command === "stills" ? "normal,narration" : "normal,narration").split(",");
const base = opt("base", "http://localhost:8765/target/web/index.html");
const note = opt("note", "");
const review = !args.includes("--no-review");
if (command === "approve") {
  const mode = opt("mode", "");
  if (!["normal", "narration"].includes(mode)) {
    console.error("usage: node tools/demo.mjs approve --mode normal|narration");
    process.exit(2);
  }
  const web = join(ROOT, "web", "demo");
  const suffix = mode === "narration" ? "-narration" : "";
  for (const format of ["landscape", "vertical"]) {
    const src = join(OUT, `demo-${format}${suffix}.mp4`);
    if (!existsSync(src)) throw new Error(`no final video ${src}: run \`node tools/demo.mjs final\` first`);
    copyFileSync(src, join(web, `demo-${format}.mp4`));
    // Poster: the "found N changes" moment, a still from the video itself.
    spawnSync("ffmpeg", ["-v", "error", "-y", "-ss", mode === "narration" ? "19" : "11", "-i", src, "-frames:v", "1", "-q:v", "3",
      join(web, `poster-${format}.jpg`)]);
  }
  copyFileSync(join(OUT, mode === "narration" ? "demo-narration.vtt" : "demo.vtt"), join(web, "demo.vtt"));
  // Captions on by default only for the narration version: the normal one
  // has its captions in the picture.
  const page = join(web, "index.html");
  const html = readFileSync(page, "utf8").replaceAll(' label="English" default>', ' label="English">');
  writeFileSync(page, mode === "narration" ? html.replaceAll(' label="English">', ' label="English" default>') : html);
  console.log(`Approved ${mode} final copied to web/demo/; tools/build_web.sh will publish it at /demo/.`);
  process.exit(0);
}
if (!["stills", "draft", "final"].includes(command)) {
  console.error("usage: node tools/demo.mjs stills|draft|final [--modes normal,narration] [--formats landscape,vertical] [--note TEXT]");
  process.exit(2);
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const sha = (data) => createHash("sha256").update(data).digest("hex");
const FRAME = { landscape: [960, 540], vertical: [540, 960] };
const hasVideoToolbox = spawnSync("ffmpeg", ["-hide_banner", "-encoders"]).stdout.toString().includes("h264_videotoolbox");
const QUALITY = {
  stills: { dpr: 2 },
  draft: {
    dpr: 1, fps: 15,
    encode: hasVideoToolbox
      ? ["-c:v", "h264_videotoolbox", "-b:v", "3M", "-profile:v", "high", "-vf", "format=yuv420p"]
      : ["-c:v", "libx264", "-preset", "veryfast", "-crf", "23", "-pix_fmt", "yuv420p"],
  },
  final: {
    dpr: 2, fps: 30,
    encode: ["-c:v", "libx264", "-preset", "slow", "-tune", "animation", "-crf", "16", "-pix_fmt", "yuv420p"],
  },
};

// --- A browser page playing the demo on the recorder's clock -------------

async function openDemo(format, mode, dpr) {
  const port = 9350 + Math.floor(Math.random() * 400);
  const chrome = spawn("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", [
    `--remote-debugging-port=${port}`, `--user-data-dir=${mkdtempSync(join(tmpdir(), "demo-"))}`,
    "--no-first-run", "--no-default-browser-check", "--window-size=1400,1000",
    "--disable-background-timer-throttling", "--disable-renderer-backgrounding",
    "--disable-backgrounding-occluded-windows", "about:blank",
  ], { stdio: "ignore" });
  let target;
  for (let i = 0; i < 80 && !target; i++) {
    try {
      target = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find((t) => t.type === "page");
    } catch {}
    if (!target) await sleep(250);
  }
  const ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((r) => ws.addEventListener("open", r));
  let id = 0;
  const pending = new Map();
  const requests = [];
  const errors = [];
  ws.addEventListener("message", (m) => {
    const d = JSON.parse(m.data);
    if (pending.has(d.id)) {
      pending.get(d.id)(d.result ?? { error: d.error });
      pending.delete(d.id);
    }
    if (d.method === "Network.requestWillBeSent") {
      const r = d.params.request;
      requests.push({ method: r.method, url: r.url, body: r.postData ? r.postData.length : 0 });
    }
    if (d.method === "Runtime.exceptionThrown") errors.push(d.params.exceptionDetails.text);
    if (d.method === "Runtime.consoleAPICalled" && d.params.type === "error") {
      errors.push(d.params.args.map((a) => a.value ?? a.description).join(" "));
    }
  });
  const send = (method, params = {}) => new Promise((r) => {
    const n = ++id;
    pending.set(n, r);
    ws.send(JSON.stringify({ id: n, method, params }));
  });
  const evaluate = async (expression) =>
    (await send("Runtime.evaluate", { expression, returnByValue: true })).result?.value;
  const nudge = async () => {
    // A Shift tap (no visible effect) makes the app draw a frame.
    await send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "Shift", code: "ShiftLeft", windowsVirtualKeyCode: 16 });
    await send("Input.dispatchKeyEvent", { type: "keyUp", key: "Shift", code: "ShiftLeft", windowsVirtualKeyCode: 16 });
  };
  await send("Network.enable");
  await send("Runtime.enable");
  await send("Page.enable");
  await send("Page.bringToFront");
  const [w, h] = FRAME[format];
  await send("Emulation.setDeviceMetricsOverride", { width: w, height: h, deviceScaleFactor: dpr, mobile: false });
  const url = `${base}?demo&record&format=${format}${mode === "narration" ? "&narration" : ""}`;
  await send("Page.navigate", { url });
  for (let i = 0; i < 600; i++) {
    if (await evaluate("!document.getElementById('loading')")) break;
    await sleep(100);
  }
  await evaluate("window.__lightableFrame = 0");
  let length;
  for (let i = 0; i < 600 && !length; i++) {
    await nudge();
    length = await evaluate("window.__lightableDemoLength");
    if (!length) await sleep(100);
  }
  if (!length) throw new Error(`the demo did not start: ${url} ${errors.join(" | ")}`);
  const shots = JSON.parse(await evaluate("window.__lightableDemoShots"));
  let last = -1;
  return {
    url, length, shots, requests, errors, chrome,
    get last() {
      return last;
    },
    // Draws frame `f` (frames must go forward) and returns what is on screen.
    async render(f) {
      await evaluate(`window.__lightableFrame = ${f}`);
      for (let i = 0; i < 600; i++) {
        if (i % 20 === 0) await nudge();
        if ((await evaluate("window.__lightableRendered")) === f) {
          last = f;
          return JSON.parse(await evaluate("window.__lightableFrameInfo"));
        }
        await sleep(5);
      }
      throw new Error(`the app did not draw frame ${f}: ${errors.join(" | ")}`);
    },
    async png() {
      return Buffer.from((await send("Page.captureScreenshot", { format: "png" })).data, "base64");
    },
    close() {
      chrome.kill();
    },
  };
}

const actName = (desc) => desc.match(/act: (\w+)/)?.[1] ?? "Step";
const frameOf = (t) => Math.round(t * CLOCK);

// --- Automatic checks on a still ------------------------------------------

function inside(a, b, tol = 1) {
  return a[0] >= b[0] - tol && a[1] >= b[1] - tol && a[2] <= b[2] + tol && a[3] <= b[3] + tol;
}
function overlaps(a, b) {
  return a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3];
}
function check(info, format, sample) {
  const [x0, y0, x1, y1] = info.screen;
  const sh = y1 - y0;
  const out = {};
  if (info.caption) {
    const share = info.caption.size / sh;
    out.caption = {
      pass: share >= 0.0395 && info.caption.rows <= 2 && inside(info.caption.rect, info.screen),
      detail: `text ${(share * 100).toFixed(1)} % of the height, ${info.caption.rows} line(s)`,
    };
  } else {
    out.caption = { pass: null, detail: "no caption" };
  }
  if (sample === "mid") {
    out.cutoff = { pass: null, detail: "mid-zoom (not checked)" };
  } else if (info.focus.length && info.canvas) {
    const cut = info.focus.filter((f) => !inside(f, info.canvas) || !inside(f, info.screen));
    const covered = info.caption ? info.focus.filter((f) => overlaps(f, info.caption.rect)) : [];
    out.cutoff = {
      pass: cut.length === 0 && covered.length === 0,
      detail: cut.length ? "the change runs off the drawing area" : covered.length ? "the caption covers the change" : "change fully visible, clear of the caption",
    };
  } else {
    out.cutoff = { pass: null, detail: info.report ? "report layout" : info.end_card ? "end card" : "nothing in focus" };
  }
  if (info.list_tail != null) {
    out.list = { pass: info.list_tail, detail: info.list_tail ? "'… and N more' fully visible" : "the list is cut off before '… and N more'" };
  }
  if (format === "vertical") {
    if (info.canvas && !info.end_card) {
      const [c0, c1, c2, c3] = info.canvas;
      const share = ((c2 - c0) * (c3 - c1)) / ((x1 - x0) * sh);
      out.drawing = { pass: share >= 0.6, detail: `drawing ${(share * 100).toFixed(0)} % of the frame` };
    } else {
      out.drawing = { pass: null, detail: info.report ? "report layout" : "end card" };
    }
  }
  return out;
}

// --- Stills ---------------------------------------------------------------

async function stills(format, mode) {
  const dir = join(OUT, "stills", `${mode}-${format}`);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  const page = await openDemo(format, mode, QUALITY.stills.dpr);
  const samples = [];
  page.shots.forEach((s, i) => {
    const next = i + 1 < page.shots.length ? page.shots[i + 1].at : page.length;
    const act = actName(s.desc);
    const zooms = !["Nothing", "Scope", "EndCard", "Report"].includes(act);
    if (zooms && next - s.at > 1.2) samples.push({ t: s.at + 0.5, step: i, act, sample: "mid" });
    samples.push({ t: Math.max(s.at, next - 2 / CLOCK), step: i, act, sample: "settled" });
  });
  const results = [];
  let lastFrame = -1;
  for (const s of samples) {
    const f = Math.max(frameOf(s.t), lastFrame + 1);
    const info = await page.render(f);
    lastFrame = f;
    const name = `${String(results.length).padStart(2, "0")}-${s.act}-${s.sample}-${(f / CLOCK).toFixed(1)}s.png`;
    writeFileSync(join(dir, name), await page.png());
    results.push({ file: name, time: f / CLOCK, step: s.step, act: s.act, sample: s.sample, checks: check(info, format, s.sample) });
  }
  page.close();
  // Contact sheet of the settled stills, and a phone-sized copy.
  const settled = results.filter((r) => r.sample === "settled");
  const tmp = mkdtempSync(join(tmpdir(), "sheet-"));
  settled.forEach((r, k) => copyFileSync(join(dir, r.file), join(tmp, `s${String(k).padStart(2, "0")}.png`)));
  const cols = format === "landscape" ? 3 : 6;
  const rows = Math.ceil(settled.length / cols);
  const cell = format === "landscape" ? 640 : 300;
  spawnSync("ffmpeg", ["-v", "error", "-y", "-framerate", "1", "-i", join(tmp, "s%02d.png"),
    "-vf", `scale=${cell}:-1,tile=${cols}x${rows}:padding=6:color=gray`, "-frames:v", "1", join(dir, "contact.png")]);
  spawnSync("ffmpeg", ["-v", "error", "-y", "-i", join(dir, "contact.png"), "-vf", "scale=390:-1", join(dir, "contact-phone.png")]);
  rmSync(tmp, { recursive: true, force: true });
  writeFileSync(join(dir, "checks.json"), JSON.stringify(results, null, 1));
  return { dir, results, requests: page.requests, errors: page.errors, url: page.url };
}

// --- Videos: segments, cache, holds ---------------------------------------

// The pixels of a PNG as RGB bytes (ffmpeg decodes).
function pixels(png) {
  const r = spawnSync("ffmpeg", ["-v", "error", "-i", "-", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"],
    { input: png, maxBuffer: 1 << 28 });
  return r.stdout;
}

// Whether a freshly drawn frame matches a cached probe: the same picture up
// to rendering noise (no channel of any pixel more than 16 levels apart). A
// real change (a character, a line, a colour) moves pixels much further.
function alike(png, cachedFile) {
  if (!existsSync(cachedFile)) return false;
  const cached = readFileSync(cachedFile);
  if (sha(png) === sha(cached)) return true;
  const [a, b] = [pixels(png), pixels(cached)];
  if (a.length !== b.length || a.length === 0) return false;
  for (let i = 0; i < a.length; i++) if (Math.abs(a[i] - b[i]) > 16) return false;
  return true;
}

async function video(format, mode, kind) {
  const q = QUALITY[kind];
  const step = CLOCK / q.fps;
  const cacheDir = join(OUT, "cache");
  mkdirSync(cacheDir, { recursive: true });
  // The plan (steps and length) comes from the app; open it once to read it.
  let page = await openDemo(format, mode, q.dpr);
  const { shots, length } = page;
  const total = Math.ceil((length * CLOCK) / step) * step;
  const align = (f) => Math.round(f / step) * step;
  // Segments start at steps; a step shorter than 1.5 s joins the one before
  // (a segment's first frame needs the previous step's view settled).
  const starts = [0];
  for (let i = 1; i < shots.length; i++) {
    const f = align(frameOf(shots[i].at));
    if (f > starts.at(-1) && f - starts.at(-1) >= 1.5 * CLOCK && f < total) starts.push(f);
  }
  const segments = starts.map((s, k) => ({ start: s, end: k + 1 < starts.length ? starts[k + 1] : total }));
  for (const seg of segments) {
    seg.key = sha(JSON.stringify({
      v: TOOL_VERSION, format, mode, kind, q, start: seg.start, end: seg.end,
      shots: shots.filter((s) => frameOf(s.at) < seg.end),
    }));
    seg.clip = join(cacheDir, `${seg.key}.mp4`);
    seg.meta = join(cacheDir, `${seg.key}.json`);
  }
  let drawn = 0;
  let held = 0;
  let probed = 0;
  for (const seg of segments) {
    // Reuse when the current build draws the cached probe frames exactly.
    if (existsSync(seg.clip) && existsSync(seg.meta)) {
      const { probes } = JSON.parse(readFileSync(seg.meta, "utf8"));
      let same = probes.every(([f]) => f > page.last);
      for (const [f, file] of probes) {
        if (!same) break;
        await page.render(f);
        probed++;
        same = alike(await page.png(), join(cacheDir, file));
      }
      seg.cached = same;
      if (same) continue;
    }
    const frames = mkdtempSync(join(tmpdir(), "seg-"));
    // Warm up: the end of the previous segment, so this one starts from
    // its settled view.
    if (page.last !== seg.start - step) {
      if (page.last > seg.start - step) {
        page.close();
        page = await openDemo(format, mode, q.dpr);
      }
      for (let f = Math.max(0, seg.start - 3 * step); f < seg.start; f += step) {
        if (f > page.last) await page.render(f);
      }
    }
    let n = 0;
    let prev = null;
    const hashes = new Map();
    const pngs = new Map();
    for (let f = seg.start; f < seg.end; ) {
      const info = await page.render(f);
      const png = await page.png();
      drawn++;
      const file = join(frames, `${String(n++).padStart(5, "0")}.png`);
      writeFileSync(file, png);
      const hash = sha(png);
      hashes.set(f, hash);
      pngs.set(f, png);
      f += step;
      // Twice the same picture with nothing moving: repeat it until the
      // next step (or the segment's end).
      if (prev && prev.hash === hash && prev.still && info.still) {
        const until = Math.min(seg.end, info.next == null ? seg.end : align(frameOf(info.next)));
        for (; f < until; f += step) {
          linkSync(file, join(frames, `${String(n++).padStart(5, "0")}.png`));
          hashes.set(f, hash);
          pngs.set(f, png);
          held++;
        }
      }
      prev = { hash, still: info.still };
    }
    const r = spawnSync("ffmpeg", ["-v", "error", "-y", "-framerate", String(q.fps), "-i", join(frames, "%05d.png"),
      ...q.encode, "-r", String(q.fps), "-an", seg.clip]);
    if (r.status !== 0) throw new Error(`ffmpeg: ${r.stderr}`);
    rmSync(frames, { recursive: true, force: true });
    // Probe frames for next time: the first, the middle and the last.
    const all = [...hashes.keys()];
    const pick = [...new Set([all[0], all[Math.floor(all.length / 2)], all.at(-1)])];
    const probes = pick.map((f) => {
      const file = `${seg.key}-${f}.png`;
      writeFileSync(join(cacheDir, file), pngs.get(f));
      return [f, file];
    });
    writeFileSync(seg.meta, JSON.stringify({ probes }));
  }
  const requests = page.requests;
  const errors = page.errors;
  page.close();
  const name = `${kind === "final" ? "demo" : "draft"}-${format}${mode === "narration" ? "-narration" : ""}.mp4`;
  const list = join(mkdtempSync(join(tmpdir(), "list-")), "list.txt");
  writeFileSync(list, segments.map((s) => `file '${s.clip}'`).join("\n"));
  const r = spawnSync("ffmpeg", ["-v", "error", "-y", "-f", "concat", "-safe", "0", "-i", list, "-c", "copy",
    "-movflags", "+faststart", join(OUT, name)]);
  if (r.status !== 0) throw new Error(`ffmpeg concat: ${r.stderr}`);
  return {
    name, file: join(OUT, name), segments: segments.length, reused: segments.filter((s) => s.cached).length,
    drawn, held, probed, requests, errors,
  };
}

// --- Review page ------------------------------------------------------------

function publish(kind, items) {
  const stamp = new Date().toISOString().replace(/[:.]/g, "-").slice(0, 19);
  const run = join(REVIEW, "runs", `${stamp}-${kind}`);
  mkdirSync(run, { recursive: true });
  const entry = { id: `${stamp}-${kind}`, kind, time: new Date().toISOString(), note, items: [] };
  for (const item of items) {
    const copy = (src) => {
      const name = `${item.label.replace(/\W+/g, "-")}-${basename(src)}`;
      copyFileSync(src, join(run, name));
      return `runs/${entry.id}/${name}`;
    };
    const out = { label: item.label, format: item.format, mode: item.mode };
    if (item.contact) out.contact = copy(item.contact);
    if (item.phone) out.phone = copy(item.phone);
    if (item.video) out.video = copy(item.video);
    if (item.vtt) out.vtt = copy(item.vtt);
    if (item.checks) out.checks = item.checks;
    if (item.stills) out.stills = item.stills.map((s) => ({ ...s, file: copy(join(item.stillsDir, s.file)) }));
    entry.items.push(out);
  }
  const manifestFile = join(REVIEW, "manifest.json");
  const manifest = existsSync(manifestFile) ? JSON.parse(readFileSync(manifestFile, "utf8")) : [];
  manifest.unshift(entry);
  writeFileSync(manifestFile, JSON.stringify(manifest, null, 1));
  copyFileSync(join(ROOT, "tools", "review", "index.html"), join(REVIEW, "index.html"));
  return entry.id;
}

// --- Main ---------------------------------------------------------------------

const started = Date.now();
const report = [];
const items = [];
let network = [];
if (command !== "stills" || review) {
  // Subtitles from the same plan the app plays.
  const r = spawnSync(join(ROOT, "target", "release", "lightable"), ["demo-cues", OUT]);
  if (r.status !== 0) console.error("note: could not write subtitles (build target/release/lightable)");
}
for (const mode of modes) {
  for (const format of formats) {
    const t0 = Date.now();
    const label = `${mode} ${format}`;
    if (command === "stills") {
      const s = await stills(format, mode);
      network.push(...s.requests);
      const failed = s.results.flatMap((r) => Object.entries(r.checks).filter(([, c]) => c.pass === false).map(([k]) => `${r.file}: ${k}`));
      report.push({ label, seconds: (Date.now() - t0) / 1000, stills: s.results.length, failed, errors: s.errors });
      items.push({ label, format, mode, contact: join(s.dir, "contact.png"), phone: join(s.dir, "contact-phone.png"), stills: s.results, stillsDir: s.dir });
    } else {
      const v = await video(format, mode, command);
      network.push(...v.requests);
      const vtt = join(OUT, mode === "narration" ? "demo-narration.vtt" : "demo.vtt");
      report.push({ label, seconds: (Date.now() - t0) / 1000, video: v.name, segments: v.segments, reused: v.reused, drawn: v.drawn, held: v.held, probed: v.probed, errors: v.errors });
      items.push({ label, format, mode, video: v.file, vtt: existsSync(vtt) ? vtt : null });
    }
  }
}
const origin = new URL(base).origin;
const summary = {
  command, seconds: (Date.now() - started) / 1000, runs: report,
  network: {
    requests: network.length,
    toOtherOrigins: network.filter((r) => !r.url.startsWith(origin) && !r.url.startsWith("data:")).map((r) => r.url),
    withBody: network.filter((r) => r.method !== "GET" || r.body > 0).map((r) => `${r.method} ${r.url}`),
  },
};
if (review) summary.review = publish(command, items);
console.log(JSON.stringify(summary, null, 1));
