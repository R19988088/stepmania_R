use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use std::time::Duration;
use std::{collections::HashMap, fs};

use lewton::inside_ogg::OggStreamReader;
use macroquad::prelude::*;
use macroquad::miniquad::{BlendFactor, BlendState, Equation, PipelineParams};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source, buffer::SamplesBuffer};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::chart::{Chart, NoteKind, beat_to_seconds, seconds_to_beat};
use crate::log_file;

const LOGICAL_W: f32 = 1280.0;
const LOGICAL_H: f32 = 2560.0;
const WINDOW_W: i32 = 960;
const WINDOW_H: i32 = 1920;
const LANE_COUNT: usize = 4;
const PLAYFIELD_WIDTH_RATIO: f32 = 0.56;
const NOTEFIELD_SCALE: f32 = 1.5;
const RECEPTOR_Y_RATIO: f32 = 0.125;
const SCROLL_PX_PER_BEAT: f32 = 240.0;
const ARROW_SIZE_RATIO: f32 = 0.92;
const NOTE_ANIM_FPS: f64 = 9.0;
const GAME_SPEED_MIN: f32 = 1.0;
const GAME_SPEED_MAX: f32 = 5.0;
const GAME_SPEED_STEP: f32 = 0.1;
const SPEED_MIN: f32 = 0.1;
const SPEED_MAX: f32 = 2.0;
const SPEED_STEP: f32 = 0.1;
const BGM_VOL_MIN: f32 = 0.0;
const BGM_VOL_MAX: f32 = 2.0;
const BGM_VOL_STEP: f32 = 0.1;
const SFX_VOL_MIN: f32 = 0.0;
const SFX_VOL_MAX: f32 = 2.0;
const SFX_VOL_STEP: f32 = 0.1;
const TAP_SFX_BASE_VOLUME: f32 = 1.65;
const HIT_FX_FRAME_SEC: f32 = 0.06;
const HIT_FX_DURATION_SEC: f32 = HIT_FX_FRAME_SEC * 2.0;

const ARROW_CANDIDATES: [&str; 4] = [
    "mini_stepmania_rust/NoteSkins/common/common/_arrow 1x8 (doubleres).png",
    "mini_stepmania_rust/NoteSkins/common/common/_arrow 1x8.png",
    "NoteSkins/common/common/_arrow 1x8 (doubleres).png",
    "NoteSkins/common/common/_arrow 1x8.png",
];
const ARROW_FALLBACK_CANDIDATES: [&str; 4] = [
    "mini_stepmania_rust/NoteSkins/common/common/Fallback Tap Note 4x1.png",
    "NoteSkins/common/common/Fallback Tap Note 4x1.png",
    "mini_stepmania_rust/NoteSkins/common/common/Fallback Tap Lift 4x1.png",
    "NoteSkins/common/common/Fallback Tap Lift 4x1.png",
];
const DIR_LEFT_CANDIDATES: [&str; 3] = [
    "mini_stepmania_rust/assets/arrows/left.png",
    "assets/arrows/left.png",
    "assets/arrow_left.png",
];
const DIR_DOWN_CANDIDATES: [&str; 3] = [
    "mini_stepmania_rust/assets/arrows/down.png",
    "assets/arrows/down.png",
    "assets/arrow_down.png",
];
const DIR_UP_CANDIDATES: [&str; 3] = [
    "mini_stepmania_rust/assets/arrows/up.png",
    "assets/arrows/up.png",
    "assets/arrow_up.png",
];
const DIR_RIGHT_CANDIDATES: [&str; 3] = [
    "mini_stepmania_rust/assets/arrows/right.png",
    "assets/arrows/right.png",
    "assets/arrow_right.png",
];
const HOLD_LEFT_BODY_ACTIVE_CANDIDATES: [&str; 2] = [
    "mini_stepmania_rust/assets/holds/left_body_active.png",
    "assets/holds/left_body_active.png",
];
const HOLD_LANE_NAMES: [&str; LANE_COUNT] = ["Left", "Down", "Up", "Right"];
const HOLD_DOWN_BODY_ACTIVE_CANDIDATES: [&str; 2] = [
    "mini_stepmania_rust/assets/holds/down_body_active.png",
    "assets/holds/down_body_active.png",
];
const HOLD_UP_BODY_ACTIVE_CANDIDATES: [&str; 2] = [
    "mini_stepmania_rust/assets/holds/up_body_active.png",
    "assets/holds/up_body_active.png",
];
const HOLD_RIGHT_BODY_ACTIVE_CANDIDATES: [&str; 2] = [
    "mini_stepmania_rust/assets/holds/right_body_active.png",
    "assets/holds/right_body_active.png",
];
const HOLD_LEFT_TOPCAP_ACTIVE_CANDIDATES: [&str; 2] = [
    "mini_stepmania_rust/assets/holds/left_topcap_active.png",
    "assets/holds/left_topcap_active.png",
];
const HOLD_DOWN_TOPCAP_ACTIVE_CANDIDATES: [&str; 2] = [
    "mini_stepmania_rust/assets/holds/down_topcap_active.png",
    "assets/holds/down_topcap_active.png",
];
const HOLD_UP_TOPCAP_ACTIVE_CANDIDATES: [&str; 2] = [
    "mini_stepmania_rust/assets/holds/up_topcap_active.png",
    "assets/holds/up_topcap_active.png",
];
const HOLD_RIGHT_TOPCAP_ACTIVE_CANDIDATES: [&str; 2] = [
    "mini_stepmania_rust/assets/holds/right_topcap_active.png",
    "assets/holds/right_topcap_active.png",
];
const HOLD_LEFT_BOTTOMCAP_ACTIVE_CANDIDATES: [&str; 2] = [
    "mini_stepmania_rust/assets/holds/left_bottomcap_active.png",
    "assets/holds/left_bottomcap_active.png",
];
const HOLD_DOWN_BOTTOMCAP_ACTIVE_CANDIDATES: [&str; 2] = [
    "mini_stepmania_rust/assets/holds/down_bottomcap_active.png",
    "assets/holds/down_bottomcap_active.png",
];
const HOLD_UP_BOTTOMCAP_ACTIVE_CANDIDATES: [&str; 2] = [
    "mini_stepmania_rust/assets/holds/up_bottomcap_active.png",
    "assets/holds/up_bottomcap_active.png",
];
const HOLD_RIGHT_BOTTOMCAP_ACTIVE_CANDIDATES: [&str; 2] = [
    "mini_stepmania_rust/assets/holds/right_bottomcap_active.png",
    "assets/holds/right_bottomcap_active.png",
];
const TAP_SFX_CANDIDATES: [&str; 3] = [
    "mini_stepmania_rust/assets/sfx/tap01.ogg",
    "assets/sfx/tap01.ogg",
    "tap01.ogg",
];
const HIT_EXPLOSION_CANDIDATES: [&str; 3] = [
    "mini_stepmania_rust/NoteSkins/common/_Editor/_Down Explosion 2x1 (res 128x64).png",
    "NoteSkins/common/_Editor/_Down Explosion 2x1 (res 128x64).png",
    "mini_stepmania_rust/assets/arrows/Fallback HitMine Explosion.png",
];
const UI_FONT_CANDIDATES: [&str; 2] = [
    "mini_stepmania_rust/assets/fonts/JingNanBoBoHei-Bold.ttf",
    "assets/fonts/JingNanBoBoHei-Bold.ttf",
];
const GAME_SPEED_FILE: &str = "last_game_speed.txt";

pub fn window_conf() -> Conf {
    Conf {
        window_title: "Mini StepMania Rewrite".to_string(),
        window_width: WINDOW_W,
        window_height: WINDOW_H,
        high_dpi: true,
        window_resizable: false,
        ..Default::default()
    }
}

pub struct Game {
    chart: Chart,
    states: Vec<bool>,
    song_time_accum: f32,
    audio_start_song_time: f32,
    song_time_anchor: f32,
    song_time_anchor_wall: f64,
    last_wall_time: f64,
    game_speed: f32,
    audio_speed: f32,
    audio_stream: Option<OutputStream>,
    audio_handle: Option<OutputStreamHandle>,
    audio_path: Option<PathBuf>,
    decoded_audio: Option<DecodedAudioClip>,
    decoded_audio_attempted: bool,
    tap_sfx: Option<DecodedAudioClip>,
    tap_sfx_attempted: bool,
    sink: Option<Sink>,
    hit_sfx_sinks: Vec<Sink>,
    hit_events: Vec<HitFxEvent>,
    arrow_tex: Option<Texture2D>,
    dir_arrow_tex: [Option<Texture2D>; LANE_COUNT],
    hold_body_active_tex: [Option<Texture2D>; LANE_COUNT],
    hold_body_inactive_tex: [Option<Texture2D>; LANE_COUNT],
    hold_topcap_active_tex: [Option<Texture2D>; LANE_COUNT],
    hold_bottomcap_active_tex: [Option<Texture2D>; LANE_COUNT],
    hold_bottomcap_inactive_tex: [Option<Texture2D>; LANE_COUNT],
    hit_explosion_tex: Option<Texture2D>,
    hit_add_material: Option<Material>,
    jacket_tex: Option<Texture2D>,
    jacket_blur_tex: Option<Texture2D>,
    jacket_blur_rt: Option<RenderTarget>,
    noteskin_cfg: NoteskinConfig,
    bgm_volume: f32,
    sfx_volume: f32,
    paused: bool,
    pause_menu_page: usize,
    mode: GameMode,
    results_entered_at: f64,
    perfect: i32,
    great: i32,
    good: i32,
    miss: i32,
    combo: i32,
    best_combo: i32,
    stall_paused: bool,
    stable_frames: i32,
    dragging_scrub: bool,
    pause_minimap_dragging: bool,
    pause_minimap_filtered_ratio: f32,
    last_scrub_y: f32,
    anim_start_time: f64,
    ui_font: Option<Font>,
    debug_last_print_song_time: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GameMode {
    Playing,
    Results,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameExitAction {
    BackToSongSelect,
    Restart,
}

#[derive(Clone)]
struct NoteskinConfig {
    note_color_count: usize,
    note_color_denominator: bool,
    lane_rotation_deg: [f32; LANE_COUNT], // Left, Down, Up, Right
}

#[derive(Clone)]
struct DecodedAudioClip {
    channels: u16,
    sample_rate: u32,
    samples: Arc<Vec<f32>>,
}

#[derive(Clone, Copy)]
struct HitFxEvent {
    lane: usize,
    time: f32,
}

impl Default for NoteskinConfig {
    fn default() -> Self {
        Self {
            note_color_count: 8,
            note_color_denominator: true,
            lane_rotation_deg: [90.0, 0.0, 180.0, -90.0],
        }
    }
}

impl Game {
    fn draw_crisp_rect_outline(&self, r: Rect, thickness: f32, color: Color) {
        let t = thickness.max(1.0).round();
        let x = r.x.round();
        let y = r.y.round();
        let w = r.w.round();
        let h = r.h.round();
        // Draw 4 edges explicitly to avoid missing edge artifacts under scaling.
        draw_rectangle(x, y, w, t, color);
        draw_rectangle(x, y + h - t, w, t, color);
        draw_rectangle(x, y, t, h, color);
        draw_rectangle(x + w - t, y, t, h, color);
    }

    pub async fn new(chart: Chart, audio_speed: f32) -> Self {
        let t0 = Instant::now();
        log_file::write(format!("Game::new start title={} notes={}", chart.title, chart.notes.len()));
        let jacket_tex = load_jacket_texture(chart.music.as_deref()).await;
        let (jacket_blur_rt, jacket_blur_tex) = build_jacket_blur_gpu(jacket_tex.as_ref());
        println!("[boot] jacket load: {} ms", t0.elapsed().as_millis());
        log_file::write(format!("[boot] jacket load: {} ms", t0.elapsed().as_millis()));
        let t1 = Instant::now();
        let arrow_tex = load_arrow_texture().await;
        let dir_arrow_tex = load_direction_arrow_textures().await;
        let hold_body_active_tex = load_hold_body_active_textures().await;
        let hold_body_inactive_tex = load_hold_body_inactive_textures().await;
        let hold_topcap_active_tex = [None, None, None, None];
        let hold_bottomcap_active_tex = load_hold_bottomcap_active_textures().await;
        let hold_bottomcap_inactive_tex = load_hold_bottomcap_inactive_textures().await;
        let hit_explosion_tex = load_texture_candidates(&HIT_EXPLOSION_CANDIDATES).await;
        let hit_add_material = load_additive_material();
        println!("[boot] noteskin texture load: {} ms", t1.elapsed().as_millis());
        log_file::write(format!("[boot] noteskin texture load: {} ms", t1.elapsed().as_millis()));
        let t2 = Instant::now();
        let noteskin_cfg = load_noteskin_config();
        println!("[boot] noteskin config load: {} ms", t2.elapsed().as_millis());
        log_file::write(format!("[boot] noteskin config load: {} ms", t2.elapsed().as_millis()));
        let t3 = Instant::now();
        let (audio_stream, audio_handle, sink, audio_path) =
            (None, None, None, chart.music.clone());
        let decoded_audio = None;
        let tap_sfx = None;
        let ui_font = load_ui_font().await;
        println!("[boot] tap sfx decode: {} ms", t3.elapsed().as_millis());
        println!("[boot] Game::new total: {} ms", t0.elapsed().as_millis());
        log_file::write(format!("[boot] tap sfx decode: {} ms", t3.elapsed().as_millis()));
        log_file::write(format!("[boot] Game::new total: {} ms", t0.elapsed().as_millis()));
        Self {
            states: vec![false; chart.notes.len()],
            chart,
            song_time_accum: 0.0,
            audio_start_song_time: 0.0,
            song_time_anchor: 0.0,
            song_time_anchor_wall: get_time(),
            last_wall_time: get_time(),
            game_speed: load_saved_game_speed(),
            audio_speed,
            audio_stream,
            audio_handle,
            audio_path,
            decoded_audio,
            decoded_audio_attempted: false,
            tap_sfx,
            tap_sfx_attempted: false,
            sink,
            hit_sfx_sinks: Vec::new(),
            hit_events: Vec::new(),
            arrow_tex,
            dir_arrow_tex,
            hold_body_active_tex,
            hold_body_inactive_tex,
            hold_topcap_active_tex,
            hold_bottomcap_active_tex,
            hold_bottomcap_inactive_tex,
            hit_explosion_tex,
            hit_add_material,
            jacket_tex,
            jacket_blur_tex,
            jacket_blur_rt,
            noteskin_cfg,
            bgm_volume: 1.0,
            sfx_volume: 1.0,
            paused: false,
            pause_menu_page: 0,
            mode: GameMode::Playing,
            results_entered_at: 0.0,
            perfect: 0,
            great: 0,
            good: 0,
            miss: 0,
            combo: 0,
            best_combo: 0,
            stall_paused: false,
            stable_frames: 0,
            dragging_scrub: false,
            pause_minimap_dragging: false,
            pause_minimap_filtered_ratio: 0.0,
            last_scrub_y: 0.0,
            anim_start_time: get_time(),
            ui_font,
            debug_last_print_song_time: -999.0,
        }
    }

    pub async fn run(&mut self) -> GameExitAction {
        self.draw(0.0);
        next_frame().await;
        self.ensure_audio_started();
        self.resync_audio_to_song_time(0.0, false, false);
        self.reset_song_time_anchor();
        self.debug_dump_healing_window_notes();
        self.last_wall_time = get_time();
        loop {
            if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Backspace) {
                return GameExitAction::BackToSongSelect;
            }

            if self.mode == GameMode::Results {
                if is_key_pressed(KeyCode::Enter)
                    || is_key_pressed(KeyCode::KpEnter)
                    || is_key_pressed(KeyCode::Space)
                    || is_key_pressed(KeyCode::Backspace)
                {
                    return GameExitAction::BackToSongSelect;
                }
                self.draw(self.song_time_accum);
                next_frame().await;
                continue;
            }

            if is_key_pressed(KeyCode::Space) {
                self.toggle_pause();
            }
            self.handle_pause_button_click();

            let now = get_time();
            let raw_dt = ((now - self.last_wall_time) as f32 * self.audio_speed).max(0.0);
            self.last_wall_time = now;
            // Auto-pause during window dragging/move stalls to avoid timeline jitter.
            if raw_dt > 0.05 {
                if !self.stall_paused {
                    self.stall_paused = true;
                    self.stable_frames = 0;
                    if let Some(s) = &self.sink {
                        s.pause();
                    }
                }
            } else if self.stall_paused {
                if raw_dt < 0.02 {
                    self.stable_frames += 1;
                } else {
                    self.stable_frames = 0;
                }
                if self.stable_frames >= 3 {
                    self.stall_paused = false;
                    if let Some(s) = &self.sink {
                        s.play();
                    }
                    self.reset_song_time_anchor();
                }
            }

            if !self.stall_paused {
                if self.paused {
                    if is_key_pressed(KeyCode::Left) {
                        self.adjust_speed(-SPEED_STEP);
                    }
                    if is_key_pressed(KeyCode::Right) {
                        self.adjust_speed(SPEED_STEP);
                    }
                    let dragging_minimap = self.handle_pause_minimap_drag();
                    if !dragging_minimap {
                        self.handle_pointer_scrub();
                    }
                    if let Some(action) = self.handle_speed_control_click() {
                        return action;
                    }
                } else {
                    self.song_time_accum = self.current_song_time(now);
                    self.update_misses(self.song_time_accum);
                }
            }
            let song_time = self.song_time_accum;
            self.debug_trace_healing_window(song_time);
            if !self.paused {
                self.handle_manual_input(song_time);
                self.autoplay(song_time);
            }
            self.hit_sfx_sinks.retain(|s| !s.empty());
            self.hit_events
                .retain(|e| song_time >= e.time && song_time - e.time <= HIT_FX_DURATION_SEC);
            self.draw(song_time);

            if self.is_finished(song_time) {
                self.mode = GameMode::Results;
                self.results_entered_at = get_time();
                self.paused = false;
                if let Some(s) = &self.sink {
                    s.pause();
                }
            }

            next_frame().await;
        }
    }

    fn autoplay(&mut self, song_time: f32) {
        for i in 0..self.chart.notes.len() {
            if self.states[i] {
                continue;
            }
            if self.chart.notes[i].time <= song_time {
                self.mark_hit_and_sfx(i);
            }
        }
    }

    fn handle_manual_input(&mut self, song_time: f32) {
        let keys = [
            (KeyCode::Left, 0usize),
            (KeyCode::Down, 1usize),
            (KeyCode::Up, 2usize),
            (KeyCode::Right, 3usize),
        ];
        for (key, lane) in keys {
            if !is_key_pressed(key) {
                continue;
            }
            if let Some(idx) = self.find_manual_hit_candidate(lane, song_time) {
                self.mark_hit_and_sfx(idx);
            }
        }
        // Touch input: split the screen into 4 equal vertical zones => L/D/U/R lanes.
        let sw = screen_width().max(1.0);
        let zone_w = sw / 4.0;
        for t in touches() {
            if t.phase != TouchPhase::Started {
                continue;
            }
            let mut lane = (t.position.x / zone_w).floor() as usize;
            if lane > 3 {
                lane = 3;
            }
            if let Some(idx) = self.find_manual_hit_candidate(lane, song_time) {
                self.mark_hit_and_sfx(idx);
            }
        }
    }

    fn find_manual_hit_candidate(&self, lane: usize, song_time: f32) -> Option<usize> {
        const HIT_WINDOW: f32 = 0.12;
        let mut best: Option<(usize, f32)> = None;
        for (i, n) in self.chart.notes.iter().enumerate() {
            if self.states[i] || n.lane != lane {
                continue;
            }
            let dt = (n.time - song_time).abs();
            if dt > HIT_WINDOW {
                continue;
            }
            match best {
                Some((_, cur)) if dt >= cur => {}
                _ => best = Some((i, dt)),
            }
        }
        best.map(|(i, _)| i)
    }

    fn mark_hit_and_sfx(&mut self, note_idx: usize) {
        if self.states.get(note_idx).copied().unwrap_or(true) {
            return;
        }
        let lane = self.chart.notes.get(note_idx).map(|n| n.lane).unwrap_or(0);
        self.states[note_idx] = true;
        let dt = self
            .chart
            .notes
            .get(note_idx)
            .map(|n| (n.time - self.song_time_accum).abs())
            .unwrap_or(1.0);
        if dt <= 0.045 {
            self.perfect += 1;
        } else if dt <= 0.090 {
            self.great += 1;
        } else {
            self.good += 1;
        }
        self.combo += 1;
        if self.combo > self.best_combo {
            self.best_combo = self.combo;
        }
        self.hit_events.push(HitFxEvent {
            lane,
            time: self.song_time_accum,
        });
        self.play_tap_sfx();
    }

    fn update_misses(&mut self, song_time: f32) {
        const MISS_WINDOW: f32 = 0.12;
        for (i, n) in self.chart.notes.iter().enumerate() {
            if self.states[i] {
                continue;
            }
            if song_time > n.time + MISS_WINDOW {
                self.states[i] = true;
                self.miss += 1;
                self.combo = 0;
            }
        }
    }

    fn is_finished(&self, song_time: f32) -> bool {
        let end = self.chart.notes.last().map(|n| n.time + 2.0).unwrap_or(3.0);
        song_time >= end
    }

    fn sync_note_states_to_time(&mut self, song_time: f32) {
        for (i, n) in self.chart.notes.iter().enumerate() {
            self.states[i] = n.time <= song_time + 0.01;
        }
    }

    fn handle_pointer_scrub(&mut self) {
        let mut pointer_active = false;
        let mut pointer_y = 0.0_f32;
        if is_mouse_button_down(MouseButton::Left) {
            let (_, y) = mouse_position();
            pointer_active = true;
            pointer_y = y;
        } else {
            let ts = touches();
            if let Some(t) = ts.first() {
                pointer_active = true;
                pointer_y = t.position.y;
            }
        }

        if pointer_active {
            if !self.dragging_scrub {
                self.dragging_scrub = true;
                self.last_scrub_y = pointer_y;
            } else {
                let dy = pointer_y - self.last_scrub_y;
                self.last_scrub_y = pointer_y;
                let delta = -(dy * 0.006);
                if delta.abs() > 0.0001 {
                    self.song_time_accum = (self.song_time_accum + delta).max(0.0);
                    self.sync_note_states_to_time(self.song_time_accum);
                    self.resync_audio_to_song_time(self.song_time_accum, true, true);
                    self.reset_song_time_anchor();
                }
            }
        } else {
            self.dragging_scrub = false;
        }
    }

    fn toggle_pause(&mut self) {
        self.paused = !self.paused;
        if self.paused {
            self.pause_menu_page = 0;
            if let Some(s) = &self.sink {
                s.pause();
            }
            self.reset_song_time_anchor();
        } else {
            self.resync_audio_to_song_time(self.song_time_accum, false, true);
            self.reset_song_time_anchor();
        }
    }

    fn reset_song_time_anchor(&mut self) {
        self.song_time_anchor = self.song_time_accum.max(0.0);
        self.song_time_anchor_wall = get_time();
    }

    fn current_song_time(&mut self, now: f64) -> f32 {
        let elapsed = ((now - self.song_time_anchor_wall) as f32 * self.audio_speed).max(0.0);
        (self.song_time_anchor + elapsed).max(0.0)
    }

    fn pause_button_rect(&self) -> Rect {
        Rect::new(LOGICAL_W - 250.0, 84.0, 190.0, 62.0)
    }

    fn handle_pause_button_click(&mut self) {
        if !is_mouse_button_pressed(MouseButton::Left) {
            return;
        }
        let (sx, sy) = mouse_position();
        let lx = sx * (LOGICAL_W / screen_width().max(1.0));
        let ly = sy * (LOGICAL_H / screen_height().max(1.0));
        let r = self.pause_button_rect();
        if lx >= r.x && lx <= r.x + r.w && ly >= r.y && ly <= r.y + r.h {
            self.toggle_pause();
        }
    }

    fn resync_audio_to_song_time(&mut self, song_time: f32, paused: bool, prefer_precise: bool) {
        self.ensure_audio_started();
        if prefer_precise {
            self.ensure_decoded_audio_loaded();
        }
        let Some(handle) = &self.audio_handle else { return; };
        // Keep seek/speed changes responsive: avoid heavy on-demand full-song decode here.
        let new_sink = if let Some(clip) = &self.decoded_audio {
            create_sink_from_decoded(handle, clip, self.audio_speed, song_time)
        } else if let Some(path) = &self.audio_path {
            create_sink_for_path(handle, path, self.audio_speed, song_time)
        } else {
            None
        };
        let Some(new_sink) = new_sink else {
            return;
        };
        self.audio_start_song_time = song_time.max(0.0);
        self.song_time_accum = self.audio_start_song_time;
        self.song_time_anchor = self.audio_start_song_time;
        self.song_time_anchor_wall = get_time();
        new_sink.set_volume(self.bgm_volume);
        if paused {
            new_sink.pause();
        } else {
            new_sink.play();
        }
        if let Some(old) = self.sink.replace(new_sink) {
            old.stop();
        }
    }

    fn ensure_decoded_audio_loaded(&mut self) {
        if self.decoded_audio_attempted {
            return;
        }
        self.decoded_audio_attempted = true;
        self.decoded_audio = load_decoded_audio_clip(self.audio_path.as_deref());
    }

    fn adjust_speed(&mut self, delta: f32) {
        let next = (self.audio_speed + delta).clamp(SPEED_MIN, SPEED_MAX);
        if (next - self.audio_speed).abs() < 0.0001 {
            return;
        }
        self.audio_speed = (next * 10.0).round() / 10.0;
        self.resync_audio_to_song_time(self.song_time_accum, true, true);
        self.reset_song_time_anchor();
    }

    fn adjust_game_speed(&mut self, delta: f32) {
        let next = (self.game_speed + delta).clamp(GAME_SPEED_MIN, GAME_SPEED_MAX);
        self.game_speed = (next * 10.0).round() / 10.0;
        save_game_speed(self.game_speed);
    }

    fn ensure_audio_started(&mut self) {
        if self.audio_handle.is_some() {
            return;
        }
        let t = Instant::now();
        log_file::write("audio init start");
        let Ok((stream, handle)) = OutputStream::try_default() else {
            println!("Audio: no default output device");
            log_file::write("audio init failed: no default output device");
            return;
        };
        self.audio_stream = Some(stream);
        self.audio_handle = Some(handle);
        self.audio_path = self.chart.music.clone();
        println!("[boot] audio device init: {} ms", t.elapsed().as_millis());
        log_file::write(format!("[boot] audio device init: {} ms", t.elapsed().as_millis()));
    }

    fn adjust_bgm_volume(&mut self, delta: f32) {
        let next = (self.bgm_volume + delta).clamp(BGM_VOL_MIN, BGM_VOL_MAX);
        self.bgm_volume = (next * 10.0).round() / 10.0;
        if let Some(s) = &self.sink {
            s.set_volume(self.bgm_volume);
        }
    }

    fn adjust_sfx_volume(&mut self, delta: f32) {
        let next = (self.sfx_volume + delta).clamp(SFX_VOL_MIN, SFX_VOL_MAX);
        self.sfx_volume = (next * 10.0).round() / 10.0;
    }

    fn play_tap_sfx(&mut self) {
        self.ensure_tap_sfx_loaded();
        let Some(handle) = &self.audio_handle else { return; };
        let Some(clip) = &self.tap_sfx else { return; };
        let Some(sink) = create_sink_from_decoded(handle, clip, 1.0, 0.0) else {
            return;
        };
        sink.set_volume(TAP_SFX_BASE_VOLUME * self.sfx_volume);
        sink.play();
        self.hit_sfx_sinks.push(sink);
    }

    fn ensure_tap_sfx_loaded(&mut self) {
        if self.tap_sfx_attempted {
            return;
        }
        self.tap_sfx_attempted = true;
        self.tap_sfx = load_decoded_audio_clip_from_candidates(&TAP_SFX_CANDIDATES);
    }

    fn speed_control_layout(&self) -> Rect {
        let panel_w = LOGICAL_W * 0.62;
        let panel_h = 560.0;
        let panel_x = (LOGICAL_W - panel_w) * 0.5;
        let panel_y = LOGICAL_H * 0.42;
        Rect::new(panel_x, panel_y, panel_w, panel_h)
    }

    fn pause_page_dot_rects(&self) -> [Rect; 2] {
        let panel = self.speed_control_layout();
        let r = 11.0;
        let gap = 34.0;
        let cy = panel.y + 42.0;
        [
            Rect::new(panel.x + panel.w * 0.5 - gap * 0.5 - r, cy - r, r * 2.0, r * 2.0),
            Rect::new(panel.x + panel.w * 0.5 + gap * 0.5 - r, cy - r, r * 2.0, r * 2.0),
        ]
    }

    fn pause_minimap_rect(&self) -> Rect {
        let w = 92.0;
        let x = LOGICAL_W - w - 18.0;
        let y = 170.0;
        let h = LOGICAL_H - 260.0;
        Rect::new(x, y, w, h.max(240.0))
    }

    fn pause_timeline_max_beat(&self) -> f32 {
        let mut max_beat = 1.0_f32;
        for n in &self.chart.notes {
            max_beat = max_beat.max(n.beat);
            if let NoteKind::Hold { end_beat, .. } = n.kind {
                max_beat = max_beat.max(end_beat);
            }
        }
        max_beat
    }

    fn handle_pause_minimap_drag(&mut self) -> bool {
        if !self.paused {
            self.pause_minimap_dragging = false;
            return false;
        }
        let r = self.pause_minimap_rect();
        let (sx, sy) = mouse_position();
        let lx = sx * (LOGICAL_W / screen_width().max(1.0));
        let ly = sy * (LOGICAL_H / screen_height().max(1.0));
        let contains = lx >= r.x && lx <= r.x + r.w && ly >= r.y && ly <= r.y + r.h;

        if is_mouse_button_pressed(MouseButton::Left) && contains {
            self.pause_minimap_dragging = true;
            let init_ratio = ((ly - r.y) / r.h).clamp(0.0, 1.0);
            self.pause_minimap_filtered_ratio = init_ratio;
        }
        if self.pause_minimap_dragging && !is_mouse_button_down(MouseButton::Left) {
            // Release snap: lock to nearest 1/4 beat for stable pause positioning.
            let max_beat = self.pause_timeline_max_beat().max(1.0);
            let cur_beat = self.pause_minimap_filtered_ratio * max_beat;
            let snapped_beat = (cur_beat * 4.0).round() / 4.0;
            let target_time = (beat_to_seconds(snapped_beat, &self.chart.bpms, &self.chart.stops)
                - self.chart.offset)
                .max(0.0);
            self.song_time_accum = target_time;
            self.sync_note_states_to_time(self.song_time_accum);
            self.resync_audio_to_song_time(self.song_time_accum, true, true);
            self.reset_song_time_anchor();
            self.pause_minimap_dragging = false;
            return false;
        }
        if !self.pause_minimap_dragging {
            return false;
        }

        let ratio = ((ly - r.y) / r.h).clamp(0.0, 1.0);
        // Small inertia-like smoothing while dragging.
        self.pause_minimap_filtered_ratio += (ratio - self.pause_minimap_filtered_ratio) * 0.35;
        let target_beat = self.pause_minimap_filtered_ratio * self.pause_timeline_max_beat();
        let target_time = (beat_to_seconds(target_beat, &self.chart.bpms, &self.chart.stops)
            - self.chart.offset)
            .max(0.0);
        self.song_time_accum = target_time;
        self.sync_note_states_to_time(self.song_time_accum);
        self.resync_audio_to_song_time(self.song_time_accum, true, true);
        self.reset_song_time_anchor();
        true
    }

    fn pause_value_row_rects(&self) -> ([Rect; 2], [Rect; 2], [Rect; 2]) {
        let panel = self.speed_control_layout();
        let page_labels = if self.pause_menu_page == 0 {
            ["游戏速度", "播放速度"]
        } else {
            ["背景音量", "音效音量"]
        };
        let mut max_label_w = 0.0_f32;
        for t in page_labels {
            let m = self.measure_text_ui(t, 34, 1.0);
            max_label_w = max_label_w.max(m.width);
        }
        let button_w = 84.0;
        let button_h = 92.0;
        let row_h = 132.0;
        let row_top = panel.y + 90.0;
        let value_w = panel.w * 0.36;
        let value_h = 78.0;
        let side_gap = 18.0;
        let label_w = (max_label_w + 18.0).ceil();
        let label_gap = 8.0;
        let group_w = label_w + label_gap + button_w + side_gap + value_w + side_gap + button_w;
        let content_left = panel.x + 20.0;
        let content_right = panel.x + panel.w - 20.0;
        let content_w = (content_right - content_left).max(group_w + 2.0);
        let group_x = content_left + (content_w - group_w) * 0.5;
        let label_x = group_x;
        let left_x = label_x + label_w + label_gap;
        let value_x = left_x + button_w + side_gap;
        let right_x = value_x + value_w + side_gap;
        let mut labels = [Rect::new(0.0, 0.0, 0.0, 0.0); 2];
        let mut lefts = [Rect::new(0.0, 0.0, 0.0, 0.0); 2];
        let mut rights = [Rect::new(0.0, 0.0, 0.0, 0.0); 2];
        for i in 0..2 {
            let y = row_top + i as f32 * row_h + (row_h - button_h) * 0.5;
            let vy = row_top + i as f32 * row_h + (row_h - value_h) * 0.5;
            labels[i] = Rect::new(label_x, vy, label_w, value_h);
            lefts[i] = Rect::new(left_x, y, button_w, button_h);
            rights[i] = Rect::new(right_x, y, button_w, button_h);
        }
        (labels, lefts, rights)
    }

    fn pause_menu_action_rects(&self) -> (Rect, Rect) {
        let panel = self.speed_control_layout();
        let bw = panel.w * 0.38;
        let bh = 82.0;
        let y = panel.y + panel.h - bh - 56.0;
        let gap = panel.w * 0.06;
        let x1 = panel.x + (panel.w - bw * 2.0 - gap) * 0.5;
        let x2 = x1 + bw + gap;
        (Rect::new(x1, y, bw, bh), Rect::new(x2, y, bw, bh))
    }

    fn handle_speed_control_click(&mut self) -> Option<GameExitAction> {
        if !is_mouse_button_pressed(MouseButton::Left) {
            return None;
        }
        let (sx, sy) = mouse_position();
        let lx = sx * (LOGICAL_W / screen_width().max(1.0));
        let ly = sy * (LOGICAL_H / screen_height().max(1.0));
        let dots = self.pause_page_dot_rects();
        let (_, lefts, rights) = self.pause_value_row_rects();
        let (back_btn, restart_btn) = self.pause_menu_action_rects();
        let contains = |r: Rect| lx >= r.x && lx <= r.x + r.w && ly >= r.y && ly <= r.y + r.h;
        if contains(dots[0]) {
            self.pause_menu_page = 0;
            return None;
        }
        if contains(dots[1]) {
            self.pause_menu_page = 1;
            return None;
        }
        for i in 0..2 {
            let item = self.pause_menu_page * 2 + i;
            if contains(lefts[i]) {
                match item {
                    0 => self.adjust_game_speed(-GAME_SPEED_STEP),
                    1 => self.adjust_speed(-SPEED_STEP),
                    2 => self.adjust_bgm_volume(-BGM_VOL_STEP),
                    3 => self.adjust_sfx_volume(-SFX_VOL_STEP),
                    _ => {}
                }
                return None;
            }
            if contains(rights[i]) {
                match item {
                    0 => self.adjust_game_speed(GAME_SPEED_STEP),
                    1 => self.adjust_speed(SPEED_STEP),
                    2 => self.adjust_bgm_volume(BGM_VOL_STEP),
                    3 => self.adjust_sfx_volume(SFX_VOL_STEP),
                    _ => {}
                }
                return None;
            }
        }
        if self.pause_menu_page == 0 {
            if contains(back_btn) {
                return Some(GameExitAction::BackToSongSelect);
            }
            if contains(restart_btn) {
                return Some(GameExitAction::Restart);
            }
        }
        None
    }

    fn draw_pause_speed_control(&self, _song_time: f32) {
        let panel = self.speed_control_layout();
        // Shadow
        draw_rectangle(
            panel.x + 10.0,
            panel.y + 14.0,
            panel.w,
            panel.h,
            Color::from_rgba(0, 0, 0, 95),
        );
        draw_rectangle(
            panel.x + 18.0,
            panel.y + 22.0,
            panel.w,
            panel.h,
            Color::from_rgba(0, 0, 0, 58),
        );
        // 实时模糊：抓取当前帧底图，对弹窗区域做多采样叠加。
        let snap = get_screen_data();
        let snap_tex = Texture2D::from_image(&snap);
        let sample_pad_x = 20.0;
        let sample_pad_top = 54.0;
        let sample_pad_bottom = 16.0;
        let sx = (panel.x - sample_pad_x).max(0.0);
        let sy = (panel.y - sample_pad_top).max(0.0);
        let sw = (panel.w + sample_pad_x * 2.0).min(LOGICAL_W - sx);
        let sh = (panel.h + sample_pad_top + sample_pad_bottom).min(LOGICAL_H - sy);
        let src_x = sx * (snap_tex.width() / LOGICAL_W);
        let src_y_top = sy * (snap_tex.height() / LOGICAL_H);
        let src_w = sw * (snap_tex.width() / LOGICAL_W);
        let src_h = sh * (snap_tex.height() / LOGICAL_H);
        // get_screen_data 的 y 方向与 draw_texture source 方向相反，这里反转修正。
        let src = Rect::new(src_x, snap_tex.height() - src_y_top, src_w, -src_h);
        let taps: &[(f32, f32, f32)] = &[
            (0.0, 0.0, 0.32),
            (-6.0, 0.0, 0.20),
            (6.0, 0.0, 0.20),
            (0.0, -6.0, 0.20),
            (0.0, 6.0, 0.20),
            (-4.0, -4.0, 0.16),
            (4.0, -4.0, 0.16),
            (-4.0, 4.0, 0.16),
            (4.0, 4.0, 0.16),
        ];
        for (dx, dy, a) in taps {
            draw_texture_ex(
                &snap_tex,
                sx + dx,
                sy + dy,
                Color::new(1.0, 1.0, 1.0, *a),
                DrawTextureParams {
                    source: Some(src),
                    dest_size: Some(vec2(sw, sh)),
                    ..Default::default()
                },
            );
        }
        draw_rectangle(panel.x, panel.y, panel.w, panel.h, Color::from_rgba(20, 24, 36, 36));
        self.draw_crisp_rect_outline(panel, 4.0, Color::from_rgba(245, 90, 90, 255));
        let labels = ["游戏速度", "播放速度", "背景音量", "音效音量"];
        let values = [
            format!("{:.1}x", self.game_speed),
            format!("{:.1}x", self.audio_speed),
            format!("{:.1}", self.bgm_volume),
            format!("{:.1}", self.sfx_volume),
        ];
        let dots = self.pause_page_dot_rects();
        for (i, d) in dots.iter().enumerate() {
            let active = i == self.pause_menu_page;
            draw_circle(
                d.x + d.w * 0.5,
                d.y + d.h * 0.5,
                d.w * 0.5,
                if active {
                    Color::from_rgba(245, 90, 90, 255)
                } else {
                    Color::from_rgba(90, 110, 145, 180)
                },
            );
        }

        let (label_rects, lefts, rights) = self.pause_value_row_rects();
        for i in 0..2 {
            let item = self.pause_menu_page * 2 + i;
            let lr = label_rects[i];
            let value_w = panel.w * 0.36;
            let value_h = 78.0;
            let value_x = lefts[i].x + lefts[i].w + 18.0;
            let value_y = lr.y;
            draw_rectangle(value_x, value_y, value_w, value_h, Color::from_rgba(28, 34, 52, 92));
            self.draw_crisp_rect_outline(
                Rect::new(value_x, value_y, value_w, value_h),
                2.0,
                Color::from_rgba(245, 90, 90, 255),
            );

            let title = labels[item];
            let tm = self.measure_text_ui(title, 34, 1.0);
            self.draw_text_ui(title, lr.x + lr.w - tm.width, lr.y + 54.0, 34.0, Color::from_rgba(245, 90, 90, 255));
            let v = &values[item];
            let m = self.measure_text_ui(v, 48, 1.0);
            self.draw_text_ui(v, value_x + (value_w - m.width) * 0.5, value_y + 54.0, 48.0, Color::from_rgba(245, 90, 90, 255));

            let left = lefts[i];
            let right = rights[i];
            draw_triangle(
                vec2(left.x + left.w * 0.68, left.y + left.h * 0.22),
                vec2(left.x + left.w * 0.68, left.y + left.h * 0.78),
                vec2(left.x + left.w * 0.28, left.y + left.h * 0.50),
                Color::from_rgba(245, 90, 90, 255),
            );
            draw_triangle(
                vec2(right.x + right.w * 0.32, right.y + right.h * 0.22),
                vec2(right.x + right.w * 0.32, right.y + right.h * 0.78),
                vec2(right.x + right.w * 0.72, right.y + right.h * 0.50),
                Color::from_rgba(245, 90, 90, 255),
            );
        }

        if self.pause_menu_page == 0 {
            let (back_btn, restart_btn) = self.pause_menu_action_rects();
            for (r, label) in [(back_btn, "返回选择歌曲"), (restart_btn, "重新开始")] {
                draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(28, 34, 52, 235));
                self.draw_crisp_rect_outline(r, 2.0, Color::from_rgba(245, 90, 90, 255));
                let m = self.measure_text_ui(label, 34, 1.0);
                self.draw_text_ui(
                    label,
                    r.x + (r.w - m.width) * 0.5,
                    r.y + 52.0,
                    34.0,
                    Color::from_rgba(245, 90, 90, 255),
                );
            }
        }

    }

    fn draw_pause_minimap(&self, song_time: f32) {
        let mini = self.pause_minimap_rect();
        draw_rectangle(mini.x, mini.y, mini.w, mini.h, Color::from_rgba(18, 22, 34, 186));
        self.draw_crisp_rect_outline(mini, 2.0, Color::from_rgba(245, 90, 90, 220));

        let max_beat = self.pause_timeline_max_beat().max(1.0);
        let lane_w = (mini.w - 12.0) / LANE_COUNT as f32;
        for n in &self.chart.notes {
            let ratio = (n.beat / max_beat).clamp(0.0, 1.0);
            let y = mini.y + ratio * mini.h;
            let x = mini.x + 6.0 + n.lane as f32 * lane_w + lane_w * 0.5;
            let c = match n.lane {
                0 => Color::from_rgba(60, 120, 255, 225),
                1 => Color::from_rgba(255, 80, 90, 225),
                2 => Color::from_rgba(255, 210, 70, 225),
                _ => Color::from_rgba(80, 220, 120, 225),
            };
            draw_rectangle(x - 1.5, y - 1.0, 3.0, 2.0, c);
        }

        let current_beat = seconds_to_beat(song_time + self.chart.offset, &self.chart.bpms, &self.chart.stops);
        let receptor_y = LOGICAL_H * RECEPTOR_Y_RATIO;
        let px_per_beat = SCROLL_PX_PER_BEAT * self.game_speed;
        let beat_top = current_beat + (0.0 - receptor_y) / px_per_beat;
        let beat_bottom = current_beat + (LOGICAL_H - receptor_y) / px_per_beat;
        let b0 = beat_top.min(beat_bottom).clamp(0.0, max_beat);
        let b1 = beat_top.max(beat_bottom).clamp(0.0, max_beat);
        let y0 = mini.y + (b0 / max_beat) * mini.h;
        let y1 = mini.y + (b1 / max_beat) * mini.h;
        let vh = (y1 - y0).max(8.0);
        draw_rectangle(mini.x + 1.0, y0, mini.w - 2.0, vh, Color::from_rgba(255, 255, 255, 32));
        self.draw_crisp_rect_outline(
            Rect::new(mini.x + 1.0, y0, mini.w - 2.0, vh),
            2.0,
            Color::from_rgba(255, 255, 255, 220),
        );
    }

    fn draw(&self, song_time: f32) {
        let cam = Camera2D {
            zoom: vec2(2.0 / LOGICAL_W, 2.0 / LOGICAL_H),
            target: vec2(LOGICAL_W * 0.5, LOGICAL_H * 0.5),
            ..Default::default()
        };
        set_camera(&cam);
        clear_background(Color::from_rgba(14, 16, 26, 255));
        if self.mode == GameMode::Results {
            self.draw_results();
        } else {
            self.draw_playfield(song_time);
            if self.paused {
                self.draw_pause_speed_control(song_time);
                self.draw_pause_minimap(song_time);
            }
        }
        let center_x = LOGICAL_W * 0.5;
        let playfield_w = (LOGICAL_W * PLAYFIELD_WIDTH_RATIO * NOTEFIELD_SCALE).min(LOGICAL_W * 0.96);
        let lane_spacing = playfield_w / LANE_COUNT as f32;
        let start_x = center_x - playfield_w * 0.5 + lane_spacing * 0.5;
        let receptor_y = LOGICAL_H * RECEPTOR_Y_RATIO;
        self.draw_hit_effects(song_time, start_x, lane_spacing, receptor_y);
        set_default_camera();
    }

    fn draw_results(&self) {
        draw_rectangle(0.0, 0.0, LOGICAL_W, LOGICAL_H, Color::from_rgba(8, 10, 18, 255));
        self.draw_jacket_layers();
        draw_rectangle(120.0, 380.0, LOGICAL_W - 240.0, 1420.0, Color::from_rgba(20, 24, 36, 225));
        draw_rectangle_lines(120.0, 380.0, LOGICAL_W - 240.0, 1420.0, 4.0, Color::from_rgba(245, 90, 90, 255));
        self.draw_text_ui("RESULT", 180.0, 510.0, 96.0, Color::from_rgba(245, 90, 90, 255));
        self.draw_text_ui(&self.chart.title, 180.0, 580.0, 44.0, WHITE);
        self.draw_text_ui(
            &format!("{} [{} {}]", self.chart.artist, self.chart.difficulty, self.chart.meter),
            180.0,
            635.0,
            36.0,
            Color::from_rgba(170, 205, 255, 255),
        );
        let lines = [
            format!("PERFECT  {}", self.perfect),
            format!("GREAT    {}", self.great),
            format!("GOOD     {}", self.good),
            format!("MISS     {}", self.miss),
            format!("BEST COMBO {}", self.best_combo),
        ];
        for (i, line) in lines.iter().enumerate() {
            self.draw_text_ui(line, 220.0, 790.0 + i as f32 * 110.0, 58.0, WHITE);
        }
        let total = (self.perfect + self.great + self.good + self.miss).max(1) as f32;
        let score = ((self.perfect as f32 * 1.0 + self.great as f32 * 0.8 + self.good as f32 * 0.5)
            / total
            * 100.0)
            .round();
        self.draw_text_ui(
            &format!("SCORE {:.0}%", score),
            220.0,
            1410.0,
            76.0,
            Color::from_rgba(255, 236, 120, 255),
        );
        let pulse = ((get_time() - self.results_entered_at) * 5.0).sin() as f32 * 0.2 + 0.8;
        self.draw_text_ui(
            "ENTER/SPACE: Back To Select",
            220.0,
            1700.0,
            44.0,
            Color::new(1.0, 1.0, 1.0, pulse),
        );
    }

    fn draw_playfield(&self, song_time: f32) {
        let center_x = LOGICAL_W * 0.5;
        let playfield_w = (LOGICAL_W * PLAYFIELD_WIDTH_RATIO * NOTEFIELD_SCALE).min(LOGICAL_W * 0.96);
        let lane_spacing = playfield_w / LANE_COUNT as f32;
        let start_x = center_x - playfield_w * 0.5 + lane_spacing * 0.5;
        let receptor_y = LOGICAL_H * RECEPTOR_Y_RATIO;
        let current_beat = seconds_to_beat(
            song_time + self.chart.offset,
            &self.chart.bpms,
            &self.chart.stops,
        );

        draw_rectangle(0.0, 0.0, LOGICAL_W, LOGICAL_H, Color::from_rgba(10, 12, 20, 255));
        self.draw_jacket_layers();

        for lane in 0..LANE_COUNT {
            let x = start_x + lane as f32 * lane_spacing;
            draw_line(x, receptor_y - 100.0, x, LOGICAL_H, 2.0, Color::from_rgba(70, 86, 110, 170));
            self.draw_arrow(lane, 0.0, x, receptor_y, lane_spacing, WHITE, true, song_time, None);
        }

        for (i, note) in self.chart.notes.iter().enumerate() {
            let x = start_x + note.lane as f32 * lane_spacing;
            match note.kind {
                NoteKind::Tap => {
                    if self.states[i] {
                        continue;
                    }
                    let y =
                        receptor_y + (note.beat - current_beat) * SCROLL_PX_PER_BEAT * self.game_speed;
                    if !(-120.0..=LOGICAL_H + 120.0).contains(&y) {
                        continue;
                    }
                    self.draw_arrow(
                        note.lane,
                        note.beat,
                        x,
                        y,
                        lane_spacing,
                        WHITE,
                        false,
                        song_time,
                        None,
                    );
                }
                NoteKind::Hold { end_beat, end_time } => {
                    if song_time > end_time + 0.02 {
                        continue;
                    }
                    let head_y =
                        receptor_y + (note.beat - current_beat) * SCROLL_PX_PER_BEAT * self.game_speed;
                    let tail_y =
                        receptor_y + (end_beat - current_beat) * SCROLL_PX_PER_BEAT * self.game_speed;
                    if head_y < -140.0 && tail_y < -140.0 {
                        continue;
                    }
                    if head_y > LOGICAL_H + 140.0 && tail_y > LOGICAL_H + 140.0 {
                        continue;
                    }
                    let hold_started = song_time >= note.time;
                    let head_draw_y = if hold_started { receptor_y } else { head_y };
                    let tail_draw_y = tail_y;
                    let body_head_y = if hold_started { receptor_y } else { head_y };
                    let body_scroll_px = if hold_started {
                        (current_beat - note.beat) * SCROLL_PX_PER_BEAT * self.game_speed
                    } else {
                        0.0
                    };
                    self.draw_hold_column(
                        note.lane,
                        x,
                        body_head_y,
                        tail_draw_y,
                        lane_spacing,
                        body_scroll_px,
                        hold_started,
                    );
                    self.draw_arrow(
                        note.lane,
                        note.beat,
                        x,
                        head_draw_y,
                        lane_spacing,
                        WHITE,
                        false,
                        song_time,
                        if hold_started { Some(0.0) } else { None },
                    );
                    if hold_started {
                        self.draw_hold_head_glow(note.lane, x, receptor_y, lane_spacing);
                    }
                }
            }
        }
        self.draw_text_ui(&self.chart.title, 40.0, 82.0, 54.0, WHITE);
        self.draw_text_ui(
            &self.chart.artist,
            40.0,
            132.0,
            44.0,
            Color::from_rgba(165, 205, 255, 255),
        );
        let r = self.pause_button_rect();
        draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(24, 32, 48, 230));
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 3.0, Color::from_rgba(245, 90, 90, 255));
        if self.paused {
            self.draw_text_ui("播放", r.x + 30.0, r.y + 42.0, 36.0, Color::from_rgba(245, 90, 90, 255));
            draw_triangle(
                vec2(r.x + r.w - 42.0, r.y + 18.0),
                vec2(r.x + r.w - 42.0, r.y + r.h - 18.0),
                vec2(r.x + r.w - 18.0, r.y + r.h * 0.5),
                Color::from_rgba(245, 90, 90, 255),
            );
        } else {
            self.draw_text_ui("暂停", r.x + 30.0, r.y + 42.0, 36.0, Color::from_rgba(245, 90, 90, 255));
            let bar_w = 8.0;
            let bar_h = 28.0;
            let bar_y = r.y + (r.h - bar_h) * 0.5;
            draw_rectangle(r.x + r.w - 44.0, bar_y, bar_w, bar_h, Color::from_rgba(245, 90, 90, 255));
            draw_rectangle(r.x + r.w - 28.0, bar_y, bar_w, bar_h, Color::from_rgba(245, 90, 90, 255));
        }
    }

    fn debug_dump_healing_window_notes(&self) {
        let title = self.chart.title.to_lowercase();
        let diff = self.chart.difficulty.to_lowercase();
        if !(title.contains("healing vision") && diff.contains("challenge")) {
            return;
        }
        println!(
            "[debug] chart={} diff={} meter={} offset={:.3}",
            self.chart.title, self.chart.difficulty, self.chart.meter, self.chart.offset
        );
        for (i, n) in self.chart.notes.iter().enumerate() {
            if (229.5..=231.5).contains(&n.beat) {
                println!(
                    "[debug] note idx={} lane={} beat={:.6} time={:.6}",
                    i, n.lane, n.beat, n.time
                );
            }
        }
    }

    fn debug_trace_healing_window(&mut self, song_time: f32) {
        let title = self.chart.title.to_lowercase();
        let diff = self.chart.difficulty.to_lowercase();
        if !(title.contains("healing vision") && diff.contains("challenge")) {
            return;
        }
        let current_beat = seconds_to_beat(
            song_time + self.chart.offset,
            &self.chart.bpms,
            &self.chart.stops,
        );
        if !(229.5..=231.5).contains(&current_beat) {
            return;
        }
        if song_time - self.debug_last_print_song_time < 0.03 {
            return;
        }
        self.debug_last_print_song_time = song_time;

        let mut nearest_idx = None;
        let mut nearest_abs_dt = f32::INFINITY;
        let mut nearest_dt = 0.0_f32;
        for (i, n) in self.chart.notes.iter().enumerate() {
            if self.states[i] {
                continue;
            }
            let dt = n.time - song_time;
            let adt = dt.abs();
            if adt < nearest_abs_dt {
                nearest_abs_dt = adt;
                nearest_dt = dt;
                nearest_idx = Some(i);
            }
        }
        if let Some(i) = nearest_idx {
            let n = &self.chart.notes[i];
            let audio_pos = self
                .sink
                .as_ref()
                .map(|s| s.get_pos().as_secs_f32())
                .unwrap_or(0.0);
            let st_minus_audio = song_time - audio_pos;
            println!(
                "[debug] t={:.6} beat={:.6} audio_pos={:.6} dt_sa={:+.6} nearest idx={} lane={} n.beat={:.6} n.time={:.6} dt={:+.6}",
                song_time,
                current_beat,
                audio_pos,
                st_minus_audio,
                i,
                n.lane,
                n.beat,
                n.time,
                nearest_dt
            );
        } else {
            println!("[debug] t={:.6} beat={:.6} nearest none", song_time, current_beat);
        }
    }

    fn draw_hit_effects(&self, song_time: f32, start_x: f32, lane_spacing: f32, receptor_y: f32) {
        let Some(tex) = &self.hit_explosion_tex else { return; };
        if let Some(m) = &self.hit_add_material {
            gl_use_material(m);
        }
        let frame_w = (tex.width() / 2.0).max(1.0);
        let frame_h = tex.height().max(1.0);
        let size = (lane_spacing * ARROW_SIZE_RATIO * 1.10).max(1.0);
        for ev in &self.hit_events {
            let age = song_time - ev.time;
            if !(0.0..=HIT_FX_DURATION_SEC).contains(&age) {
                continue;
            }
            let x = start_x + ev.lane as f32 * lane_spacing;
            let rot = self.noteskin_cfg.lane_rotation_deg[ev.lane].to_radians();
            let src0 = Rect::new(0.0, 0.0, frame_w, frame_h);
            let src1 = Rect::new(frame_w, 0.0, frame_w, frame_h);

            if age < HIT_FX_FRAME_SEC {
                // Frame 1: both frames stacked to make a strong flash.
                for src in [src0, src1] {
                    draw_texture_ex(
                        tex,
                        x - size * 0.5,
                        receptor_y - size * 0.5,
                        Color::new(1.0, 1.0, 1.0, 0.92),
                        DrawTextureParams {
                            dest_size: Some(vec2(size, size)),
                            source: Some(src),
                            rotation: rot,
                            pivot: Some(vec2(x, receptor_y)),
                            ..Default::default()
                        },
                    );
                }
                let glow_size = size * 1.14;
                for src in [src0, src1] {
                    draw_texture_ex(
                        tex,
                        x - glow_size * 0.5,
                        receptor_y - glow_size * 0.5,
                        Color::new(1.0, 1.0, 1.0, 0.56),
                        DrawTextureParams {
                            dest_size: Some(vec2(glow_size, glow_size)),
                            source: Some(src),
                            rotation: rot,
                            pivot: Some(vec2(x, receptor_y)),
                            ..Default::default()
                        },
                    );
                }
            } else {
                // Frame 2: keep flashing only.
                draw_texture_ex(
                    tex,
                    x - size * 0.5,
                    receptor_y - size * 0.5,
                    Color::new(1.0, 1.0, 1.0, 0.96),
                    DrawTextureParams {
                        dest_size: Some(vec2(size, size)),
                        source: Some(src1),
                        rotation: rot,
                        pivot: Some(vec2(x, receptor_y)),
                        ..Default::default()
                    },
                );
                let glow_size = size * 1.18;
                draw_texture_ex(
                    tex,
                    x - glow_size * 0.5,
                    receptor_y - glow_size * 0.5,
                    Color::new(1.0, 1.0, 1.0, 0.52),
                    DrawTextureParams {
                        dest_size: Some(vec2(glow_size, glow_size)),
                        source: Some(src1),
                        rotation: rot,
                        pivot: Some(vec2(x, receptor_y)),
                        ..Default::default()
                    },
                );
            }
        }
        gl_use_default_material();
    }

    fn draw_hold_head_glow(&self, lane: usize, x: f32, receptor_y: f32, lane_spacing: f32) {
        let Some(tex) = &self.hit_explosion_tex else { return; };
        if let Some(m) = &self.hit_add_material {
            gl_use_material(m);
        }
        let frame_w = (tex.width() / 2.0).max(1.0);
        let frame_h = tex.height().max(1.0);
        let src0 = Rect::new(0.0, 0.0, frame_w, frame_h);
        let rot = self.noteskin_cfg.lane_rotation_deg[lane].to_radians();
        let size = (lane_spacing * ARROW_SIZE_RATIO * 1.08).max(1.0);
        draw_texture_ex(
            tex,
            x - size * 0.5,
            receptor_y - size * 0.5,
            Color::new(1.0, 1.0, 1.0, 0.68),
            DrawTextureParams {
                dest_size: Some(vec2(size, size)),
                source: Some(src0),
                rotation: rot,
                pivot: Some(vec2(x, receptor_y)),
                ..Default::default()
            },
        );
        let glow_size = size * 1.12;
        draw_texture_ex(
            tex,
            x - glow_size * 0.5,
            receptor_y - glow_size * 0.5,
            Color::new(1.0, 1.0, 1.0, 0.38),
            DrawTextureParams {
                dest_size: Some(vec2(glow_size, glow_size)),
                source: Some(src0),
                rotation: rot,
                pivot: Some(vec2(x, receptor_y)),
                ..Default::default()
            },
        );
        gl_use_default_material();
    }

    fn draw_jacket_layers(&self) {
        let Some(tex) = &self.jacket_tex else { return; };

        let card_size = (LOGICAL_W * 0.45 * 1.3 * 1.2).round();
        let card_x = (LOGICAL_W - card_size) * 0.5;
        let card_y = LOGICAL_H * 0.32;

        let blur_area_y = LOGICAL_H * 0.18;
        let blur_area_h = LOGICAL_H * 0.74;
        if let Some(bg) = &self.jacket_blur_tex {
            let blur_scale = (LOGICAL_W / bg.width()).max(blur_area_h / bg.height()) * 1.16;
            let blur_w = bg.width() * blur_scale;
            let blur_h = bg.height() * blur_scale;
            let blur_x = (LOGICAL_W - blur_w) * 0.5;
            let blur_y = blur_area_y + (blur_area_h - blur_h) * 0.5;
            draw_texture_ex(
                bg,
                blur_x,
                blur_y,
                Color::new(1.0, 1.0, 1.0, 0.72),
                DrawTextureParams {
                    dest_size: Some(vec2(blur_w, blur_h)),
                    ..Default::default()
                },
            );
        } else {
            let blur_scale = (LOGICAL_W / tex.width()).max(blur_area_h / tex.height()) * 1.16;
            let blur_w = tex.width() * blur_scale;
            let blur_h = tex.height() * blur_scale;
            let blur_x = (LOGICAL_W - blur_w) * 0.5;
            let blur_y = blur_area_y + (blur_area_h - blur_h) * 0.5;
            let taps: &[(f32, f32, f32)] = &[
                (0.0, 0.0, 0.18),
                (-26.0, 0.0, 0.10),
                (26.0, 0.0, 0.10),
                (0.0, -26.0, 0.10),
                (0.0, 26.0, 0.10),
                (-18.0, -18.0, 0.075),
                (18.0, -18.0, 0.075),
                (-18.0, 18.0, 0.075),
                (18.0, 18.0, 0.075),
                (-38.0, 0.0, 0.06),
                (38.0, 0.0, 0.06),
                (0.0, -38.0, 0.06),
                (0.0, 38.0, 0.06),
                (-28.0, -28.0, 0.045),
                (28.0, -28.0, 0.045),
                (-28.0, 28.0, 0.045),
                (28.0, 28.0, 0.045),
            ];
            for (dx, dy, a) in taps {
                draw_texture_ex(
                    tex,
                    blur_x + dx,
                    blur_y + dy,
                    Color::new(1.0, 1.0, 1.0, *a),
                    DrawTextureParams {
                        dest_size: Some(vec2(blur_w, blur_h)),
                        ..Default::default()
                    },
                );
            }
        }
        draw_rectangle(0.0, 0.0, LOGICAL_W, LOGICAL_H, Color::from_rgba(5, 8, 15, 92));

        draw_texture_ex(
            tex,
            card_x,
            card_y,
            Color::new(0.5, 0.5, 0.5, 1.0),
            DrawTextureParams {
                dest_size: Some(vec2(card_size, card_size)),
                ..Default::default()
            },
        );
        draw_rectangle_lines(card_x, card_y, card_size, card_size, 2.0, Color::from_rgba(220, 230, 255, 170));
    }

    fn draw_arrow(
        &self,
        lane: usize,
        beat: f32,
        x: f32,
        y: f32,
        lane_spacing: f32,
        tint: Color,
        receptor: bool,
        song_time: f32,
        fixed_col: Option<f32>,
    ) {
        let size = (lane_spacing * ARROW_SIZE_RATIO).max(1.0);
        let mut color_row = note_color_frame(
            beat,
            self.noteskin_cfg.note_color_count,
            self.noteskin_cfg.note_color_denominator,
        ) as f32;
        let mut base_tint = tint;
        if receptor {
            // Follow Receptor.lua vibe: gray glowblink on beat.
            color_row = 8.0;
            let blink = ((song_time as f64 * 8.0).sin() * 0.5 + 0.5) as f32;
            let g = 0.40 + (0.80 - 0.40) * blink;
            base_tint = Color::new(g, g, g, 0.9);
        }
        let rot = self.noteskin_cfg.lane_rotation_deg[lane].to_radians();
        let (curr_col, next_col, next_alpha) = if let Some(c) = fixed_col {
            (c, c, 0.0)
        } else if receptor {
            (1.0, 1.0, 0.0) // top receptors fixed to frame #2
        } else {
            animated_cols_4123_accum(song_time, self.anim_start_time)
        };
        if let Some(tex) = self.dir_arrow_tex.get(lane).and_then(|t| t.as_ref()) {
            let cols = 8.0;
            let rows = 9.0;
            let frame_w = tex.width() / cols;
            let frame_h = tex.height() / rows;
            let src_y = frame_h * color_row.min(rows - 1.0);
            let scale = size / frame_w.max(1.0);
            let dw = frame_w * scale;
            let dh = frame_h * scale;
            let src_curr = frame_w * curr_col.min(cols - 1.0);
            draw_texture_ex(
                tex,
                x - dw * 0.5,
                y - dh * 0.5,
                base_tint,
                DrawTextureParams {
                    dest_size: Some(vec2(dw, dh)),
                    source: Some(Rect::new(src_curr, src_y, frame_w, frame_h)),
                    rotation: rot,
                    pivot: Some(vec2(x, y)),
                    ..Default::default()
                },
            );
            if next_alpha > 0.001 && !receptor && fixed_col.is_none() {
                let src_x = frame_w * next_col.min(cols - 1.0);
                draw_texture_ex(
                    tex,
                    x - dw * 0.5,
                    y - dh * 0.5,
                    Color::new(base_tint.r, base_tint.g, base_tint.b, base_tint.a * next_alpha),
                    DrawTextureParams {
                        dest_size: Some(vec2(dw, dh)),
                        source: Some(Rect::new(src_x, src_y, frame_w, frame_h)),
                        rotation: rot,
                        pivot: Some(vec2(x, y)),
                        ..Default::default()
                    },
                );
            }
            return;
        }

        if let Some(tex) = &self.arrow_tex {
            let frame_h = tex.height() / 8.0;
            let frame_w = tex.width();
            let frame = color_row.min(7.0);
            draw_texture_ex(
                tex,
                x - size * 0.5,
                y - size * 0.5,
                tint,
                DrawTextureParams {
                    dest_size: Some(vec2(size, size)),
                    source: Some(Rect::new(0.0, frame_h * frame, frame_w, frame_h)),
                    rotation: rot,
                    pivot: Some(vec2(x, y)),
                    ..Default::default()
                },
            );
            return;
        }

        draw_poly(x, y, 3, 66.0, lane as f32 * 90.0, tint);
    }

    fn draw_hold_column(
        &self,
        lane: usize,
        x: f32,
        head_y: f32,
        tail_y: f32,
        lane_spacing: f32,
        body_scroll_px: f32,
        is_active: bool,
    ) {
        let top_y = head_y.min(tail_y);
        let bottom_y = head_y.max(tail_y);
        let full_h = (bottom_y - top_y).max(1.0);
        // Match StepMania-like hold width: almost same visual width as note head.
        let hold_w = (lane_spacing * ARROW_SIZE_RATIO).max(1.0);
        let left = x - hold_w * 0.5;

        let body_tex = if is_active {
            self.hold_body_active_tex.get(lane).and_then(|t| t.as_ref())
        } else {
            self.hold_body_inactive_tex.get(lane).and_then(|t| t.as_ref())
        };
        // Keep hold head as the normal arrow head and force body overlap to avoid seam/gap.
        let top_tex: Option<&Texture2D> = None;
        let bottom_tex = if is_active {
            self.hold_bottomcap_active_tex
                .get(lane)
                .and_then(|t| t.as_ref())
        } else {
            self.hold_bottomcap_inactive_tex
                .get(lane)
                .and_then(|t| t.as_ref())
        };
        let top_nat_h = top_tex
            .map(|t| hold_w * (t.height() / t.width().max(1.0)))
            .unwrap_or(0.0);
        let bottom_nat_h = bottom_tex
            .map(|t| hold_w * (t.height() / t.width().max(1.0)))
            .unwrap_or(0.0);
        let top_cap_h = top_nat_h.min(full_h);
        let bottom_cap_h = bottom_nat_h.min((full_h - top_cap_h).max(0.0));
        // Small overlap with head to prevent top seam while avoiding visible gap.
        let head_cover = (hold_w * 0.03).max(2.0);
        let body_start_y = top_y + top_cap_h + head_cover;
        let body_end_y = (bottom_y - bottom_cap_h).max(body_start_y);
        let body_h = (body_end_y - body_start_y).max(0.0);

        if let Some(tex) = body_tex {
            let tw = tex.width().max(1.0);
            let th = tex.height().max(1.0);
            let tile_h = (hold_w * (th / tw)).max(1.0);
            // Bottom-anchored tiling: bottom is never clipped, only top can be clipped.
            // This preserves tail connection and avoids broken lower-frame stitching.
            let _ = body_scroll_px; // reserved for later phase animation refinement
            let mut y_bottom = body_end_y;
            while y_bottom > body_start_y + 0.001 {
                let draw_h = tile_h.min(y_bottom - body_start_y);
                let y_top = y_bottom - draw_h;
                let src_h = th * (draw_h / tile_h);
                let src_y = th - src_h;
                draw_texture_ex(
                    tex,
                    left,
                    y_top,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(vec2(hold_w, draw_h)),
                        source: Some(Rect::new(0.0, src_y, tw, src_h)),
                        ..Default::default()
                    },
                );
                y_bottom = y_top;
            }
        } else if body_h > 0.5 {
            let c = if is_active {
                Color::new(0.35, 0.95, 0.55, 0.62)
            } else {
                Color::new(0.55, 0.55, 0.55, 0.52)
            };
            draw_rectangle(left, body_start_y, hold_w, body_h, c);
        }

        if let Some(tex) = top_tex {
            let tw = tex.width().max(1.0);
            let th = tex.height().max(1.0);
            let src_h = if top_nat_h > 0.0 {
                th * (top_cap_h / top_nat_h)
            } else {
                th
            };
            draw_texture_ex(
                tex,
                left,
                top_y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(hold_w, top_cap_h)),
                    source: Some(Rect::new(0.0, 0.0, tw, src_h)),
                    ..Default::default()
                },
            );
        }

        if let Some(tex) = bottom_tex {
            let tw = tex.width().max(1.0);
            let th = tex.height().max(1.0);
            let src_h = if bottom_nat_h > 0.0 {
                th * (bottom_cap_h / bottom_nat_h)
            } else {
                th
            };
            draw_texture_ex(
                tex,
                left,
                bottom_y - bottom_cap_h,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(hold_w, bottom_cap_h)),
                    source: Some(Rect::new(0.0, th - src_h, tw, src_h)),
                    ..Default::default()
                },
            );
        }
    }
    fn draw_text_ui(&self, text: &str, x: f32, y: f32, size: f32, color: Color) {
        if let Some(font) = self.ui_font.as_ref() {
            draw_text_ex(
                text,
                x,
                y,
                TextParams {
                    font: Some(font),
                    font_size: size as u16,
                    font_scale: 1.0,
                    color,
                    ..Default::default()
                },
            );
        } else {
            draw_text(text, x, y, size, color);
        }
    }

    fn measure_text_ui(&self, text: &str, font_size: u16, font_scale: f32) -> TextDimensions {
        measure_text(text, self.ui_font.as_ref(), font_size, font_scale)
    }
}

fn start_audio(
    path: Option<&PathBuf>,
    speed: f32,
) -> (
    Option<OutputStream>,
    Option<OutputStreamHandle>,
    Option<Sink>,
    Option<PathBuf>,
) {
    let Some(path) = path else {
        println!("Audio: no music path resolved");
        return (None, None, None, None);
    };

    println!("Audio: primary path={}", path.display());

    let Ok((stream, handle)) = OutputStream::try_default() else {
        println!("Audio: no default output device");
        return (None, None, None, Some(path.clone()));
    };

    if let Some(sink) = create_sink_for_path(&handle, path, speed, 0.0) {
        println!("Audio: playing {}", path.display());
        (Some(stream), Some(handle), Some(sink), Some(path.clone()))
    } else {
        println!("Audio: failed to start playback");
        (Some(stream), Some(handle), None, Some(path.clone()))
    }
}

fn audio_candidate_paths(primary: &Path) -> Vec<PathBuf> {
    let mut candidates = vec![primary.to_path_buf()];
    let parent = primary.parent().unwrap_or_else(|| Path::new("."));
    let stem = primary.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    for ext in ["ogg", "mp3", "wav", "m4a", "aac", "flac"] {
        let p = parent.join(format!("{stem}.{ext}"));
        if p != primary && p.exists() {
            candidates.push(p);
        }
    }
    candidates
}

fn create_sink_for_path(
    handle: &OutputStreamHandle,
    path: &Path,
    speed: f32,
    start_sec: f32,
) -> Option<Sink> {
    for p in audio_candidate_paths(path) {
        if let Ok(src) = build_audio_source_for_path(&p) {
            let Ok(sink) = Sink::try_new(handle) else {
                return None;
            };
            sink.set_speed(speed.clamp(SPEED_MIN, SPEED_MAX));
            sink.append(src.skip_duration(Duration::from_secs_f32(start_sec.max(0.0))));
            return Some(sink);
        }
    }
    None
}

fn load_decoded_audio_clip(path: Option<&Path>) -> Option<DecodedAudioClip> {
    let path = path?;
    for p in audio_candidate_paths(path) {
        if let Ok(samples) = decode_with_symphonia_to_samples(&p) {
            return Some(DecodedAudioClip {
                channels: samples.channels(),
                sample_rate: samples.sample_rate(),
                samples: Arc::new(samples.collect()),
            });
        }
    }
    None
}

fn load_decoded_audio_clip_from_candidates(candidates: &[&str]) -> Option<DecodedAudioClip> {
    for c in candidates {
        let p = Path::new(c);
        if !p.exists() {
            continue;
        }
        if let Ok(samples) = decode_with_symphonia_to_samples(p) {
            return Some(DecodedAudioClip {
                channels: samples.channels(),
                sample_rate: samples.sample_rate(),
                samples: Arc::new(samples.collect()),
            });
        }
    }
    None
}

fn create_sink_from_decoded(
    handle: &OutputStreamHandle,
    clip: &DecodedAudioClip,
    speed: f32,
    start_sec: f32,
) -> Option<Sink> {
    let ch = clip.channels.max(1);
    let sr = clip.sample_rate.max(1);
    let frame_offset = (start_sec.max(0.0) * sr as f32).floor() as usize;
    let sample_offset = frame_offset.saturating_mul(ch as usize);
    let tail = if sample_offset < clip.samples.len() {
        clip.samples[sample_offset..].to_vec()
    } else {
        Vec::new()
    };
    let Ok(sink) = Sink::try_new(handle) else {
        return None;
    };
    sink.set_speed(speed.clamp(SPEED_MIN, SPEED_MAX));
    sink.append(SamplesBuffer::new(ch, sr, tail));
    Some(sink)
}

fn build_audio_source_for_path(path: &Path) -> Result<Box<dyn Source<Item = f32> + Send>, String> {
    if let Ok(decoder) = try_build_decoder(path) {
        return Ok(Box::new(decoder.convert_samples::<f32>()));
    }
    if let Ok(mem_src) = decode_with_symphonia_to_samples(path) {
        return Ok(Box::new(mem_src));
    }
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ext == "ogg" {
        let ogg = decode_ogg_with_lewton(path)?;
        return Ok(Box::new(ogg));
    }
    Err(format!("decode failed {}", path.display()))
}

pub fn build_preview_source_for_path(path: &Path) -> Result<Box<dyn Source<Item = f32> + Send>, String> {
    build_audio_source_for_path(path)
}

fn decode_with_symphonia_to_samples(path: &Path) -> Result<SamplesBuffer<f32>, String> {
    let file = File::open(path).map_err(|e| format!("symphonia open failed: {e}"))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| format!("symphonia probe failed: {e}"))?;
    let mut format = probed.format;

    let track = format
        .default_track()
        .ok_or_else(|| "symphonia no default track".to_string())?;
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("symphonia make decoder failed: {e}"))?;

    let channels = track
        .codec_params
        .channels
        .ok_or_else(|| "symphonia missing channels".to_string())?
        .count() as u16;
    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| "symphonia missing sample_rate".to_string())?;

    let mut samples = Vec::<f32>::new();
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(_) => break,
        };
        if packet.track_id() != track_id {
            continue;
        }
        if let Ok(audio) = decoder.decode(&packet) {
            let spec = *audio.spec();
            let mut sample_buf = SampleBuffer::<f32>::new(audio.capacity() as u64, spec);
            sample_buf.copy_interleaved_ref(audio);
            samples.extend_from_slice(sample_buf.samples());
        }
    }

    if samples.is_empty() {
        return Err("symphonia decoded empty stream".to_string());
    }
    Ok(SamplesBuffer::new(channels, sample_rate, samples))
}

fn try_build_decoder(path: &Path) -> Result<Decoder<BufReader<File>>, String> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let file = File::open(path).map_err(|_| format!("open failed {}", path.display()))?;
    let reader = BufReader::new(file);
    let specific = match ext.as_str() {
        "wav" => Decoder::new_wav(reader),
        "mp3" => Decoder::new_mp3(reader),
        "ogg" => Decoder::new_vorbis(reader),
        "aac" | "m4a" => Decoder::new_aac(reader),
        _ => Decoder::new(reader),
    };
    specific
        .or_else(|_| {
            let f2 = File::open(path).map_err(|_| format!("open retry failed {}", path.display()))?;
            Decoder::new(BufReader::new(f2)).map_err(|e| e.to_string())
        })
        .map_err(|e| format!("decoder failed: {e}"))
}

struct LewtonOggSource {
    reader: OggStreamReader<BufReader<File>>,
    current_packet: Vec<i16>,
    packet_index: usize,
    channels: u16,
    sample_rate: u32,
    finished: bool,
}

fn decode_ogg_with_lewton(path: &Path) -> Result<LewtonOggSource, String> {
    let file = File::open(path).map_err(|_| format!("open failed {}", path.display()))?;
    let ogg = OggStreamReader::new(BufReader::new(file))
        .map_err(|e| format!("lewton open failed: {e}"))?;
    Ok(LewtonOggSource {
        channels: ogg.ident_hdr.audio_channels as u16,
        sample_rate: ogg.ident_hdr.audio_sample_rate,
        reader: ogg,
        current_packet: Vec::new(),
        packet_index: 0,
        finished: false,
    })
}

impl Iterator for LewtonOggSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        loop {
            if self.packet_index < self.current_packet.len() {
                let s = self.current_packet[self.packet_index];
                self.packet_index += 1;
                return Some((s as f32) / 32768.0);
            }
            match self.reader.read_dec_packet_itl() {
                Ok(Some(packet)) => {
                    self.current_packet = packet;
                    self.packet_index = 0;
                }
                Ok(None) => {
                    self.finished = true;
                    return None;
                }
                Err(_) => {
                    self.finished = true;
                    return None;
                }
            }
        }
    }
}

impl Source for LewtonOggSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        self.channels
    }
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

fn note_color_frame(beat: f32, note_color_count: usize, denominator_mode: bool) -> usize {
    if !denominator_mode {
        return 0;
    }
    // StepMania denominator-style note coloring using 48 rows per beat.
    let row = (beat * 48.0).round() as i32;
    let abs = row.abs();
    let idx = if abs % 48 == 0 {
        0
    } else if abs % 24 == 0 {
        1
    } else if abs % 16 == 0 {
        2
    } else if abs % 12 == 0 {
        3
    } else if abs % 8 == 0 {
        4
    } else if abs % 6 == 0 {
        5
    } else if abs % 4 == 0 {
        6
    } else {
        7
    };
    idx.min(note_color_count.saturating_sub(1))
}

fn animated_cols_4123_accum(song_time: f32, anim_start_time: f64) -> (f32, f32, f32) {
    // Cycle base frames with first visible image as #2.
    const SEQ: [f32; 4] = [1.0, 2.0, 3.0, 0.0];
    let t = if song_time >= 0.0 {
        song_time as f64
    } else {
        (get_time() - anim_start_time).max(0.0)
    };
    let pos = t * NOTE_ANIM_FPS;
    let idx = pos.floor() as usize;
    let curr = SEQ[idx % SEQ.len()];
    let next = SEQ[(idx + 1) % SEQ.len()];
    let phase = (pos - pos.floor()) as f32;
    // Cumulative transition: current frame stays opaque, next frame fades in.
    let eased = phase.powf(0.85);
    let next_alpha = eased;
    (curr, next, next_alpha)
}

fn load_saved_game_speed() -> f32 {
    let raw = fs::read_to_string(game_speed_path()).ok();
    let parsed = raw
        .as_deref()
        .unwrap_or("")
        .trim()
        .parse::<f32>()
        .unwrap_or(1.0);
    parsed.clamp(GAME_SPEED_MIN, GAME_SPEED_MAX)
}

fn save_game_speed(speed: f32) {
    let path = game_speed_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(
        path,
        format!("{:.1}", speed.clamp(GAME_SPEED_MIN, GAME_SPEED_MAX)),
    );
}

fn game_speed_path() -> PathBuf {
    app_storage_root().join(GAME_SPEED_FILE)
}

fn app_storage_root() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        let p1 = PathBuf::from("/storage/emulated/0/stepmania");
        if p1.exists() {
            return p1;
        }
        let p2 = PathBuf::from("/sdcard/stepmania");
        if p2.exists() {
            return p2;
        }
        p1
    }
    #[cfg(not(target_os = "android"))]
    {
        PathBuf::from("mini_stepmania_rust")
    }
}

fn load_noteskin_config() -> NoteskinConfig {
    let mut cfg = NoteskinConfig::default();
    let editor_metrics = Path::new("mini_stepmania_rust/NoteSkins/common/_Editor/metrics.ini");
    let common_metrics = Path::new("mini_stepmania_rust/NoteSkins/common/common/metrics.ini");
    apply_metrics_file(common_metrics, &mut cfg);
    apply_metrics_file(editor_metrics, &mut cfg);

    let editor_lua = Path::new("mini_stepmania_rust/NoteSkins/common/_Editor/NoteSkin.lua");
    apply_rotate_lua(editor_lua, &mut cfg);
    cfg
}

fn apply_metrics_file(path: &Path, cfg: &mut NoteskinConfig) {
    let Ok(raw) = fs::read_to_string(path) else { return; };
    let mut in_notedisplay = false;
    let mut kv = HashMap::<String, String>::new();
    for line in raw.lines() {
        let line = line.split("//").next().unwrap_or("").split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_notedisplay = &line[1..line.len() - 1] == "NoteDisplay";
            continue;
        }
        if !in_notedisplay {
            continue;
        }
        if let Some(eq) = line.find('=') {
            let k = line[..eq].trim().to_string();
            let v = line[eq + 1..].trim().to_string();
            kv.insert(k, v);
        }
    }

    if let Some(v) = kv.get("TapNoteNoteColorCount").and_then(|v| v.parse::<usize>().ok()) {
        cfg.note_color_count = v.max(1);
    } else if let Some(sp) = kv
        .get("TapNoteNoteColorTextureCoordSpacingY")
        .and_then(|v| v.parse::<f32>().ok())
        .filter(|v| *v > 0.0)
    {
        cfg.note_color_count = ((1.0 / sp).round() as usize).max(1);
    }
    if let Some(t) = kv.get("TapNoteNoteColorType") {
        cfg.note_color_denominator = t.eq_ignore_ascii_case("Denominator");
    }
}

fn apply_rotate_lua(path: &Path, cfg: &mut NoteskinConfig) {
    let Ok(raw) = fs::read_to_string(path) else { return; };
    let mut in_rotate = false;
    let mut map = HashMap::<String, f32>::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.starts_with("ret.Rotate") {
            in_rotate = true;
            continue;
        }
        if !in_rotate {
            continue;
        }
        if line.starts_with("};") || line == "}" {
            break;
        }
        if let Some(eq) = line.find('=') {
            let k = line[..eq].trim().trim_matches(',').to_string();
            let v = line[eq + 1..].trim().trim_matches(',').parse::<f32>();
            if let Ok(v) = v {
                map.insert(k, v);
            }
        }
    }
    if let Some(v) = map.get("Left") {
        cfg.lane_rotation_deg[0] = *v;
    }
    if let Some(v) = map.get("Down") {
        cfg.lane_rotation_deg[1] = *v;
    }
    if let Some(v) = map.get("Up") {
        cfg.lane_rotation_deg[2] = *v;
    }
    if let Some(v) = map.get("Right") {
        cfg.lane_rotation_deg[3] = *v;
    }
}

async fn load_arrow_texture() -> Option<Texture2D> {
    for c in ARROW_CANDIDATES
        .iter()
        .chain(ARROW_FALLBACK_CANDIDATES.iter())
        .copied()
    {
        let p = Path::new(c);
        if !p.exists() {
            continue;
        }
        if let Ok(t) = load_texture(&p.display().to_string()).await {
            t.set_filter(FilterMode::Linear);
            return Some(t);
        }
    }
    None
}

async fn load_direction_arrow_textures() -> [Option<Texture2D>; LANE_COUNT] {
    [
        load_texture_candidates(&DIR_LEFT_CANDIDATES).await,
        load_texture_candidates(&DIR_DOWN_CANDIDATES).await,
        load_texture_candidates(&DIR_UP_CANDIDATES).await,
        load_texture_candidates(&DIR_RIGHT_CANDIDATES).await,
    ]
}

async fn load_hold_body_active_textures() -> [Option<Texture2D>; LANE_COUNT] {
    let mut out: [Option<Texture2D>; LANE_COUNT] = [None, None, None, None];
    for (lane, slot) in out.iter_mut().enumerate() {
        let mut tex = load_hold_part_texture_for_lane(lane, "Roll Body Active (res 64x32).png").await;
        if tex.is_none() {
            tex = load_hold_part_texture_for_lane(lane, "Hold Body Active (res 64x32).png").await;
        }
        *slot = tex;
    }
    out
}

async fn load_hold_body_inactive_textures() -> [Option<Texture2D>; LANE_COUNT] {
    let mut out: [Option<Texture2D>; LANE_COUNT] = [None, None, None, None];
    for (lane, slot) in out.iter_mut().enumerate() {
        let mut tex =
            load_hold_part_texture_for_lane(lane, "Roll Body Inactive (res 64x32).png").await;
        if tex.is_none() {
            tex = load_hold_part_texture_for_lane(lane, "Hold Body Inactive (res 64x32).png").await;
        }
        *slot = tex;
    }
    out
}

async fn load_hold_topcap_active_textures() -> [Option<Texture2D>; LANE_COUNT] {
    [
        load_texture_candidates(&HOLD_LEFT_TOPCAP_ACTIVE_CANDIDATES).await,
        load_texture_candidates(&HOLD_DOWN_TOPCAP_ACTIVE_CANDIDATES).await,
        load_texture_candidates(&HOLD_UP_TOPCAP_ACTIVE_CANDIDATES).await,
        load_texture_candidates(&HOLD_RIGHT_TOPCAP_ACTIVE_CANDIDATES).await,
    ]
}

async fn load_hold_bottomcap_active_textures() -> [Option<Texture2D>; LANE_COUNT] {
    let mut out: [Option<Texture2D>; LANE_COUNT] = [None, None, None, None];
    for (lane, slot) in out.iter_mut().enumerate() {
        let mut tex = load_hold_part_texture_for_lane_variants(
            lane,
            &[
                "Roll BottomCap active (res 64x64).png",
                "Roll BottomCap Active (res 64x64).png",
                "Roll BottomCap active (res 64x32).png",
                "Roll BottomCap Active (res 64x32).png",
            ],
        )
        .await;
        if tex.is_none() {
            tex = load_hold_part_texture_for_lane_variants(
                lane,
                &[
                    "Hold BottomCap active (res 64x64).png",
                    "Hold BottomCap Active (res 64x64).png",
                    "Hold BottomCap active (res 64x32).png",
                    "Hold BottomCap Active (res 64x32).png",
                ],
            )
            .await;
        }
        *slot = tex;
    }
    out
}

async fn load_hold_bottomcap_inactive_textures() -> [Option<Texture2D>; LANE_COUNT] {
    let mut out: [Option<Texture2D>; LANE_COUNT] = [None, None, None, None];
    for (lane, slot) in out.iter_mut().enumerate() {
        let mut tex = load_hold_part_texture_for_lane_variants(
            lane,
            &[
                "Roll BottomCap inactive (res 64x64).png",
                "Roll BottomCap Inactive (res 64x64).png",
                "Roll BottomCap inactive (res 64x32).png",
                "Roll BottomCap Inactive (res 64x32).png",
            ],
        )
        .await;
        if tex.is_none() {
            tex = load_hold_part_texture_for_lane_variants(
                lane,
                &[
                    "Hold BottomCap inactive (res 64x64).png",
                    "Hold BottomCap Inactive (res 64x64).png",
                    "Hold BottomCap inactive (res 64x32).png",
                    "Hold BottomCap Inactive (res 64x32).png",
                ],
            )
            .await;
        }
        *slot = tex;
    }
    out
}

async fn load_hold_part_texture_for_lane_variants(
    lane: usize,
    file_names: &[&str],
) -> Option<Texture2D> {
    for f in file_names {
        if let Some(t) = load_hold_part_texture_for_lane(lane, f).await {
            return Some(t);
        }
    }
    None
}

async fn load_hold_part_texture_for_lane(lane: usize, file_name: &str) -> Option<Texture2D> {
    let lane_name = HOLD_LANE_NAMES.get(lane).copied().unwrap_or("Down");
    let candidates = [
        format!("mini_stepmania_rust/NoteSkins/common/_Editor/{lane_name} {file_name}"),
        format!("NoteSkins/common/_Editor/{lane_name} {file_name}"),
    ];
    for c in candidates {
        let p = Path::new(&c);
        if !p.exists() {
            continue;
        }
        if let Ok(t) = load_texture(&p.display().to_string()).await {
            t.set_filter(FilterMode::Linear);
            return Some(t);
        }
    }
    None
}

async fn load_jacket_texture(music_path: Option<&Path>) -> Option<Texture2D> {
    let Some(music_path) = music_path else {
        return None;
    };
    let Some(folder) = music_path.parent() else {
        return None;
    };
    let Some(stem) = music_path.file_stem().and_then(|s| s.to_str()) else {
        return None;
    };

    let mut candidates: Vec<PathBuf> = vec![
        folder.join(format!("{stem}-jacket.png")),
        folder.join(format!("{stem}-jacket.jpg")),
        folder.join(format!("{stem}-jacket.jpeg")),
    ];

    if let Ok(entries) = fs::read_dir(folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            let lower = name.to_ascii_lowercase();
            let is_jacket = lower.ends_with("-jacket.png")
                || lower.ends_with("-jacket.jpg")
                || lower.ends_with("-jacket.jpeg");
            if is_jacket && !candidates.iter().any(|p| p == &path) {
                candidates.push(path);
            }
        }
    }

    for path in candidates {
        if !path.exists() {
            continue;
        }
        if let Ok(sharp) = load_texture(&path.display().to_string()).await {
            sharp.set_filter(FilterMode::Linear);
            return Some(sharp);
        }
    }
    None
}

fn build_jacket_blur_gpu(jacket: Option<&Texture2D>) -> (Option<RenderTarget>, Option<Texture2D>) {
    let Some(tex) = jacket else {
        return (None, None);
    };
    let src_w = tex.width().max(1.0);
    let src_h = tex.height().max(1.0);
    let rt = render_target(src_w as u32, src_h as u32);
    rt.texture.set_filter(FilterMode::Linear);

    let mut cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, src_w, src_h));
    cam.render_target = Some(rt.clone());
    set_camera(&cam);
    clear_background(BLACK);

    // Full-resolution weighted kernel blur (no downscale, no block artifacts).
    let taps: &[(f32, f32, f32)] = &[
        (0.0, 0.0, 0.120),
        (-2.0, 0.0, 0.095), (2.0, 0.0, 0.095), (0.0, -2.0, 0.095), (0.0, 2.0, 0.095),
        (-4.0, 0.0, 0.070), (4.0, 0.0, 0.070), (0.0, -4.0, 0.070), (0.0, 4.0, 0.070),
        (-6.0, 0.0, 0.050), (6.0, 0.0, 0.050), (0.0, -6.0, 0.050), (0.0, 6.0, 0.050),
        (-8.0, 0.0, 0.035), (8.0, 0.0, 0.035), (0.0, -8.0, 0.035), (0.0, 8.0, 0.035),
        (-3.0, -3.0, 0.055), (3.0, -3.0, 0.055), (-3.0, 3.0, 0.055), (3.0, 3.0, 0.055),
        (-5.0, -5.0, 0.040), (5.0, -5.0, 0.040), (-5.0, 5.0, 0.040), (5.0, 5.0, 0.040),
        (-7.0, -7.0, 0.026), (7.0, -7.0, 0.026), (-7.0, 7.0, 0.026), (7.0, 7.0, 0.026),
    ];
    for (dx, dy, a) in taps {
        draw_texture_ex(
            tex,
            *dx,
            *dy,
            Color::new(1.0, 1.0, 1.0, *a),
            DrawTextureParams {
                dest_size: Some(vec2(src_w, src_h)),
                ..Default::default()
            },
        );
    }

    set_default_camera();
    (Some(rt.clone()), Some(rt.texture))
}

async fn load_texture_candidates(candidates: &[&str]) -> Option<Texture2D> {
    for c in candidates {
        let p = Path::new(c);
        if !p.exists() {
            continue;
        }
        if let Ok(t) = load_texture(&p.display().to_string()).await {
            t.set_filter(FilterMode::Linear);
            return Some(t);
        }
    }
    None
}

async fn load_ui_font() -> Option<Font> {
    for p in UI_FONT_CANDIDATES {
        if Path::new(p).is_file() {
            if let Ok(font) = load_ttf_font(p).await {
                return Some(font);
            }
        }
    }
    None
}

fn load_additive_material() -> Option<Material> {
    let vertex = r#"
        attribute vec3 position;
        attribute vec2 texcoord;
        attribute vec4 color0;
        varying lowp vec2 uv;
        varying lowp vec4 color;
        uniform mat4 Model;
        uniform mat4 Projection;
        void main() {
            gl_Position = Projection * Model * vec4(position, 1.0);
            color = color0 / 255.0;
            uv = texcoord;
        }
    "#;
    let fragment = r#"
        varying lowp vec2 uv;
        varying lowp vec4 color;
        uniform sampler2D Texture;
        void main() {
            gl_FragColor = texture2D(Texture, uv) * color;
        }
    "#;
    let mut params = PipelineParams::default();
    params.color_blend = Some(BlendState::new(
        Equation::Add,
        BlendFactor::One,
        BlendFactor::One,
    ));
    params.alpha_blend = Some(BlendState::new(
        Equation::Add,
        BlendFactor::One,
        BlendFactor::One,
    ));
    load_material(
        ShaderSource::Glsl {
            vertex,
            fragment,
        },
        MaterialParams {
            pipeline_params: params,
            ..Default::default()
        },
    )
    .ok()
}
