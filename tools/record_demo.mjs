// Records the demo to an MP4 (H.264, 30 fps, no audio), frame by frame:
//
//   node tools/record_demo.mjs URL OUT.mp4 WIDTH HEIGHT
//
// URL must have `?demo&record` (plus `&format=landscape|vertical`, and
// `&narration` for voice-over timing). WIDTH × HEIGHT are CSS pixels at a
// device pixel ratio of 2, so 960 540 gives 1920 × 1080 frames. For each
// frame the recorder sets `window.__lightableFrame`, waits until the app
// has drawn that frame (`window.__lightableRendered`) and saves a lossless
// screenshot; ffmpeg encodes the frames afterwards (encoding alongside slows
// the screenshots down). Demo time is the frame number / 30, so the video
// is smooth however slowly the page renders.
//
// Needs Chrome and ffmpeg (with libx264). Also writes OUT.network.json:
// every request the page made. Development only.

import { spawn } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const [url, out, w, h] = process.argv.slice(2);
const FPS = 30;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const port = 9347;

const chrome = spawn("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", [
  `--remote-debugging-port=${port}`, `--user-data-dir=${mkdtempSync(join(tmpdir(), "rec-"))}`,
  "--no-first-run", "--no-default-browser-check", "--window-size=1400,1000",
  // Keep drawing at full speed when the window is not in front.
  "--disable-background-timer-throttling", "--disable-renderer-backgrounding",
  "--disable-backgrounding-occluded-windows", "about:blank",
], { stdio: "ignore" });
let target;
for (let i = 0; i < 60 && !target; i++) {
  try {
    target = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find((t) => t.type === "page");
  } catch {}
  if (!target) await sleep(500);
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
  if (d.method === "Runtime.exceptionThrown") errors.push(JSON.stringify(d.params.exceptionDetails.text));
});
const send = (method, params = {}) => new Promise((r) => {
  const n = ++id;
  pending.set(n, r);
  ws.send(JSON.stringify({ id: n, method, params }));
});
const evaluate = async (expression) =>
  (await send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true })).result?.value;

await send("Network.enable");
await send("Page.enable");
await send("Page.bringToFront");
await send("Runtime.enable");
await send("Emulation.setDeviceMetricsOverride", { width: Number(w), height: Number(h), deviceScaleFactor: 2, mobile: false });
await send("Page.navigate", { url });

// Wait for the app, then for the demo plan (built at frame 0).
for (let i = 0; i < 600; i++) {
  if (await evaluate("!document.getElementById('loading')")) break;
  await sleep(100);
}
await evaluate("window.__lightableFrame = 0");
await send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "Shift", code: "ShiftLeft", windowsVirtualKeyCode: 16 });
await send("Input.dispatchKeyEvent", { type: "keyUp", key: "Shift", code: "ShiftLeft", windowsVirtualKeyCode: 16 });
let length;
for (let i = 0; i < 600 && !length; i++) {
  length = await evaluate("window.__lightableDemoLength");
  if (!length) {
    await send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "Shift", code: "ShiftLeft", windowsVirtualKeyCode: 16 });
    await send("Input.dispatchKeyEvent", { type: "keyUp", key: "Shift", code: "ShiftLeft", windowsVirtualKeyCode: 16 });
    await sleep(100);
  }
}
if (!length) throw new Error("the demo did not start (is ?demo&record in the URL?)");
const frames = Math.ceil(length * FPS);

const dir = mkdtempSync(join(tmpdir(), "frames-"));
const started = Date.now();
for (let f = 0; f < frames; f++) {
  await evaluate(`window.__lightableFrame = ${f}`);
  // The app draws only when asked: a Shift tap (no visible effect) makes
  // it draw this frame.
  for (let i = 0; i < 400; i++) {
    if (i % 20 === 0) {
      await send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "Shift", code: "ShiftLeft", windowsVirtualKeyCode: 16 });
      await send("Input.dispatchKeyEvent", { type: "keyUp", key: "Shift", code: "ShiftLeft", windowsVirtualKeyCode: 16 });
    }
    if ((await evaluate("window.__lightableRendered")) === f) break;
    await sleep(5);
  }
  const shot = await send("Page.captureScreenshot", { format: "png" });
  writeFileSync(join(dir, `f${String(f).padStart(5, "0")}.png`), Buffer.from(shot.data, "base64"));
  if (f % 300 === 0) process.stderr.write(`frame ${f}/${frames}\n`);
}
chrome.kill();
const ffmpeg = spawn("ffmpeg", [
  "-y", "-loglevel", "error", "-framerate", String(FPS), "-i", join(dir, "f%05d.png"),
  "-c:v", "libx264", "-preset", "slow", "-tune", "animation", "-crf", "16", "-pix_fmt", "yuv420p",
  "-r", String(FPS), "-movflags", "+faststart", "-an", out,
], { stdio: ["ignore", "inherit", "inherit"] });
const code = await new Promise((r) => ffmpeg.on("close", r));
rmSync(dir, { recursive: true, force: true });
writeFileSync(`${out}.network.json`, JSON.stringify(requests, null, 1));
const origin = new URL(url).origin;
console.log(JSON.stringify({
  out, frames, seconds: frames / FPS, ffmpeg: code, recordSeconds: Math.round((Date.now() - started) / 1000),
  requests: requests.length,
  toOtherOrigins: requests.filter((r) => !r.url.startsWith(origin) && !r.url.startsWith("data:")).map((r) => r.url),
  withBody: requests.filter((r) => r.method !== "GET" || r.body > 0).map((r) => `${r.method} ${r.url}`),
  errors,
}));
process.exit(code === 0 ? 0 : 1);
