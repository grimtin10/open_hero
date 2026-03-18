mod chart;
mod config;
mod input;
mod render;

use std::{collections::VecDeque, sync::OnceLock, time::{Duration, Instant}};

use kira::{sound::streaming::StreamingSoundData, AudioManager, AudioManagerSettings, Tween};
use macroquad::prelude::*;

use crate::{chart::{Difficulty, Instrument, Note}, config::{Config, load_config}, input::InputManager, render::*};

// haha it says fart
const FAR_T: f32 = 0.0;
const NEAR_T: f32 = 1.5;
const FADE_T: f32 = 0.1;

// clone hero hit window (for now)
const HIT_FRONT: f64 = 0.07;
const HIT_BACK:  f64 = 0.07;

struct NoteAssets {
    pub note: Texture2D,
    pub hopo: Texture2D,
    pub tap: Texture2D,
    pub wor_tap: Texture2D,
    pub open: Texture2D,
    pub open_hopo: Texture2D,
}

struct FretAssets {
    pub fret: Texture2D,
    pub fret_pressed: Texture2D,
    pub pressed: Texture2D,
    pub ring: Texture2D,
    pub shell: Texture2D,
}

struct Assets {
    pub notes: NoteAssets,

    pub frets: [FretAssets; 3],
    pub fret_piston: Texture2D, // doesn't differ between frets so we just have it here
}

#[derive(Debug, Default, Clone, Copy)]
struct FretState {
    pub height: f32,
}

#[derive(Debug, Default, Clone, Copy)]
struct Strikeline {
    pub frets: [FretState; 5],
    pub pressed: u8,
}

#[derive(Debug, Clone, Copy)]
struct NoteContainer {
    pub note: Note,
    pub t: f32,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

fn config() -> &'static Config {
    CONFIG.get_or_init(|| load_config().expect("failed to load config"))
}

fn window_conf() -> Conf {
    let config = config();
    Conf {
        window_title: "Open Hero".into(),
        window_width: config.width as i32,
        window_height: config.height as i32,
        window_resizable: config.resizable,

        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let start = Instant::now();
    let assets = load_assets("assets").await;
    println!("loading assets took {}ms", start.elapsed().as_millis());

    let config = config();

    let mut strikeline = Strikeline::default();

    let song_name = "Star";
    let audio_file = "song.ogg";
    let song = chart::parse(format!("songs/{song_name}/notes.chart")).unwrap();
    let chart = song.charts.get(&(Instrument::Single, Difficulty::Expert)).unwrap();
    let mut notes: VecDeque<NoteContainer> = chart.notes.iter().map(|note| NoteContainer { note: *note, t: 0.0 }).collect();

    let volume = -12.0;
    // let volume = -100000.0;
    let mut manager: AudioManager = AudioManager::new(AudioManagerSettings::default()).unwrap();
    let audio = StreamingSoundData::from_file(format!("songs/{song_name}/{audio_file}")).unwrap().volume(volume);

    let mut audio_playing = false;
    let mut audio_handle = manager.play(audio).unwrap();
    audio_handle.pause(Tween {
        duration: Duration::from_millis(1),
        ..Default::default()
    });
    audio_handle.seek_to(0.0);

    let mut input = InputManager::new(false);

    let mut time = -2.5;
    let mut time_offset = 0.0; // offset between input system time and game time
    let mut frame_count = 0;
    loop {
        clear_background(Color::from_rgba(0, 0, 0, 0));

        handle_inputs(&mut input, &mut strikeline, time, &mut notes);

        // highway background
        draw_polygon(&[
            vec2(t_to_x(NEAR_T, -0.5), t_to_y(NEAR_T)),
            vec2(t_to_x(NEAR_T, 4.5), t_to_y(NEAR_T)),
            vec2(t_to_x(FAR_T, 4.5), t_to_y(FAR_T)),
            vec2(t_to_x(FAR_T, -0.5), t_to_y(FAR_T)),
        ], BLACK);

        // hit window
        let hit_start = perspective(time_to_t(HIT_FRONT, config.notespeed));
        let hit_end = perspective(time_to_t(-HIT_BACK, config.notespeed));
        draw_polygon(&[
            vec2(t_to_x(hit_start, -0.5), t_to_y(hit_start)),
            vec2(t_to_x(hit_start, 4.5), t_to_y(hit_start)),
            vec2(t_to_x(hit_end,   4.5), t_to_y(hit_end)),
            vec2(t_to_x(hit_end,   -0.5), t_to_y(hit_end)),
        ], Color::new(1.0, 1.0, 1.0, 0.25));

        // strikeline
        for i in 0..5 {
            render_fret(&assets, i, strikeline.frets[i], strikeline.pressed >> i & 1 == 1);
        }

        // update `t` values
        for note in &mut notes {
            note.t = time_to_t(note.note.time - time, config.notespeed);
        }

        // find the visible range
        let render_start = notes.partition_point(|n| n.t > NEAR_T);
        let render_end = notes.partition_point(|n| n.t > FAR_T);

        // render notes
        for note in notes.range(render_start..render_end).rev() {
            render_note(&assets, config, &note.note, note.t);
        }

        draw_fps();

        // skip the first couple frames because of large frame times
        if frame_count > 2 {
            // sync the game time and the input handler time
            if time_offset == 0.0 {
                time_offset = time - input.elapsed().as_secs_f64();
            }
            time = input.elapsed().as_secs_f64() + time_offset;
        }

        // start the song when time >= 0
        if time >= 0.0 && !audio_playing {
            audio_handle.resume(Tween {
                duration: Duration::from_millis(1),
                ..Default::default()
            });
            audio_playing = true;
        }

        frame_count += 1;

        next_frame().await;
    }
}

fn handle_inputs(
    input: &mut InputManager,
    strikeline: &mut Strikeline,
    time: f64,
    notes: &mut VecDeque<NoteContainer>
) {
    // update fret hit animation
    for fret in &mut strikeline.frets {
        if fret.height > 0.0 {
            fret.height -= get_frame_time() * 10.0;
        }
        fret.height = fret.height.max(0.0);
    }

    input.update(strikeline, notes, time);

    if !notes.is_empty() && notes[notes.len()-1].t > NEAR_T {
        notes.pop_back();
    }
}

async fn load_assets(folder: &'static str) -> Assets {
    let frets = [
        load_fret_assets(folder, 0).await,
        load_fret_assets(folder, 1).await,
        load_fret_assets(folder, 2).await,
    ];

    Assets {
        notes: NoteAssets {
            note: load_texture(&format!("{folder}/notes/note.png")).await.unwrap(),
            hopo: load_texture(&format!("{folder}/notes/hopo.png")).await.unwrap(),
            tap: load_texture(&format!("{folder}/notes/tap.png")).await.unwrap(),
            wor_tap: load_texture(&format!("{folder}/notes/wor_tap.png")).await.unwrap(),
            open: load_texture(&format!("{folder}/notes/open.png")).await.unwrap(),
            open_hopo: load_texture(&format!("{folder}/notes/open_hopo.png")).await.unwrap(),
        },
        frets,
        fret_piston: load_texture(&format!("{folder}/frets/piston.png")).await.unwrap(),
    }
}

async fn load_fret_assets(folder: &'static str, fret: usize) -> FretAssets {
    FretAssets {
        fret: load_texture(&format!("{folder}/frets/{fret}_fret.png")).await.unwrap(),
        fret_pressed: load_texture(&format!("{folder}/frets/{fret}_fret_pressed.png")).await.unwrap(),
        pressed: load_texture(&format!("{folder}/frets/{fret}_pressed.png")).await.unwrap(),
        ring: load_texture(&format!("{folder}/frets/{fret}_ring.png")).await.unwrap(),
        shell: load_texture(&format!("{folder}/frets/{fret}_shell.png")).await.unwrap(),
    }
}
