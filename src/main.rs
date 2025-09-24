mod controllers;
mod song;

use std::time::Instant;

use gilrs::Button;
use kira::{sound::streaming::StreamingSoundData, AudioManager, AudioManagerSettings, DefaultBackend};
use macroquad::{audio, prelude::*};

use crate::{controllers::{ControllerEventType, ControllerManager}, song::{Difficulty, Instrument, Note}};

const START_X: [f32; 5] = [576.0, 608.0, 640.0, 672.0, 704.0];
const END_X: [f32; 5]   = [435.2, 536.6, 640.0, 742.4, 844.8];

struct FretAssets {
    pub fret: Texture2D,
    pub fret_pressed: Texture2D,
    pub pressed: Texture2D,
    pub ring: Texture2D,
    pub shell: Texture2D,
}

struct Assets {
    pub note: Texture2D,

    pub frets: Vec<FretAssets>,
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Open Hero".into(),
        window_width: 1280,
        window_height: 720,
        window_resizable: false,

        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let start = Instant::now();
    let assets = load_assets("assets").await;
    println!("Loading assets took {}ms", (Instant::now()-start).as_millis());

    let mut controllers = ControllerManager::new().unwrap();
    let mut pressed: [bool; 5] = [false, false, false, false, false];

    let song = song::parse("songs/Star/notes.chart".into()).unwrap();
    let chart = song.charts.get(&(Instrument::Single, Difficulty::Expert)).unwrap();

    let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap();
    let audio = StreamingSoundData::from_file("songs/Star/song.ogg").unwrap();

    let audio_handle = manager.play(audio).unwrap();

    let mut time = 0.0;
    loop {
        for event in controllers.drain_events() {
            match event.event {
                ControllerEventType::ButtonPressed(button) => {
                    match button {
                        Button::West =>        pressed[0] = true,
                        Button::South =>       pressed[1] = true,
                        Button::North =>       pressed[2] = true,
                        Button::East =>        pressed[3] = true,
                        Button::LeftTrigger => pressed[4] = true,
                        _ => {}
                    }
                }
                ControllerEventType::ButtonReleased(button) => {
                    match button {
                        Button::West =>        pressed[0] = false,
                        Button::South =>       pressed[1] = false,
                        Button::North =>       pressed[2] = false,
                        Button::East =>        pressed[3] = false,
                        Button::LeftTrigger => pressed[4] = false,
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        clear_background(BLACK);

        for i in 0..5 {
            render_fret(&assets, i, pressed[i]);
        }

        for note in chart.notes.iter().rev() {
            render_note(&assets, &note, time);
        }

        draw_fps();
        // time += get_frame_time();
        time = audio_handle.position() as f32;

        next_frame().await;
    }
}

async fn load_assets(folder: &'static str) -> Assets {
    let mut frets = Vec::new();

    for i in 0..3 {
        frets.push(FretAssets {
            fret: load_texture(&format!("{folder}/frets/{i}_fret.png")).await.unwrap(),
            fret_pressed: load_texture(&format!("{folder}/frets/{i}_fret_pressed.png")).await.unwrap(),
            pressed: load_texture(&format!("{folder}/frets/{i}_pressed.png")).await.unwrap(),
            ring: load_texture(&format!("{folder}/frets/{i}_ring.png")).await.unwrap(),
            shell: load_texture(&format!("{folder}/frets/{i}_shell.png")).await.unwrap()
        });
    }

    Assets {
        note: load_texture(&format!("{folder}/notes/note.png")).await.unwrap(),
        frets
    }
}

fn render_fret(assets: &Assets, fret: usize, pressed: bool) {
    let textures = if fret == 0 || fret == 4 {
        &assets.frets[0]
    } else if fret == 1 || fret == 3 {
        &assets.frets[1]
    } else {
        &assets.frets[2]
    };

    let flip = fret > 2;

    let x = END_X[fret];
    let y = 655.0;

    if pressed {
        render_texture(&textures.pressed, x, y, 0.8, flip);
    } else {
        render_texture(&textures.shell, x, y, 0.8, flip);
        render_texture(&textures.fret, x, y, 0.8, flip);
        render_texture(&textures.ring, x, y, 0.8, flip);
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 { a + t * (b - a) }

fn render_texture(texture: &Texture2D, x: f32, y: f32, scale: f32, flip_x: bool) {
    let width = texture.width() * scale;
    let height = texture.height() * scale;

    draw_texture_ex(texture, x - width / 2.0, y - height / 2.0, WHITE, DrawTextureParams {
        dest_size: Some(vec2(width, height)),
        flip_x,
        ..Default::default() 
    });
}

fn render_note(assets: &Assets, note: &Note, time: f32) {
    let t = 1.0 - (note.time as f32 - time);
    if t < 0.0 || t > 1.5 { return }
    for i in 0..5 {
        if note.frets >> i & 1 == 1 {
            render_gem(assets, i, t);
        }
    }
}

fn render_gem(assets: &Assets, fret: usize, t: f32) {
    let perspective_factor = 3.0; // found through the power of bullshit (but it's actually perfect)
    let t_p = t / (perspective_factor - (perspective_factor - 1.0) * t);
    let x = lerp(START_X[fret], END_X[fret], t_p);
    let y = lerp(305.0, 655.0, t_p);
    let scale = lerp(0.4, 1.2, t_p) * 0.629;

    render_texture(&assets.note, x, y, scale, false);
}
