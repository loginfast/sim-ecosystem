//! Whole-population history, independent of regional selection and window lifetime.
use std::collections::VecDeque;

use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    ecs::schedule::ScheduleLabel,
    prelude::*,
    window::{PrimaryWindow, WindowRef, WindowResolution},
};
use bevy_egui::{EguiContext, EguiSchedule, egui};

use crate::{
    creature::{Creature, CreatureBorn, CreatureDied, CreatureSimulation, Genes, Stats},
    sim::SimParams,
};

const SAMPLE_INTERVAL: f64 = 0.5;
const MAX_FRAMES: usize = 1800;
const MAX_OBSERVATIONS: usize = 200_000;
const MAX_FRAME_OBSERVATIONS: usize = 5000;
const DRAW_OBSERVATIONS: usize = 6000;
const DIET_BANDS: usize = 8;
const RATE_WINDOW: f64 = 60.0;
const GENE_NAMES: [&str; 4] = ["Speed", "Size", "Vision", "Attack"];
const COLORS: [egui::Color32; 4] = [
    egui::Color32::from_rgb(70, 180, 255),
    egui::Color32::from_rgb(255, 175, 65),
    egui::Color32::from_rgb(140, 220, 150),
    egui::Color32::from_rgb(200, 150, 255),
];

pub(super) struct EvolutionPlugin;

impl Plugin for EvolutionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EvolutionWindow>()
            .init_resource::<History>()
            .init_resource::<Tallies>()
            .init_resource::<View>()
            .add_observer(record_birth)
            .add_observer(record_death)
            .add_systems(Update, manage_window)
            .add_systems(FixedUpdate, measure_exposure.before(CreatureSimulation))
            .add_systems(FixedUpdate, sample_population.after(CreatureSimulation))
            .add_systems(EvolutionContextPass, evolution_ui);
    }
}

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct EvolutionContextPass;

#[derive(Component)]
#[relationship(relationship_target = EvolutionCameras)]
struct EvolutionCamera(Entity);

// Ownership without a spatial parent: native windows have no transform/visibility.
#[derive(Component)]
#[relationship_target(relationship = EvolutionCamera, linked_spawn)]
struct EvolutionCameras(Vec<Entity>);

#[derive(Resource)]
pub(super) struct EvolutionWindow {
    entity: Option<Entity>,
    requested: bool,
}

impl Default for EvolutionWindow {
    fn default() -> Self {
        Self {
            entity: None,
            requested: true,
        }
    }
}

impl EvolutionWindow {
    pub(super) fn request_open(&mut self) {
        self.requested = true;
    }
}

fn manage_window(
    mut commands: Commands,
    mut state: ResMut<EvolutionWindow>,
    windows: Query<(), With<Window>>,
    primary: Query<(), (With<Window>, With<PrimaryWindow>)>,
) {
    if state.entity.is_some_and(|e| !windows.contains(e)) {
        state.entity = None;
    }
    if !state.requested || primary.is_empty() {
        return;
    }
    state.requested = false;
    if state.entity.is_some() {
        return;
    }
    let window = commands
        .spawn(Window {
            title: "Terrarium — Evolution".into(),
            resolution: WindowResolution::new(1200, 900),
            resize_constraints: bevy::window::WindowResizeConstraints {
                min_width: 640.0,
                min_height: 560.0,
                ..default()
            },
            ..default()
        })
        .id();
    // A separate camera/context keeps the simulation's Camera2d queries unambiguous.
    // The plot is projected by egui; this camera renders no simulation entities.
    commands.spawn((
        Camera3d::default(),
        RenderLayers::none(),
        RenderTarget::Window(WindowRef::Entity(window)),
        EguiSchedule::new(EvolutionContextPass),
        EvolutionCamera(window),
    ));
    state.entity = Some(window);
}

fn diet_band(diet: f32) -> usize {
    (((diet + 1.0) * 0.5 * DIET_BANDS as f32) as usize).min(DIET_BANDS - 1)
}

#[derive(Default, Clone, Copy)]
struct BandStats {
    born: u32,
    offspring: u32,
    died: u32,
    creature_seconds: f64,
}

impl BandStats {
    fn add(&mut self, other: Self) {
        self.born += other.born;
        self.offspring += other.offspring;
        self.died += other.died;
        self.creature_seconds += other.creature_seconds;
    }

    fn births_per_creature_minute(self) -> Option<f64> {
        (self.creature_seconds > 0.0).then(|| 60.0 * self.offspring as f64 / self.creature_seconds)
    }
}

#[derive(Resource, Default)]
struct Tallies([BandStats; DIET_BANDS]);

fn record_birth(event: On<CreatureBorn>, mut tallies: ResMut<Tallies>) {
    tallies.0[diet_band(event.offspring_diet)].born += 1;
    tallies.0[diet_band(event.parent_diet)].offspring += 1;
}

fn record_death(event: On<CreatureDied>, mut tallies: ResMut<Tallies>) {
    tallies.0[diet_band(event.diet)].died += 1;
}

// Measure the animals alive at the start of the tick, before births/deaths are applied.
fn measure_exposure(
    time: Res<Time<Fixed>>,
    creatures: Query<(&Genes, &Stats), With<Creature>>,
    mut tallies: ResMut<Tallies>,
) {
    let dt = time.delta_secs_f64();
    for (genes, stats) in &creatures {
        if stats.energy > 0.0 {
            tallies.0[diet_band(genes.diet.into_inner() as f32)].creature_seconds += dt;
        }
    }
}

#[derive(Clone, Copy)]
struct Observation {
    entity: Entity,
    diet: f32,
    values: [f32; 4],
}

fn normalize(entity: Entity, genes: &Genes, maxima: [f32; 4]) -> Observation {
    let raw = [
        genes.speed,
        genes.size,
        genes.vision_radius as f32,
        genes.attack,
    ];
    Observation {
        entity,
        diet: genes.diet.into_inner() as f32,
        values: std::array::from_fn(|i| {
            if maxima[i] > 0.0 {
                raw[i] / maxima[i]
            } else {
                0.0
            }
        }),
    }
}

struct Frame {
    time: f64,
    observations: Vec<Observation>,
    alive: [usize; DIET_BANDS],
    events: [BandStats; DIET_BANDS],
}

impl Frame {
    fn population(&self) -> usize {
        self.alive.iter().sum()
    }
}

#[derive(Resource, Default)]
struct History {
    frames: VecDeque<Frame>,
    observations: usize,
    maxima: Option<[f32; 4]>,
    origin: Option<f64>,
    last_sample: Option<f64>,
    ceiling: f32,
}

impl History {
    fn normalization(&mut self, params: &SimParams) -> [f32; 4] {
        *self.maxima.get_or_insert([
            params.speed_max,
            params.size_max,
            params.vision_radius_max as f32,
            params.attack_max,
        ])
    }

    fn push(&mut self, frame: Frame) {
        self.ceiling = frame
            .observations
            .iter()
            .flat_map(|o| o.values)
            .fold(self.ceiling.max(1.0), f32::max);
        self.observations += frame.observations.len();
        self.frames.push_back(frame);
        while self.frames.len() > MAX_FRAMES || self.observations > MAX_OBSERVATIONS {
            let removed = self.frames.pop_front().unwrap();
            self.observations -= removed.observations.len();
        }
    }

    fn nearest(&self, time: f64) -> usize {
        // Binary search is supported by VecDeque even when its storage wraps.
        match self.frames.binary_search_by(|f| f.time.total_cmp(&time)) {
            Ok(i) => i,
            Err(0) => 0,
            Err(i) if i == self.frames.len() => i.saturating_sub(1),
            Err(i) => {
                if time - self.frames[i - 1].time <= self.frames[i].time - time {
                    i - 1
                } else {
                    i
                }
            }
        }
    }

    fn recent_events(&self, selected: usize) -> [BandStats; DIET_BANDS] {
        let mut totals = [BandStats::default(); DIET_BANDS];
        let end = self.frames[selected].time;
        for frame in self.frames.iter().take(selected + 1).rev() {
            if frame.time <= end - RATE_WINDOW {
                break;
            }
            for (total, events) in totals.iter_mut().zip(frame.events) {
                total.add(events);
            }
        }
        totals
    }

    fn y_max(&self) -> f32 {
        self.ceiling.max(1.0)
    }
}

fn sample_population(
    time: Res<Time<Fixed>>,
    params: Res<SimParams>,
    creatures: Query<(Entity, &Genes), With<Creature>>,
    mut tallies: ResMut<Tallies>,
    mut history: ResMut<History>,
) {
    let now = time.elapsed_secs_f64();
    if history
        .last_sample
        .is_some_and(|last| now - last + 1e-9 < SAMPLE_INTERVAL)
    {
        return;
    }
    let maxima = history.normalization(&params);
    let origin = *history.origin.get_or_insert(now);
    let mut alive = [0; DIET_BANDS];
    let count = creatures.iter().count();
    let stride = count.div_ceil(MAX_FRAME_OBSERVATIONS).max(1);
    let mut observations = Vec::with_capacity(count.div_ceil(stride));
    for (i, (entity, genes)) in creatures.iter().enumerate() {
        alive[diet_band(genes.diet.into_inner() as f32)] += 1;
        if i % stride == 0 {
            observations.push(normalize(entity, genes, maxima));
        }
    }
    history.push(Frame {
        time: now - origin,
        observations,
        alive,
        events: std::mem::take(&mut tallies.0),
    });
    history.last_sample = Some(now);
}

#[derive(Resource)]
struct View {
    visible: [bool; 4],
    live: bool,
    selected_time: f64,
    yaw: f32,
    pitch: f32,
    zoom: f32,
}

impl Default for View {
    fn default() -> Self {
        Self {
            visible: [true; 4],
            live: true,
            selected_time: 0.0,
            yaw: -0.6,
            pitch: 0.35,
            zoom: 1.0,
        }
    }
}

fn evolution_ui(
    mut context: Single<&mut EguiContext, With<EvolutionCamera>>,
    history: Res<History>,
    mut view: ResMut<View>,
) {
    draw_ui(context.get_mut(), &history, &mut view);
}

fn draw_ui(context: &egui::Context, history: &History, view: &mut View) {
    let mut viewport = egui::Ui::new(
        context.clone(),
        "evolution_viewport".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(context.content_rect()),
    );
    egui::CentralPanel::default().show(&mut viewport, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Population evolution");
            ui.label("X: diet (−1 meat, +1 plants)  ·  Y: gene / configured maximum  ·  Z: simulation seconds");
            ui.horizontal_wrapped(|ui| {
                for i in 0..4 {
                    ui.checkbox(&mut view.visible[i], egui::RichText::new(GENE_NAMES[i]).color(COLORS[i]));
                }
                ui.separator();
                ui.checkbox(&mut view.live, "Follow latest");
                if ui.button("Reset view").clicked() {
                    let defaults = View::default();
                    view.yaw = defaults.yaw;
                    view.pitch = defaults.pitch;
                    view.zoom = defaults.zoom;
                }
            });
            let (Some(first), Some(last)) = (history.frames.front(), history.frames.back()) else {
                ui.label("Waiting for the first simulation sample…");
                return;
            };
            if view.live { view.selected_time = last.time; }
            view.selected_time = view.selected_time.clamp(first.time, last.time);
            let slider = egui::Slider::new(&mut view.selected_time, first.time..=last.time)
                .text("Time slice (s)").fixed_decimals(1);
            if ui.add(slider).changed() { view.live = false; }
            let selected = history.nearest(view.selected_time);
            let frame = &history.frames[selected];
            ui.label(format!(
                "Slice: {:.1}s · {} creatures     Latest: {:.1}s · {} creatures     Retained: {:.1}–{:.1}s",
                frame.time, frame.population(), last.time, last.population(), first.time, last.time,
            ));
            ui.small("Drag the 3D plot to rotate; wheel to zoom. Hover a point for values. The outlined plane follows the time slice.");
            let y_max = history.y_max();
            let width = ui.available_width();
            if width >= 850.0 {
                ui.horizontal_top(|ui| {
                    ui.allocate_ui(egui::vec2(width * 0.62, 380.0), |ui| {
                        ui.label("3D population history");
                        history_plot(ui, history, view, selected, y_max, 350.0);
                    });
                    ui.allocate_ui(egui::vec2((width * 0.38 - 12.0).max(200.0), 380.0), |ui| {
                        ui.label("Selected time slice");
                        slice_plot(ui, frame, view, y_max, 350.0);
                    });
                });
            } else {
                ui.label("3D population history");
                history_plot(ui, history, view, selected, y_max, 330.0);
                ui.label("Selected time slice");
                slice_plot(ui, frame, view, y_max, 260.0);
            }
            if history.observations > DRAW_OBSERVATIONS || frame.population() > frame.observations.len() {
                ui.small("Dense point clouds are sampled for display. Population and event counts include every creature.");
            }
            let maxima = history.maxima.unwrap();
            ui.small(format!(
                "Fixed normalization: speed / {:.2} · size / {:.2} · vision / {:.0} · attack / {:.2}",
                maxima[0], maxima[1], maxima[2], maxima[3],
            ));
            ui.separator();
            population_table(ui, history, selected);
        });
    });
}

struct Projection3d {
    rotation: Quat,
    center: egui::Pos2,
    scale: f32,
}

impl Projection3d {
    fn new(rect: egui::Rect, view: &View) -> Self {
        let rotation = Quat::from_rotation_x(view.pitch) * Quat::from_rotation_y(view.yaw);
        let mut extent = Vec2::ZERO;
        for x in [-1.0, 1.0] {
            for y in [-1.0, 1.0] {
                for z in [-1.0, 1.0] {
                    let p = rotation * (Vec3::new(x, y, z) * Self::ASPECT);
                    extent = extent.max(p.truncate().abs());
                }
            }
        }
        Self {
            rotation,
            center: rect.center(),
            scale: (rect.width() / (2.0 * extent.x)).min(rect.height() / (2.0 * extent.y))
                * view.zoom,
        }
    }

    const ASPECT: Vec3 = Vec3::new(1.6, 1.0, 2.0);

    fn project(&self, point: Vec3) -> (egui::Pos2, f32) {
        let point = self.rotation * (point * Self::ASPECT);
        (
            self.center + egui::vec2(point.x, -point.y) * self.scale,
            point.z,
        )
    }
}

fn coordinates(diet: f32, gene: f32, time: f64, start: f64, end: f64, y_max: f32) -> Vec3 {
    Vec3::new(
        diet,
        2.0 * gene / y_max - 1.0,
        2.0 * ((time - start) / (end - start).max(SAMPLE_INTERVAL)) as f32 - 1.0,
    )
}

struct PlotPoint {
    position: egui::Pos2,
    depth: f32,
    observation: Observation,
    gene: usize,
    time: f64,
    selected: bool,
}

fn history_plot(
    ui: &mut egui::Ui,
    history: &History,
    view: &mut View,
    selected: usize,
    y_max: f32,
    height: f32,
) {
    let (response, painter) = ui.allocate_painter(
        egui::vec2(ui.available_width(), height),
        egui::Sense::drag(),
    );
    if response.dragged() {
        let motion = response.drag_motion();
        view.yaw += motion.x * 0.008;
        view.pitch = (view.pitch + motion.y * 0.008).clamp(-1.3, 1.3);
    }
    if response.hovered() {
        // Consume the wheel here so the surrounding page does not scroll while zooming.
        let scroll = ui.input_mut(|i| std::mem::take(&mut i.smooth_scroll_delta).y);
        view.zoom = (view.zoom * (scroll * 0.002).exp()).clamp(0.5, 2.2);
    }
    let rect = egui::Rect::from_min_max(
        response.rect.min + egui::vec2(120.0, 45.0),
        response.rect.max - egui::vec2(70.0, 55.0),
    );
    let projection = Projection3d::new(rect, view);
    let start = history.frames.front().unwrap().time;
    let end = history.frames.back().unwrap().time;
    let project = |p| projection.project(p).0;
    let faint = egui::Stroke::new(1.0, ui.visuals().weak_text_color().gamma_multiply(0.4));
    for axis in 0..3 {
        for a in [-1.0, 1.0] {
            for b in [-1.0, 1.0] {
                let mut p = Vec3::splat(-1.0);
                p[(axis + 1) % 3] = a;
                p[(axis + 2) % 3] = b;
                let mut q = p;
                q[axis] = 1.0;
                painter.line_segment([project(p), project(q)], faint);
            }
        }
    }
    let selected_z = coordinates(0.0, 0.0, history.frames[selected].time, start, end, y_max).z;
    let plane = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
        .map(|(x, y)| project(Vec3::new(x, y, selected_z)));
    painter.add(egui::Shape::convex_polygon(
        plane.to_vec(),
        egui::Color32::from_white_alpha(10),
        egui::Stroke::new(1.0, egui::Color32::from_white_alpha(110)),
    ));
    let stride = history.observations.div_ceil(DRAW_OBSERVATIONS).max(1);
    let mut points = Vec::new();
    let mut index = 0;
    for (frame_index, frame) in history.frames.iter().enumerate() {
        for observation in &frame.observations {
            let draw = frame_index == selected || index % stride == 0;
            index += 1;
            if !draw {
                continue;
            }
            for gene in 0..4 {
                if !view.visible[gene] {
                    continue;
                }
                let (position, depth) = projection.project(coordinates(
                    observation.diet,
                    observation.values[gene],
                    frame.time,
                    start,
                    end,
                    y_max,
                ));
                points.push(PlotPoint {
                    position,
                    depth,
                    observation: *observation,
                    gene,
                    time: frame.time,
                    selected: frame_index == selected,
                });
            }
        }
    }
    // Painter order supplies depth sorting for this orthographic 3D projection.
    points.sort_unstable_by(|a, b| a.depth.total_cmp(&b.depth));
    let clipped = painter.with_clip_rect(response.rect);
    let pointer = response.hover_pos();
    let mut nearest = None;
    let mut distance = 64.0;
    for point in &points {
        let alpha = if point.selected { 240 } else { 75 };
        clipped.circle_filled(
            point.position,
            if point.selected { 2.8 } else { 1.5 },
            COLORS[point.gene]
                .to_opaque()
                .linear_multiply(alpha as f32 / 255.0),
        );
        if let Some(pointer) = pointer {
            let d = pointer.distance_sq(point.position);
            if d < distance && response.rect.contains(point.position) {
                distance = d;
                nearest = Some(point);
            }
        }
    }
    draw_axes(
        &painter,
        &projection,
        start,
        end,
        y_max,
        ui.visuals().text_color(),
    );
    if let Some(point) = nearest {
        response.on_hover_text_at_pointer(point_label(point.observation, point.gene, point.time));
    }
}

fn draw_axes(
    painter: &egui::Painter,
    projection: &Projection3d,
    start: f64,
    end: f64,
    y_max: f32,
    ink: egui::Color32,
) {
    let font = egui::FontId::proportional(12.0);
    for axis in 0..3 {
        // Label outer edges: diet below, gene values to the left, time to the right.
        // Re-evaluate after rotation so labels stay outside the cloud.
        let mut edge = (Vec3::ZERO, Vec3::ZERO);
        let mut best = f32::NEG_INFINITY;
        for a in [-1.0, 1.0] {
            for b in [-1.0, 1.0] {
                let mut p = Vec3::splat(-1.0);
                p[(axis + 1) % 3] = a;
                p[(axis + 2) % 3] = b;
                let mut q = p;
                q[axis] = 1.0;
                let mid = projection.project((p + q) * 0.5).0;
                let score = match axis {
                    0 => mid.y,
                    1 => -mid.x,
                    _ => mid.x,
                };
                if score > best {
                    best = score;
                    edge = (p, q);
                }
            }
        }
        let (p, q) = edge;
        let from = projection.project(p).0;
        let to = projection.project(q).0;
        let direction = to - from;
        let mid = from + direction * 0.5;
        let mut normal = egui::vec2(-direction.y, direction.x) / direction.length().max(0.001);
        if normal.dot(mid - projection.center) < 0.0 {
            normal = -normal;
        }
        painter.line_segment([from, to], egui::Stroke::new(1.2, ink.gamma_multiply(0.6)));
        for tick in 0..=2 {
            let t = tick as f32 * 0.5;
            let label = match axis {
                0 => format!("{:.1}", 2.0 * t - 1.0),
                1 => format!("{:.2}", t * y_max),
                _ => format!("{:.1}s", start + t as f64 * (end - start)),
            };
            painter.text(
                from + direction * t + normal * 15.0,
                egui::Align2::CENTER_CENTER,
                label,
                font.clone(),
                ink,
            );
        }
        painter.text(
            mid + normal * if axis == 1 { 76.0 } else { 42.0 },
            egui::Align2::CENTER_CENTER,
            ["Diet", "Gene / max", "Time"][axis],
            font.clone(),
            ink,
        );
    }
}

fn point_label(observation: Observation, gene: usize, time: f64) -> String {
    format!(
        "Creature {:?}\n{} / max: {:.3}\nDiet: {:.3}\nTime: {:.1}s",
        observation.entity, GENE_NAMES[gene], observation.values[gene], observation.diet, time
    )
}

fn slice_plot(ui: &mut egui::Ui, frame: &Frame, view: &View, y_max: f32, height: f32) {
    let (response, painter) = ui.allocate_painter(
        egui::vec2(ui.available_width(), height),
        egui::Sense::hover(),
    );
    let rect = egui::Rect::from_min_max(
        response.rect.min + egui::vec2(48.0, 16.0),
        response.rect.max - egui::vec2(18.0, 40.0),
    );
    let project = |diet: f32, gene: f32| {
        egui::pos2(
            rect.left() + (diet + 1.0) * 0.5 * rect.width(),
            rect.bottom() - gene / y_max * rect.height(),
        )
    };
    let font = egui::FontId::proportional(12.0);
    let ink = ui.visuals().text_color();
    let grid = egui::Stroke::new(1.0, ui.visuals().weak_text_color().gamma_multiply(0.25));
    for tick in 0..=4 {
        let t = tick as f32 * 0.25;
        let diet = 2.0 * t - 1.0;
        let y = t * y_max;
        painter.line_segment([project(diet, 0.0), project(diet, y_max)], grid);
        painter.line_segment([project(-1.0, y), project(1.0, y)], grid);
        painter.text(
            project(diet, 0.0) + egui::vec2(0.0, 8.0),
            egui::Align2::CENTER_TOP,
            format!("{diet:.1}"),
            font.clone(),
            ink,
        );
        painter.text(
            project(-1.0, y) - egui::vec2(8.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            format!("{y:.2}"),
            font.clone(),
            ink,
        );
    }
    painter.text(
        egui::pos2(rect.center().x, rect.bottom() + 30.0),
        egui::Align2::CENTER_CENTER,
        "Diet (-1 meat, +1 plants)",
        font,
        ink,
    );
    let clipped = painter.with_clip_rect(rect.expand(4.0));
    let pointer = response.hover_pos();
    let mut nearest = None;
    let mut distance = 64.0;
    for observation in &frame.observations {
        for gene in 0..4 {
            if !view.visible[gene] {
                continue;
            }
            let position = project(observation.diet, observation.values[gene]);
            clipped.circle_filled(position, 3.0, COLORS[gene].linear_multiply(0.8));
            if let Some(pointer) = pointer {
                let d = pointer.distance_sq(position);
                if d < distance {
                    distance = d;
                    nearest = Some((*observation, gene));
                }
            }
        }
    }
    if let Some((observation, gene)) = nearest {
        response.on_hover_text_at_pointer(point_label(observation, gene, frame.time));
    }
}

fn population_table(ui: &mut egui::Ui, history: &History, selected: usize) {
    let frame = &history.frames[selected];
    let events = history.recent_events(selected);
    ui.label("Survival and reproduction by diet");
    ui.small("Events over the preceding 60 simulation seconds (or retained history). Birth rate is offspring per creature-minute, attributed to the parent's diet.");
    egui::Grid::new("evolution_diet_bands")
        .striped(true)
        .show(ui, |ui| {
            for label in ["Diet band", "Alive at slice", "Born", "Birth rate", "Died"] {
                ui.label(label);
            }
            ui.end_row();
            for (i, stats) in events.into_iter().enumerate() {
                let low = -1.0 + i as f32 * 2.0 / DIET_BANDS as f32;
                let high = low + 2.0 / DIET_BANDS as f32;
                ui.label(format!("{low:+.2} … {high:+.2}"));
                ui.label(frame.alive[i].to_string());
                ui.label(stats.born.to_string());
                ui.label(
                    stats
                        .births_per_creature_minute()
                        .map_or("—".into(), |v| format!("{v:.2}")),
                );
                ui.label(stats.died.to_string());
                ui.end_row();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ranged::SignedUnitInterval;
    use std::time::Duration;

    fn genes() -> Genes {
        Genes {
            speed: 5.0,
            size: 0.475,
            diet: SignedUnitInterval::try_new(-0.75).unwrap(),
            vision_radius: 4,
            attack: 15.0,
        }
    }

    fn frame(time: f64, count: usize) -> Frame {
        let observation = normalize(Entity::PLACEHOLDER, &genes(), [10.0, 0.95, 32.0, 30.0]);
        Frame {
            time,
            observations: vec![observation; count],
            alive: [count; DIET_BANDS],
            events: [BandStats::default(); DIET_BANDS],
        }
    }

    #[test]
    fn normalization_uses_configured_maxima_and_stays_fixed_for_history() {
        let mut history = History::default();
        let mut params = SimParams::default();
        let maxima = history.normalization(&params);
        let observation = normalize(Entity::PLACEHOLDER, &genes(), maxima);
        assert_eq!(observation.values, [0.5, 0.5, 0.125, 0.5]);
        assert_eq!(observation.diet, -0.75);
        params.speed_max *= 2.0;
        assert_eq!(history.normalization(&params), maxima);
        let guarded = normalize(observation.entity, &genes(), [0.0; 4]);
        assert_eq!(guarded.values, [0.0; 4]);
    }

    #[test]
    fn history_bounds_memory_and_retains_empty_population_samples() {
        let mut history = History::default();
        for i in 0..=MAX_OBSERVATIONS / MAX_FRAME_OBSERVATIONS {
            history.push(frame(i as f64 * SAMPLE_INTERVAL, MAX_FRAME_OBSERVATIONS));
        }
        assert_eq!(history.observations, MAX_OBSERVATIONS);
        assert_eq!(history.frames.front().unwrap().time, SAMPLE_INTERVAL);
        assert_eq!(history.nearest(-1.0), 0);
        assert_eq!(history.nearest(f64::MAX), history.frames.len() - 1);
        assert_eq!(history.frames[history.nearest(1.1)].time, 1.0);
        for i in 0..MAX_FRAMES {
            history.push(frame(100.0 + i as f64 * SAMPLE_INTERVAL, 0));
        }
        assert_eq!(history.frames.len(), MAX_FRAMES);
        assert_eq!(history.observations, 0);
        assert_eq!(history.frames.back().unwrap().population(), 0);
    }

    #[test]
    fn birth_and_death_statistics_include_events_between_samples_and_parent_diet() {
        let mut app = App::new();
        app.init_resource::<Tallies>()
            .init_resource::<History>()
            .init_resource::<SimParams>()
            .insert_resource(Time::<Fixed>::from_seconds(0.25))
            .add_observer(record_birth)
            .add_observer(record_death)
            .add_systems(FixedUpdate, (measure_exposure, sample_population).chain());
        app.world_mut().spawn((
            Creature,
            genes(),
            Stats {
                energy: 100.0,
                reproduction_ticks_remaining: 10,
            },
        ));
        app.world_mut()
            .resource_mut::<Time<Fixed>>()
            .advance_by(Duration::from_secs_f64(0.25));
        app.world_mut().run_schedule(FixedUpdate);
        app.world_mut().trigger(CreatureBorn {
            parent_diet: -0.75,
            offspring_diet: 0.75,
        });
        app.world_mut().trigger(CreatureDied { diet: 0.75 });
        for _ in 0..2 {
            app.world_mut()
                .resource_mut::<Time<Fixed>>()
                .advance_by(Duration::from_secs_f64(0.25));
            app.world_mut().run_schedule(FixedUpdate);
        }
        let history = app.world().resource::<History>();
        assert_eq!(history.frames.len(), 2);
        let events = history.recent_events(1);
        assert_eq!(events[diet_band(-0.75)].offspring, 1);
        assert_eq!(events[diet_band(0.75)].born, 1);
        assert_eq!(events[diet_band(0.75)].died, 1);
        assert_eq!(
            events[diet_band(-0.75)].births_per_creature_minute(),
            Some(80.0)
        );
        assert_eq!(events[diet_band(0.75)].births_per_creature_minute(), None);
        assert_eq!(diet_band(-1.0), 0);
        assert_eq!(diet_band(1.0), DIET_BANDS - 1);
    }

    #[test]
    fn diet_statistics_do_not_include_the_future_or_events_older_than_sixty_seconds() {
        let mut history = History::default();
        for time in [0.0, 0.5, 60.0, 60.5, 61.0] {
            let mut frame = frame(time, 0);
            frame.events[0].born = 1;
            history.push(frame);
        }
        assert_eq!(history.recent_events(0)[0].born, 1);
        assert_eq!(history.recent_events(3)[0].born, 2);
        assert_eq!(history.recent_events(4)[0].born, 3);
    }

    #[test]
    fn closing_and_reopening_the_native_window_preserves_history_and_the_simulation() {
        let mut app = App::new();
        app.init_resource::<EvolutionWindow>()
            .init_resource::<History>()
            .add_systems(Update, manage_window);
        let primary = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        app.world_mut()
            .resource_mut::<History>()
            .push(frame(0.0, 1));
        app.update();
        let window = app.world().resource::<EvolutionWindow>().entity.unwrap();
        assert_ne!(window, primary);
        let mut cameras = app
            .world_mut()
            .query_filtered::<Entity, With<EvolutionCamera>>();
        let camera = cameras.single(app.world()).unwrap();
        assert!(app.world().get::<Camera2d>(camera).is_none());
        assert!(
            matches!(app.world().get::<RenderTarget>(camera), Some(RenderTarget::Window(WindowRef::Entity(e))) if *e == window)
        );
        app.world_mut().despawn(window);
        assert!(app.world().get_entity(camera).is_err());
        app.update();
        assert!(app.world().resource::<EvolutionWindow>().entity.is_none());
        app.world_mut()
            .resource_mut::<EvolutionWindow>()
            .request_open();
        app.update();
        assert!(
            app.world()
                .resource::<EvolutionWindow>()
                .entity
                .is_some_and(|e| e != window)
        );
        assert!(app.world().get::<Window>(primary).is_some());
        assert_eq!(app.world().resource::<History>().frames.len(), 1);
    }

    #[test]
    fn plot_layout_and_rotation_produce_finite_geometry_at_both_window_sizes() {
        let mut history = History::default();
        history.maxima = Some([10.0, 0.95, 32.0, 30.0]);
        for time in [0.0, 0.5, 1.0] {
            history.push(frame(time, 2));
        }
        let context = egui::Context::default();
        let mut view = View::default();
        for width in [640.0, 1200.0] {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 900.0),
                )),
                ..default()
            };
            let mut output = context.run_ui(input, |_| draw_ui(&context, &history, &mut view));
            output.textures_delta.clear();
            let primitives = context.tessellate(output.shapes, output.pixels_per_point);
            assert!(!primitives.is_empty());
            for primitive in primitives {
                if let egui::epaint::Primitive::Mesh(mesh) = primitive.primitive {
                    assert!(
                        mesh.vertices
                            .iter()
                            .all(|v| v.pos.x.is_finite() && v.pos.y.is_finite())
                    );
                }
            }
        }
        assert_eq!(view.selected_time, 1.0);
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 350.0));
        let point = coordinates(-0.75, 0.5, 0.5, 0.0, 1.0, 1.0);
        let before = Projection3d::new(rect, &view).project(point).0;
        view.yaw += 0.5;
        let after = Projection3d::new(rect, &view).project(point).0;
        assert!(before.distance(after) > 1.0);
        assert!(coordinates(0.0, 0.5, 0.0, 0.0, 0.0, 1.0).is_finite());
    }
}
