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
// demo time = frame / 30, and every zoom starts at its scheduled time. The
// tool performs the person's gestures (the plan's `__lightableDemoGestures`)
// as browser input events: the example files dragged and dropped, mouse
// moves and clicks, ctrl + wheel to pinch, wheel steps to scroll the list.
// Frames a gesture needs are always drawn in order, so any frame looks the
// same however it is reached. Stills jump
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
import { cpus, tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";

const ROOT = resolve(new URL("..", import.meta.url).pathname);
const OUT = join(ROOT, "out", "demo");
const REVIEW = join(ROOT, "out", "review");
const ASSETS = join(ROOT, "crates", "lightable", "assets");
const TOOL_VERSION = 4;
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
    // Poster: the overlay and the change list, a still from the video itself.
    spawnSync("ffmpeg", ["-v", "error", "-y", "-ss", mode === "narration" ? "16" : "10", "-i", src, "-frames:v", "1", "-q:v", "3",
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
if (!["stills", "cut", "draft", "final"].includes(command)) {
  console.error("usage: node tools/demo.mjs stills|cut|draft|final [--modes normal,narration] [--formats landscape,vertical] [--workers N] [--note TEXT]");
  process.exit(2);
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
// Time spent per phase (printed with --profile).
const PROFILE = {};
const timed = async (phase, work) => {
  const t0 = Date.now();
  try {
    return await work();
  } finally {
    PROFILE[phase] = (PROFILE[phase] ?? 0) + Date.now() - t0;
  }
};
const sha = (data) => createHash("sha256").update(data).digest("hex");
const FRAME = { landscape: [960, 540], vertical: [540, 960] };
const hasVideoToolbox = spawnSync("ffmpeg", ["-hide_banner", "-encoders"]).stdout.toString().includes("h264_videotoolbox");
const QUALITY = {
  stills: { dpr: 2 },
  // Like draft, but every step cuts in without its zoom: content review in
  // seconds.
  // Mostly held frames in short segments: x264 "ultrafast" starts quicker
  // than a VideoToolbox session per segment.
  // Drafts render at 2x like finals (on a high-density screen Chrome
  // reports 2x device pixels to the app even when 1x is emulated, which
  // laid 1x pages out at half scale) and are captured scaled to half.
  cut: {
    dpr: 2, shrink: 0.5, fps: 15, cut: true,
    encode: ["-c:v", "libx264", "-preset", "ultrafast", "-crf", "23", "-pix_fmt", "yuv420p"],
  },
  draft: {
    dpr: 2, shrink: 0.5, fps: 15,
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

// Browsers working at once (`--workers N`): half the physical cores, 1 to
// 4. Each browser plus its share of x264 keeps about two cores busy; on a
// 2-core Mac two workers were slower than one (measured).
function physicalCores() {
  const mac = spawnSync("sysctl", ["-n", "hw.physicalcpu"]).stdout?.toString().trim();
  if (mac && Number(mac) > 0) return Number(mac);
  const linux = spawnSync("sh", ["-c", "lscpu -p=core 2>/dev/null | grep -v '^#' | sort -u | wc -l"]).stdout?.toString().trim();
  if (linux && Number(linux) > 0) return Number(linux);
  return Math.max(1, Math.floor(cpus().length / 2));
}
// Stills and cut drafts mostly wait for pages to load: at least two there.
const WORKERS = Number(opt("workers", 0))
  || Math.min(4, Math.max(["stills", "cut"].includes(command) ? 2 : 1, Math.floor(physicalCores() / 2)));
const CHROME = process.env.CHROME ?? "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
let nextPort = 9350 + Math.floor(Math.random() * 300);

// One Chrome for the whole run (launching one with a fresh profile takes
// about 8 s); each page gets its own window, so none is a hidden tab.
let browser = null;
function sharedBrowser() {
  browser ??= (async () => {
    const port = nextPort++;
    const proc = spawn(CHROME, [
      `--remote-debugging-port=${port}`, `--user-data-dir=${mkdtempSync(join(tmpdir(), "demo-"))}`,
      "--no-first-run", "--no-default-browser-check", "--window-size=1400,1000",
      "--disable-background-timer-throttling", "--disable-renderer-backgrounding",
      "--disable-backgrounding-occluded-windows", "about:blank",
    ], { stdio: "ignore" });
    // Never left running, even when the run fails.
    process.on("exit", () => proc.kill());
    let version;
    for (let i = 0; i < 120 && !version; i++) {
      try {
        version = await (await fetch(`http://127.0.0.1:${port}/json/version`)).json();
      } catch {}
      if (!version) await sleep(250);
    }
    const ws = new WebSocket(version.webSocketDebuggerUrl);
    await new Promise((r) => ws.addEventListener("open", r));
    let id = 0;
    const pending = new Map();
    // Downloads (the demo's exported report), by the page that made them.
    const downloads = new Map();
    ws.addEventListener("message", (m) => {
      const d = JSON.parse(m.data);
      if (pending.has(d.id)) {
        pending.get(d.id)(d.result ?? { error: d.error });
        pending.delete(d.id);
      }
      if (d.method === "Browser.downloadWillBegin") {
        downloads.set(d.params.guid, { frame: d.params.frameId, name: d.params.suggestedFilename, state: "started" });
      }
      if (d.method === "Browser.downloadProgress" && downloads.has(d.params.guid)) {
        downloads.get(d.params.guid).state = d.params.state;
      }
    });
    const send = (method, params = {}) => new Promise((r) => {
      const n = ++id;
      pending.set(n, r);
      ws.send(JSON.stringify({ id: n, method, params }));
    });
    // Saved under their download id in one folder for the run.
    const folder = mkdtempSync(join(tmpdir(), "demo-export-"));
    await send("Browser.setDownloadBehavior", { behavior: "allowAndName", downloadPath: folder, eventsEnabled: true });
    return { port, proc, send, downloads, folder };
  })();
  return browser;
}

async function openDemo(format, mode, dpr, cut = false) {
  const tOpen = Date.now();
  const shared = await sharedBrowser();
  const { port } = shared;
  const { targetId } = await shared.send("Target.createTarget", { url: "about:blank", newWindow: true });
  const ws = new WebSocket(`ws://127.0.0.1:${port}/devtools/page/${targetId}`);
  await new Promise((r) => ws.addEventListener("open", r));
  const tLaunched = Date.now();
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
  const url = `${base}?demo&record&format=${format}${mode === "narration" ? "&narration" : ""}${cut ? "&cut" : ""}`;
  await send("Page.navigate", { url });
  for (let i = 0; i < 600; i++) {
    if (await evaluate("!document.getElementById('loading')")) break;
    await sleep(100);
  }
  const tLoaded = Date.now();
  // A window on a high-density screen can report its own pixel density
  // before the emulated one, and the app keeps the canvas size it saw: a
  // resize makes it measure again (1x pages were laid out at half scale).
  const remeasure = async () => {
    await send("Emulation.setDeviceMetricsOverride", { width: w + 2, height: h, deviceScaleFactor: dpr, mobile: false });
    await sleep(200);
    await send("Emulation.setDeviceMetricsOverride", { width: w, height: h, deviceScaleFactor: dpr, mobile: false });
    await sleep(200);
  };
  await remeasure();
  // Browser (CSS) pixels across the page: input events are given in them.
  const cssWidth = await evaluate("innerWidth");
  await evaluate("window.__lightableFrame = 0");
  let length;
  for (let i = 0; i < 600 && !length; i++) {
    await nudge();
    length = await evaluate("window.__lightableDemoLength");
    if (!length) await sleep(100);
  }
  if (!length) throw new Error(`the demo did not start: ${url} ${errors.join(" | ")}`);
  // The app's width in points times its zoom must be the page's width, or
  // the pointer would land beside what it aims at.
  for (let i = 0; ; i++) {
    for (let k = 0; k < 3; k++) {
      await nudge();
      await sleep(60);
    }
    const now = JSON.parse(await evaluate("window.__lightableFrameInfo"));
    const ratio = cssWidth / (now.screen[2] - now.screen[0]);
    if (Math.abs(ratio - now.zoom) < 0.01) break;
    if (i >= 10) throw new Error(`the page is drawn at the wrong scale (${ratio} CSS pixels a point, zoom ${now.zoom})`);
    await remeasure();
  }
  const shots = JSON.parse(await evaluate("window.__lightableDemoShots"));
  const gestures = JSON.parse(await evaluate("window.__lightableDemoGestures"));
  const files = JSON.parse(await evaluate("window.__lightableDemoFiles")).map((f) => join(ASSETS, f));
  // The report the demo exports is really saved (the browser reports it).
  const exports = () => [...shared.downloads.entries()]
    .filter(([, d]) => d.frame === targetId && d.state === "completed")
    .map(([guid, d]) => ({ name: d.name, file: join(shared.folder, guid) }));
  PROFILE.launch = (PROFILE.launch ?? 0) + (tLaunched - tOpen);
  PROFILE.load = (PROFILE.load ?? 0) + (tLoaded - tLaunched);
  PROFILE.start = (PROFILE.start ?? 0) + (Date.now() - tLoaded);
  let last = -1;
  const draws = async () => (await evaluate("window.__lightableDraws")) ?? 0;
  const info = async () => JSON.parse(await evaluate("window.__lightableFrameInfo"));
  // Waits for the app to draw after `before` draws; nudges if it does not.
  const drawn = async (before) => {
    for (let i = 0; i < 4000; i++) {
      // Only a fallback: an extra nudge can mean an extra draw, so wait
      // long enough for a busy machine.
      if (i > 0 && i % 600 === 0) await nudge();
      if ((await draws()) > before) return;
      await sleep(5);
    }
    const state = await evaluate("JSON.stringify({frame: window.__lightableFrame, rendered: window.__lightableRendered, draws: window.__lightableDraws})");
    writeFileSync(join(tmpdir(), "demo-stall.png"), Buffer.from((await send("Page.captureScreenshot", { format: "png" })).data, "base64"));
    throw new Error(`the app stopped drawing (${state}): ${errors.join(" | ")}`);
  };
  // --- The person: pointer, clicks, drop and pinch, as browser input -----
  // Frames a gesture needs drawn in order (the pointer's path, the click,
  // each step of the pinch, and a few frames after): never skipped.
  const busyFrames = (step) => gestures.map((g) => [
    Math.floor((g.from * CLOCK) / step) * step - step,
    Math.ceil((g.until * CLOCK) / step) * step + 3 * step,
  ]);
  const driver = { next: 0, path: null, pos: null, zoomed: 0, dragging: false };
  let latest = null; // the last frame's info
  // The input events for frame f (CSS pixels), from the plan's gestures and
  // where their targets were on the last frame.
  function eventsFor(f, cut) {
    const t = f / CLOCK;
    const events = [];
    for (;;) {
      const g = gestures[driver.next];
      if (!g || t < (cut ? g.at : g.from)) return events;
      // CSS pixels per app point (the app's zoom; measured, as 1x pages
      // scale differently).
      const z = cssWidth / (latest.screen[2] - latest.screen[0]);
      const click = !["drop", "pinch"].includes(g.target);
      // A row outside the list's view: the pointer goes to the list and
      // scrolls it (small wheel steps, which egui applies at once) first.
      const row = latest.targets[g.target];
      const list = latest.targets.list;
      // In view (a point of slack), or no longer moving as the list scrolls.
      const stuck = driver.scroll?.last && row && driver.scroll.last === row[1];
      const inside = (r, o) => stuck || (r[1] >= o[1] - 1 && r[3] <= o[3] + 1);
      if (!driver.path && !cut && g.target.startsWith("row:") && row && list && !inside(row, list)) {
        const centre = [((list[0] + list[2]) / 2) * z, ((list[1] + list[3]) / 2) * z];
        driver.scroll ??= { start: t, from: driver.pos ?? centre };
        const k = Math.min(1, (t - driver.scroll.start) / 0.3);
        const e = k * k * (3 - 2 * k);
        const pos = [driver.scroll.from[0] + (centre[0] - driver.scroll.from[0]) * e, driver.scroll.from[1] + (centre[1] - driver.scroll.from[1]) * e];
        events.push(["Input.dispatchMouseEvent", { type: "mouseMoved", x: pos[0], y: pos[1], button: "none" }]);
        driver.pos = pos;
        if (k >= 1) {
          driver.scroll.last = row[1];
          const down = row[3] > list[3] ? 7 : -7;
          for (let n = 0; n < 3; n++) {
            events.push(["Input.dispatchMouseEvent", { type: "mouseWheel", x: pos[0], y: pos[1], deltaX: 0, deltaY: down }]);
          }
        }
        return events;
      }
      if (cut && g.target.startsWith("row:") && row && list && !inside(row, list)) {
        // Cut drafts: scroll at once (still for real), then click.
        const centre = [((list[0] + list[2]) / 2) * z, ((list[1] + list[3]) / 2) * z];
        const down = row[3] > list[3] ? 7 : -7;
        for (let n = 0; n < 3; n++) {
          events.push(["Input.dispatchMouseEvent", { type: "mouseWheel", x: centre[0], y: centre[1], deltaX: 0, deltaY: down }]);
        }
        driver.pos = centre;
        return events;
      }
      driver.scroll = null;
      if (!driver.path) {
        let to;
        let factor = 1;
        if (g.target === "pinch") {
          if (!latest.pinch) throw new Error(`frame ${f}: no revision cloud to pinch`);
          to = [latest.pinch[0] * z, latest.pinch[1] * z];
          factor = latest.pinch[2];
        } else {
          const r = latest.targets[g.target];
          if (!r) throw new Error(`frame ${f}: "${g.target}" is not on screen (on screen: ${Object.keys(latest.targets).join(", ")}; list ${latest.list}, dropped ${latest.dropped})`);
          to = [((r[0] + r[2]) / 2) * z, ((r[1] + r[3]) / 2) * z];
        }
        // The first gesture comes in from the right-hand edge.
        const [w, h] = [latest.screen[2] * z, latest.screen[3] * z];
        // Clicks: the pointer takes at most 0.8 s to get there.
        driver.path = { from: driver.pos ?? [w - 2, h * 0.6], to, factor, start: click ? Math.max(t, g.at - 0.8) : g.from };
      }
      const { from, to, factor, start } = driver.path;
      const drag = g.target === "drop";
      const data = { items: [], files, dragOperationsMask: 1 };
      if (t < g.at) {
        let k = Math.min(1, Math.max(0, (t - start) / Math.max(1e-3, g.at - 0.1 - start)));
        k = k * k * (3 - 2 * k);
        const pos = [from[0] + (to[0] - from[0]) * k, from[1] + (to[1] - from[1]) * k];
        if (drag) {
          events.push(["Input.dispatchDragEvent", { type: driver.dragging ? "dragOver" : "dragEnter", x: pos[0], y: pos[1], data }]);
          driver.dragging = true;
        } else if (!driver.pos || pos[0] !== driver.pos[0] || pos[1] !== driver.pos[1]) {
          events.push(["Input.dispatchMouseEvent", { type: "mouseMoved", x: pos[0], y: pos[1], button: "none" }]);
        }
        driver.pos = pos;
        return events;
      }
      const [x, y] = to;
      if (drag) {
        if (!driver.dragging) events.push(["Input.dispatchDragEvent", { type: "dragEnter", x, y, data }]);
        events.push(["Input.dispatchDragEvent", { type: "dragOver", x, y, data }]);
        events.push(["Input.dispatchDragEvent", { type: "drop", x, y, data }]);
        driver.dragging = false;
      } else {
        if (!driver.pos || x !== driver.pos[0] || y !== driver.pos[1]) {
          events.push(["Input.dispatchMouseEvent", { type: "mouseMoved", x, y, button: "none" }]);
        }
        if (g.target === "pinch") {
          // Ctrl + wheel, as a trackpad pinch sends it: the browser's
          // factor is exp(-deltaY / 100).
          const p = cut || g.until <= g.at ? 1 : Math.min(1, (t - g.at) / (g.until - g.at));
          const step = factor ** (p - driver.zoomed);
          driver.zoomed = p;
          events.push(["Input.dispatchMouseEvent", { type: "mouseWheel", x, y, deltaX: 0, deltaY: -100 * Math.log(step), modifiers: 2 }]);
          if (p < 1) {
            driver.pos = to;
            return events;
          }
          driver.zoomed = 0;
        } else {
          events.push(["Input.dispatchMouseEvent", { type: "mousePressed", x, y, button: "left", buttons: 1, clickCount: 1 }]);
          events.push(["Input.dispatchMouseEvent", { type: "mouseReleased", x, y, button: "left", buttons: 0, clickCount: 1 }]);
        }
      }
      driver.pos = to;
      driver.path = null;
      driver.next++;
    }
  }
  // Draws frame f: its input events first (each drawn), or a nudge; then
  // again while the app is busy (files being read, the report captured).
  async function draw(f, cut) {
    await evaluate(`window.__lightableFrame = ${f}`);
    const events = latest ? eventsFor(f, cut) : [];
    if (events.length === 0) {
      const before = await draws();
      await nudge();
      await drawn(before);
    }
    for (const [method, params] of events) {
      const before = await draws();
      await send(method, params);
      await drawn(before);
    }
    for (let i = 0; i < 6000; i++) {
      if ((await evaluate("window.__lightableRendered")) === f) break;
      await sleep(5);
    }
    let now = await info();
    if (process.env.DEMO_DEBUG) console.error(f, JSON.stringify(events.map(([m, p]) => [p.type, Math.round(p.x), Math.round(p.y)])), now.pointer);
    for (let i = 0; now.busy; i++) {
      if (i > 400) throw new Error(`frame ${f}: the app stayed busy: ${errors.join(" | ")}`);
      await sleep(20);
      const before = await draws();
      await nudge();
      await drawn(before);
      now = await info();
    }
    latest = now;
    last = f;
    return now;
  }
  return {
    url, length, shots, gestures, requests, errors, exports,
    get last() {
      return last;
    },
    // Draws frame `f` (frames must go forward) and returns what is on
    // screen. Frames skipped on the way are drawn too where a gesture needs
    // them, so the person's actions happen exactly as when every frame is
    // drawn.
    async render(f, step = 1, cut = false) {
      for (const [a, b] of busyFrames(step)) {
        for (let k = Math.max(a, last + 1); k < Math.min(b + 1, f); k++) {
          if (k % step === 0 && k > last) await timed("gesture", () => draw(k, cut));
        }
      }
      return draw(f, cut);
    },
    async png(scale = 1) {
      const clip = scale === 1 ? undefined : { x: 0, y: 0, width: w, height: h, scale };
      return Buffer.from((await send("Page.captureScreenshot", { format: "png", clip })).data, "base64");
    },
    close() {
      shared.send("Target.closeTarget", { targetId });
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
function check(info, format, sample, target) {
  const [x0, y0, x1, y1] = info.screen;
  const sh = y1 - y0;
  const out = {};
  // The pointer reached the control before using it.
  if (sample === "aim") {
    const p = info.pointer;
    const r = target === "pinch" ? info.pinch && [info.pinch[0] - 3, info.pinch[1] - 3, info.pinch[0] + 3, info.pinch[1] + 3] : info.targets[target];
    out.pointer = {
      pass: !!(p && r && p[0] >= r[0] && p[0] <= r[2] && p[1] >= r[1] && p[1] <= r[3]),
      detail: p && r ? `pointer at ${p.map(Math.round)} on ${target}` : `pointer ${p ? "shown" : "missing"}, ${target} ${r ? "on screen" : "not on screen"}`,
    };
  }
  // After the drop: the comparison came from the dropped example files.
  if (info.list && !info.phone) {
    out.dropped = { pass: info.dropped && info.example_files, detail: info.dropped ? (info.example_files ? "compared the dropped example files" : "dropped files are not the example's") : "not opened by a drop" };
  }
  if (info.caption) {
    const share = info.caption.size / sh;
    out.caption = {
      pass: share >= 0.0395 && info.caption.rows <= 2 && inside(info.caption.rect, info.screen),
      detail: `text ${(share * 100).toFixed(1)} % of the height, ${info.caption.rows} line(s)`,
    };
  } else {
    out.caption = { pass: null, detail: "no caption" };
  }
  if (sample !== "settled") {
    out.cutoff = { pass: null, detail: `${sample} (not checked)` };
  } else if (info.focus.length && info.canvas) {
    const cut = info.focus.filter((f) => !inside(f, info.canvas) || !inside(f, info.screen));
    const covered = info.caption ? info.focus.filter((f) => overlaps(f, info.caption.rect)) : [];
    out.cutoff = {
      pass: cut.length === 0 && covered.length === 0,
      detail: cut.length ? "the change runs off the drawing area" : covered.length ? "the caption covers the change" : "change fully visible, clear of the caption",
    };
  } else {
    out.cutoff = { pass: null, detail: info.report ? "report layout" : info.end_card ? "end card" : info.phone ? "phone" : "nothing in focus" };
  }
  if (info.list_tail != null) {
    out.list = { pass: info.list_tail, detail: info.list_tail ? "'… and N more' fully visible" : "the list is cut off before '… and N more'" };
  }
  if (format === "vertical") {
    if (info.canvas && !info.end_card && !info.phone) {
      const [c0, c1, c2, c3] = info.canvas;
      const share = ((c2 - c0) * (c3 - c1)) / ((x1 - x0) * sh);
      // Details open (under a second, before "Export PNG report"): its
      // extra toolbar row is the real layout; reported, not judged.
      const details = !!info.targets.export_png;
      out.drawing = {
        pass: details ? null : share >= 0.6,
        detail: `drawing ${(share * 100).toFixed(0)} % of the frame${details ? " (Details open, not checked)" : ""}`,
      };
    } else {
      out.drawing = { pass: null, detail: info.report ? "report layout" : info.phone ? "phone" : "end card" };
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
  const stepAt = (t) => Math.max(0, page.shots.findLastIndex((s) => s.at <= t));
  page.shots.forEach((s, i) => {
    const next = i + 1 < page.shots.length ? page.shots[i + 1].at : page.length;
    if (next - s.at > 2.5) samples.push({ t: s.at + 1.0, step: i, act: actName(s.desc), sample: "early" });
    samples.push({ t: Math.max(s.at, next - 2 / CLOCK), step: i, act: actName(s.desc), sample: "settled" });
  });
  // Each gesture: the pointer on its way, and on its target just before it
  // acts (checked); a pinch half-way.
  for (const g of page.gestures) {
    const step = stepAt(g.at);
    const act = actName(page.shots[step].desc);
    samples.push({ t: (g.from + g.at) / 2, step, act, sample: "moving" });
    samples.push({ t: g.at - 1 / CLOCK, step, act, sample: "aim", target: g.target });
    if (g.until > g.at) samples.push({ t: (g.at + g.until) / 2, step, act, sample: "mid" });
  }
  samples.sort((a, b) => a.t - b.t);
  const results = [];
  let lastFrame = -1;
  for (const s of samples) {
    const f = Math.max(frameOf(s.t), lastFrame + 1);
    const info = await page.render(f);
    lastFrame = f;
    const name = `${String(results.length).padStart(2, "0")}-${s.act}-${s.sample}-${(f / CLOCK).toFixed(1)}s.png`;
    writeFileSync(join(dir, name), await page.png());
    results.push({ file: name, time: f / CLOCK, step: s.step, act: s.act, sample: s.sample, checks: check(info, format, s.sample, s.target) });
  }
  // The report the demo exported: saved for real.
  // (Chrome finishes writing it in the background.)
  let exported = [];
  for (let i = 0; i < 300 && exported.length === 0; i++) {
    exported = page.exports().filter((e) => e.name.endsWith(".png") && existsSync(e.file));
    if (exported.length === 0) await sleep(100);
  }
  exported = exported.map((e) => e.name);
  const last = results.at(-1);
  last.checks.export = { pass: exported.length === 1, detail: exported.length ? `report saved: ${exported.join(", ")}` : "no report was saved" };
  page.close();
  // Contact sheet of the settled stills, and a phone-sized copy.
  const settled = results.filter((r) => r.sample === "settled" || r.sample === "aim");
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

// Runs at most `n` jobs at a time.
function limiter(n) {
  let active = 0;
  const waiting = [];
  return async (job) => {
    while (active >= n) await new Promise((r) => waiting.push(r));
    active++;
    try {
      return await job();
    } finally {
      active--;
      waiting.shift()?.();
    }
  };
}

function ffmpeg(args) {
  return new Promise((resolve, reject) => {
    const p = spawn("ffmpeg", args, { stdio: ["ignore", "ignore", "pipe"] });
    let err = "";
    p.stderr.on("data", (d) => (err += d));
    p.on("close", (code) => (code === 0 ? resolve() : reject(new Error(`ffmpeg: ${err}`))));
  });
}

async function video(format, mode, kind, workers, encodeSlot) {
  const q = QUALITY[kind];
  const step = CLOCK / q.fps;
  const cacheDir = join(OUT, "cache");
  mkdirSync(cacheDir, { recursive: true });
  // The plan (steps and length) comes from the app; the first page reads it.
  const first = await timed("open", () => openDemo(format, mode, q.dpr, q.cut));
  const { shots, length } = first;
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
      gestures: first.gestures.filter((g) => frameOf(g.from) < seg.end),
    }));
    seg.clip = join(cacheDir, `${seg.key}.mp4`);
    seg.meta = join(cacheDir, `${seg.key}.json`);
  }
  const counts = { drawn: 0, held: 0, probed: 0 };
  const encodes = [];
  const requests = [];
  const errors = [];
  const pages = [first];

  async function segmentWork(page, seg) {
    // Reuse when the current build draws the cached probe frames alike.
    if (existsSync(seg.clip) && existsSync(seg.meta)) {
      const { probes } = JSON.parse(readFileSync(seg.meta, "utf8"));
      let same = probes.every(([f]) => f > page.last);
      for (const [f, file] of probes) {
        if (!same) break;
        await timed("probe", () => page.render(f, step, q.cut));
        counts.probed++;
        same = await timed("probe", async () => alike(await page.png(q.shrink ?? 1), join(cacheDir, file)));
      }
      seg.cached = same;
      if (same) return page;
    }
    // Warm up: the end of the previous segment, so this one starts from its
    // settled view.
    if (seg.start > 0 && page.last !== seg.start - step) {
      if (page.last > seg.start - step) {
        page.close();
        page = await timed("open", () => openDemo(format, mode, q.dpr, q.cut));
      }
      for (let f = Math.max(0, seg.start - 3 * step); f < seg.start; f += step) {
        if (f > page.last) await timed("render", () => page.render(f, step, q.cut));
      }
    }
    const frames = mkdtempSync(join(tmpdir(), "seg-"));
    let n = 0;
    let prev = null;
    const pngs = new Map();
    for (let f = seg.start; f < seg.end; ) {
      const info = await timed("render", () => page.render(f, step, q.cut));
      const png = await timed("capture", () => page.png(q.shrink ?? 1));
      counts.drawn++;
      const hash = sha(png);
      pngs.set(f, png);
      const file = join(frames, `${String(n++).padStart(5, "0")}.png`);
      writeFileSync(file, png);
      f += step;
      // Twice the same picture with nothing moving: repeat it (hard links)
      // until the next step (or the segment's end).
      if (prev && prev.hash === hash && prev.still && info.still) {
        const until = Math.min(seg.end, info.next == null ? seg.end : align(frameOf(info.next)));
        for (; f < until; f += step) {
          linkSync(file, join(frames, `${String(n++).padStart(5, "0")}.png`));
          pngs.set(f, png);
          counts.held++;
        }
      }
      prev = { hash, still: info.still };
    }
    // Probe frames for next time: the first, the middle and the last.
    const all = [...pngs.keys()];
    const pick = [...new Set([all[0], all[Math.floor(all.length / 2)], all.at(-1)])];
    const probes = pick.map((f) => {
      const file = `${seg.key}-${f}.png`;
      writeFileSync(join(cacheDir, file), pngs.get(f));
      return [f, file];
    });
    // Encode in the background while rendering goes on: the numbered frames
    // exactly as before (a concat list with durations shifted frames by one).
    encodes.push(encodeSlot(() => timed("encode", async () => {
      await ffmpeg(["-v", "error", "-y", "-framerate", String(q.fps), "-i", join(frames, "%05d.png"),
        ...q.encode, "-r", String(q.fps), "-an", seg.clip]);
      writeFileSync(seg.meta, JSON.stringify({ probes }));
      rmSync(frames, { recursive: true, force: true });
    })));
    return page;
  }

  // Workers take the next segment in order; each keeps its own page.
  let next = 0;
  await Promise.all(Array.from({ length: Math.min(workers, segments.length) }, async (_, w) => {
    let page = w === 0 ? first : null;
    while (next < segments.length) {
      const seg = segments[next++];
      if (!page) {
        page = await timed("open", () => openDemo(format, mode, q.dpr, q.cut));
        pages.push(page);
      }
      page = await segmentWork(page, seg);
      if (!pages.includes(page)) pages.push(page);
    }
  }));
  await Promise.all(encodes);
  for (const p of pages) {
    requests.push(...p.requests);
    errors.push(...p.errors);
    p.close();
  }
  const prefix = kind === "final" ? "demo" : kind;
  const name = `${prefix}-${format}${mode === "narration" ? "-narration" : ""}.mp4`;
  const list = join(mkdtempSync(join(tmpdir(), "list-")), "list.txt");
  writeFileSync(list, segments.map((s) => `file '${s.clip}'`).join("\n"));
  await ffmpeg(["-v", "error", "-y", "-f", "concat", "-safe", "0", "-i", list, "-c", "copy",
    "-movflags", "+faststart", join(OUT, name)]);
  return {
    name, file: join(OUT, name), segments: segments.length, reused: segments.filter((s) => s.cached).length,
    ...counts, requests, errors,
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
// Variants run side by side, WORKERS browsers in all: whole variants per
// worker when there are enough of them, otherwise segments of one video
// across several.
const variants = modes.flatMap((mode) => formats.map((format) => ({ mode, format })));
const runVariant = limiter(WORKERS);
const encodeSlot = limiter(WORKERS);
const perVideo = Math.max(1, Math.floor(WORKERS / variants.length));
await Promise.all(variants.map(({ mode, format }, k) => runVariant(async () => {
  const t0 = Date.now();
  const label = `${mode} ${format}`;
  if (command === "stills") {
    const s = await stills(format, mode);
    network.push(...s.requests);
    const failed = s.results.flatMap((r) => Object.entries(r.checks).filter(([, c]) => c.pass === false).map(([key]) => `${r.file}: ${key}`));
    report[k] = { label, seconds: (Date.now() - t0) / 1000, stills: s.results.length, failed, errors: s.errors };
    items[k] = { label, format, mode, contact: join(s.dir, "contact.png"), phone: join(s.dir, "contact-phone.png"), stills: s.results, stillsDir: s.dir };
  } else {
    const v = await video(format, mode, command, perVideo, encodeSlot);
    network.push(...v.requests);
    const vtt = join(OUT, mode === "narration" ? "demo-narration.vtt" : "demo.vtt");
    report[k] = { label, seconds: (Date.now() - t0) / 1000, video: v.name, segments: v.segments, reused: v.reused, drawn: v.drawn, held: v.held, probed: v.probed, errors: v.errors };
    items[k] = { label, format, mode, video: v.file, vtt: existsSync(vtt) ? vtt : null };
  }
})));
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
if (args.includes("--profile")) summary.profile = PROFILE;
summary.workers = WORKERS;
console.log(JSON.stringify(summary, null, 1));
if (browser) (await browser).proc.kill();
process.exit(0);
