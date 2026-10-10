//! The viewer: a canvas with pan and zoom, a layer panel and a status bar.

use std::sync::{Arc, Mutex};

use eframe::egui::{
    self, Align2, Color32, FontId, PointerButton, Pos2, Rect, Sense, epaint::TextShape,
};
use eframe::{egui_glow, glow};

use cad_render::{HAlign, Scene, Shape, VAlign, View, tess};

use crate::gpu::{DrawStats, GpuScene};

/// Grid size for GPU chunks (per layer): finer means better culling and
/// precision, coarser means fewer draw calls.
const GRID: usize = 16;
/// Text smaller than this on screen is not drawn.
const MIN_TEXT_PX: f64 = 4.0;
/// At most this many texts per frame (zoomed far out over dense text).
const MAX_TEXTS: usize = 5000;
pub const BACKGROUND: Color32 = Color32::from_rgb(0x21, 0x21, 0x21);

struct Doc {
    name: String,
    scene: Arc<Scene>,
    /// Per chunk: layer, bounds and the text items in it.
    texts: Vec<(u32, Option<cad_render::Bounds>, Vec<u32>)>,
    gpu: Option<Arc<Mutex<GpuScene>>>,
    visible: Vec<bool>,
    warnings: usize,
    stats: Arc<Mutex<DrawStats>>,
    vertices: usize,
}

/// Animated pan/zoom for `--bench`, reporting frame times at the end.
pub struct Bench {
    pub frames: usize,
    done: usize,
    times: Vec<f64>,
    #[cfg(not(target_arch = "wasm32"))]
    shot: Option<String>,
}

impl Bench {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(frames: usize) -> Self {
        Self {
            frames,
            done: 0,
            times: Vec::new(),
            shot: None,
        }
    }

    /// Instead of animating: after a few frames, save a screenshot of the
    /// window (binary PPM) and quit.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn screenshot(path: String) -> Self {
        Self {
            frames: 0,
            done: 0,
            times: Vec::new(),
            shot: Some(path),
        }
    }
}

/// A file read in the background: name and bytes.
type FileSlot = Arc<Mutex<Option<(String, Vec<u8>)>>>;

pub struct CadApp {
    doc: Option<Doc>,
    view: View,
    fit_pending: bool,
    message: Option<String>,
    frame_times: Vec<f64>,
    bench: Option<Bench>,
    pending: Option<(String, Vec<u8>)>,
    /// Files read in the background (the web reads dropped files asynchronously).
    incoming: FileSlot,
}

impl CadApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        file: Option<(String, Vec<u8>)>,
        bench: Option<Bench>,
    ) -> Self {
        cc.egui_ctx.set_theme(egui::ThemePreference::Dark);
        Self {
            doc: None,
            view: View::new([800.0, 600.0]),
            fit_pending: true,
            message: None,
            frame_times: Vec::new(),
            bench,
            pending: file,
            incoming: Arc::default(),
        }
    }

    fn load(&mut self, gl: Option<&Arc<glow::Context>>, name: String, bytes: &[u8]) {
        let (drawing, warnings) = match cad_dxf::read_bytes(bytes) {
            Ok(r) => r,
            Err(e) => {
                self.message = Some(format!("{name}: {e}"));
                return;
            }
        };
        let scene = cad_render::build(&drawing);
        let geometry = tess::tessellate(&scene, GRID);
        if let (Some(gl), Some(old)) = (gl, self.doc.take()) {
            if let Some(gpu) = old.gpu {
                gpu.lock().map(|g| g.destroy(gl)).ok();
            }
        }
        let gpu = match gl.map(|gl| GpuScene::new(gl, &geometry)) {
            Some(Ok(g)) => Some(Arc::new(Mutex::new(g))),
            Some(Err(e)) => {
                self.message = Some(format!("graphics error: {e}"));
                None
            }
            None => None,
        };
        let texts = geometry
            .chunks
            .iter()
            .filter(|c| !c.texts.is_empty())
            .map(|c| (c.layer, c.bounds, c.texts.clone()))
            .collect();
        self.doc = Some(Doc {
            name,
            visible: scene.default_visibility(),
            vertices: geometry.vertex_count(),
            scene: Arc::new(scene),
            texts,
            gpu,
            warnings: warnings.len(),
            stats: Arc::default(),
        });
        self.fit_pending = true;
        if self
            .message
            .as_deref()
            .is_some_and(|m| !m.starts_with("graphics"))
        {
            self.message = None;
        }
    }

    fn fit(&mut self) {
        if let Some(doc) = &self.doc
            && let Some(b) = doc.scene.bounds(Some(&doc.visible))
        {
            self.view.fit(&b, 0.03);
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            match &self.doc {
                Some(doc) => ui.strong(&doc.name),
                None => ui.label("Drop a DXF file here (or pass one on the command line)"),
            };
            if ui.button("Zoom extents").clicked() {
                self.fit();
            }
            if let Some(doc) = &self.doc {
                let stats = doc.stats.lock().map(|s| *s).unwrap_or_default();
                ui.separator();
                ui.label(format!(
                    "{} items, {} vertices",
                    doc.scene.items.len(),
                    doc.vertices
                ));
                ui.label(format!(
                    "drawn: {} chunks, {} vertices",
                    stats.chunks, stats.vertices
                ));
                if doc.warnings > 0 {
                    ui.label(format!("{} read warnings", doc.warnings));
                }
            }
            if !self.frame_times.is_empty() {
                let avg = self.frame_times.iter().sum::<f64>() / self.frame_times.len() as f64;
                ui.separator();
                ui.label(format!("{:.1} ms/frame", avg * 1000.0));
            }
        });
    }

    fn layer_panel(&mut self, ui: &mut egui::Ui) {
        let Some(doc) = &mut self.doc else {
            ui.label("No drawing");
            return;
        };
        ui.horizontal(|ui| {
            if ui.button("All on").clicked() {
                doc.visible.iter_mut().for_each(|v| *v = true);
            }
            if ui.button("All off").clicked() {
                doc.visible.iter_mut().for_each(|v| *v = false);
            }
        });
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, layer) in doc.scene.layers.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut doc.visible[i], "");
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), Sense::hover());
                    let c = layer.color;
                    ui.painter()
                        .rect_filled(rect, 2.0, Color32::from_rgb(c.0, c.1, c.2));
                    ui.label(&layer.name);
                });
            }
        });
    }

    fn canvas(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        self.view.size = [f64::from(rect.width()), f64::from(rect.height())];
        if self.fit_pending && self.doc.is_some() && rect.width() > 0.0 {
            self.fit();
            self.fit_pending = false;
        }
        let local = |p: Pos2| [f64::from(p.x - rect.min.x), f64::from(p.y - rect.min.y)];
        if response.dragged_by(PointerButton::Middle) || response.dragged_by(PointerButton::Primary)
        {
            let d = response.drag_delta();
            self.view.pan(f64::from(d.x), f64::from(d.y));
        }
        if response.double_clicked_by(PointerButton::Middle) {
            self.fit();
        }
        if let Some(pos) = response.hover_pos() {
            let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
            let factor = f64::from(scroll * 0.0025).exp() * f64::from(pinch);
            if factor != 1.0 {
                self.view.zoom_at(local(pos), factor);
            }
        }

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, BACKGROUND);
        let Some(doc) = &self.doc else { return };
        if let Some(gpu) = &doc.gpu {
            let gpu = gpu.clone();
            let view = self.view;
            let visible = doc.visible.clone();
            let stats = doc.stats.clone();
            let ppp = ui.ctx().pixels_per_point();
            let callback = egui_glow::CallbackFn::new(move |_info, painter| {
                if let Ok(gpu) = gpu.lock() {
                    let s = gpu.draw(painter.gl(), &view, &visible, ppp);
                    if let Ok(mut out) = stats.lock() {
                        *out = s;
                    }
                }
            });
            painter.add(egui::PaintCallback {
                rect,
                callback: Arc::new(callback),
            });
        }
        draw_texts(&painter, rect, &self.view, doc);
    }
}

fn anchor(t: &cad_render::TextShape) -> Align2 {
    match (t.halign, t.valign) {
        (HAlign::Left, VAlign::Top) => Align2::LEFT_TOP,
        (HAlign::Center, VAlign::Top) => Align2::CENTER_TOP,
        (HAlign::Right, VAlign::Top) => Align2::RIGHT_TOP,
        (HAlign::Left, VAlign::Middle) => Align2::LEFT_CENTER,
        (HAlign::Center, VAlign::Middle) => Align2::CENTER_CENTER,
        (HAlign::Right, VAlign::Middle) => Align2::RIGHT_CENTER,
        (HAlign::Left, _) => Align2::LEFT_BOTTOM,
        (HAlign::Center, _) => Align2::CENTER_BOTTOM,
        (HAlign::Right, _) => Align2::RIGHT_BOTTOM,
    }
}

/// Draws the texts that are on screen and large enough to read.
fn draw_texts(painter: &egui::Painter, rect: Rect, view: &View, doc: &Doc) {
    let shown = view.world_bounds();
    let mut count = 0;
    for (layer, bounds, items) in &doc.texts {
        if !doc.visible.get(*layer as usize).copied().unwrap_or(true)
            || bounds.is_some_and(|b| !b.intersects(&shown))
        {
            continue;
        }
        for &i in items {
            let item = &doc.scene.items[i as usize];
            let Shape::Text(t) = &item.shape else {
                continue;
            };
            let px = t.height * view.scale;
            if px < MIN_TEXT_PX {
                continue;
            }
            if item.shape.bounds().is_some_and(|b| !b.intersects(&shown)) {
                continue;
            }
            let color = Color32::from_rgb(item.color.0, item.color.1, item.color.2);
            // CAD text height is the cap height, about 0.7 of the font size.
            let size = (px / 0.7).min(2000.0) as f32;
            let galley = painter.layout_no_wrap(t.text.clone(), FontId::proportional(size), color);
            let p = view.to_screen(t.at);
            let pos = Pos2::new(rect.min.x + p[0] as f32, rect.min.y + p[1] as f32);
            painter.add(
                TextShape::new(pos, galley, color)
                    .with_angle_and_anchor(-t.rotation as f32, anchor(t)),
            );
            count += 1;
            if count >= MAX_TEXTS {
                return;
            }
        }
    }
}

impl eframe::App for CadApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let dt = ui.input(|i| f64::from(i.unstable_dt));
        self.frame_times.push(dt);
        if self.frame_times.len() > 60 {
            self.frame_times.remove(0);
        }
        if let Some((name, bytes)) = self.pending.take() {
            self.load(frame.gl(), name, &bytes);
        }
        let dropped = ui.input(|i| i.raw.dropped_files.clone());
        for file in dropped {
            let name = file.path().display().to_string();
            #[cfg(not(target_arch = "wasm32"))]
            match file.bytes() {
                Ok(bytes) => self.load(frame.gl(), name, &bytes),
                Err(e) => self.message = Some(format!("{name}: {e}")),
            }
            #[cfg(target_arch = "wasm32")]
            {
                // Browsers read files asynchronously; the bytes arrive in a
                // later frame through `incoming`.
                let incoming = self.incoming.clone();
                let ctx = ui.ctx().clone();
                wasm_bindgen_futures::spawn_local(async move {
                    if let Ok(bytes) = file.bytes_async().await
                        && let Ok(mut slot) = incoming.lock()
                    {
                        *slot = Some((name, bytes));
                        ctx.request_repaint();
                    }
                });
            }
        }
        let arrived = self.incoming.lock().ok().and_then(|mut slot| slot.take());
        if let Some((name, bytes)) = arrived {
            self.load(frame.gl(), name, &bytes);
        }

        egui::Panel::top("toolbar").show(ui, |ui| self.toolbar(ui));
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(m) = &self.message {
                    ui.colored_label(Color32::LIGHT_RED, m);
                }
                if let Some(doc) = &self.doc
                    && !doc.scene.skipped.is_empty()
                {
                    let skipped: Vec<String> = doc
                        .scene
                        .skipped
                        .iter()
                        .map(|(k, n)| format!("{n} {k}"))
                        .collect();
                    ui.label(format!("not drawn: {}", skipped.join(", ")));
                }
                if let Some(p) = ui.input(|i| i.pointer.hover_pos()) {
                    let w = self.view.to_world([f64::from(p.x), f64::from(p.y)]);
                    ui.label(format!("{:.4}, {:.4}", w[0], w[1]));
                }
            });
        });
        egui::Panel::left("layers")
            .resizable(true)
            .default_size(200.0)
            .show(ui, |ui| {
                ui.heading("Layers");
                self.layer_panel(ui);
            });
        egui::CentralPanel::no_frame().show(ui, |ui| self.canvas(ui));

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(bench) = &mut self.bench
            && let Some(path) = bench.shot.clone()
        {
            ui.ctx().request_repaint();
            bench.done += 1;
            if bench.done == 5 {
                ui.ctx()
                    .send_viewport_cmd(
                        egui::ViewportCommand::Screenshot(egui::UserData::default()),
                    );
            }
            let image = ui.input(|i| {
                i.raw.events.iter().find_map(|e| match e {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            });
            if let Some(image) = image {
                let mut ppm =
                    format!("P6\n{} {}\n255\n", image.size[0], image.size[1]).into_bytes();
                for p in &image.pixels {
                    ppm.extend_from_slice(&[p.r(), p.g(), p.b()]);
                }
                if let Err(e) = std::fs::write(&path, ppm) {
                    eprintln!("{path}: {e}");
                }
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
            return;
        }
        if let Some(bench) = &mut self.bench {
            ui.ctx().request_repaint();
            if self.doc.is_some() {
                bench.done += 1;
                // Skip the first frames (loading, first upload).
                if bench.done > 10 {
                    bench.times.push(dt);
                }
                let t = bench.done as f64;
                let factor = if (bench.done / 60) % 2 == 0 {
                    1.03
                } else {
                    1.0 / 1.03
                };
                let c = [
                    self.view.size[0] / 2.0 + 100.0 * (t / 20.0).sin(),
                    self.view.size[1] / 2.0,
                ];
                self.view.zoom_at(c, factor);
                self.view.pan(3.0 * (t / 15.0).cos(), 2.0);
                if bench.done == bench.frames + 10 {
                    report(bench, self.doc.as_ref());
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
    }
}

fn report(bench: &Bench, doc: Option<&Doc>) {
    let mut times = bench.times.clone();
    times.sort_by(|a, b| a.total_cmp(b));
    let n = times.len().max(1);
    let avg = times.iter().sum::<f64>() / n as f64;
    let p95 = times[(n * 95 / 100).min(n - 1)];
    let worst = times.last().copied().unwrap_or(0.0);
    if let Some(doc) = doc {
        println!(
            "{}: {} items, {} vertices",
            doc.name,
            doc.scene.items.len(),
            doc.vertices
        );
    }
    println!(
        "{} frames of pan/zoom: average {:.2} ms ({:.0} fps), 95th percentile {:.2} ms, worst {:.2} ms",
        n,
        avg * 1000.0,
        1.0 / avg.max(1e-9),
        p95 * 1000.0,
        worst * 1000.0
    );
}
