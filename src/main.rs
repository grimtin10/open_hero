mod controllers;
mod song;

use std::{thread, time::{Duration, Instant}};

use gilrs::Button;
use kira::{sound::streaming::StreamingSoundData, AudioManager, AudioManagerSettings, DefaultBackend, Tween};
use macroquad::{audio, prelude::*, telemetry::frame};

use crate::{controllers::{ControllerEventType, ControllerManager}, song::{Difficulty, Instrument, Note}};

// haha it says fart
const FAR_T: f32 = 0.0;
const NEAR_T: f32 = 1.5;
const FADE_T: f32 = 0.1;

// clone hero hit window (for now)
const HIT_FRONT: f32 = 0.07;
const HIT_BACK:  f32 = 0.07;

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

    pub frets: Vec<FretAssets>,
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
    println!("Loading assets took {}ms", start.elapsed().as_millis());

    // TODO: keyboard support/proper InputManager system
    let mut controllers = ControllerManager::new().unwrap();
    let mut strikeline = Strikeline::default();

    let song_name = "Star";
    let song = song::parse(format!("songs/{song_name}/notes.chart")).unwrap();
    let chart = song.charts.get(&(Instrument::Single, Difficulty::Expert)).unwrap();
    let mut notes: Vec<NoteContainer> = chart.notes.iter().map(|note| NoteContainer { note: *note, t: 0.0 }).collect();
    notes.reverse();

    let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap();
    let audio = StreamingSoundData::from_file(format!("songs/{song_name}/song.ogg")).unwrap().volume(-12.0);

    let mut audio_playing = false;
    let mut audio_handle = manager.play(audio).unwrap();
    audio_handle.pause(Tween {
        duration: Duration::from_millis(1),
        ..Default::default()
    });
    audio_handle.seek_to(0.0);

    let mut time = -5.0;
    let mut time_offset = 0.0; // offset between input system time and game time
    let mut frame_count = 0;
    loop {
        clear_background(BLACK);

        handle_inputs(&mut controllers, &mut strikeline, time, &mut notes);

        // // highway background
        // draw_polygon(&[
        //     vec2(t_to_x(2.0, -0.5), t_to_y(NEAR_T)),
        //     vec2(t_to_x(2.0, 4.5), t_to_y(NEAR_T)),
        //     vec2(t_to_x(FAR_T, 4.5), t_to_y(FAR_T)),
        //     vec2(t_to_x(FAR_T, -0.5), t_to_y(FAR_T)),
        // ], BLACK);

        // hit window
        let hit_start = perspective(time_to_t(HIT_FRONT));
        let hit_end = perspective(time_to_t(-HIT_BACK));
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

        // get notes on screen
        let mut render_end = notes.len() - 1;
        let mut render_start = notes.len() - 1;
        let mut i = notes.len() - 1;
        loop {
            let note = &mut notes[i];
            note.t = time_to_t(note.note.time as f32 - time);
            if note.t > NEAR_T { render_end = i; }
            if note.t < FAR_T { render_start = i; break; }

            if i == 0 { break; }
            i -= 1;
        }

        // render notes
        let mut i = render_start;
        while i <= render_end {
            let note = &notes[i];
            render_note(&assets, &note.note, note.t);
            i += 1;
        }

        draw_fps();

        // skip the first couple frames because of large frame times
        if frame_count > 2 {
            // sync the game time and the input handler time
            if time_offset == 0.0 {
                time_offset = time - controllers.start.elapsed().as_secs_f32();
            }
            time = controllers.start.elapsed().as_secs_f32() + time_offset;
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

fn handle_inputs(controllers: &mut ControllerManager, strikeline: &mut Strikeline, time: f32, notes: &mut Vec<NoteContainer>) {
    // get offset from input handler time to game time
    let input_time = controllers.start.elapsed().as_secs_f32();
    let time_offset = time - input_time;
    
    // update fret hit animation
    for fret in &mut strikeline.frets {
        if fret.height > 0.0 {
            fret.height -= get_frame_time() * 10.0;
        }
        fret.height = fret.height.max(0.0);
    }

    let mut hits = 0;
    // loop over every event and check for note hit
    for event in controllers.drain_events() {
        // convert microseconds to seconds then to game time
        let event_time = event.timestamp as f32 / 1_000_000.0 + time_offset;

        // bitmasks are fun
        match event.event {
            ControllerEventType::ButtonPressed(button) => {
                match button {
                    Button::West =>        strikeline.pressed |= 1 << 0,
                    Button::South =>       strikeline.pressed |= 1 << 1,
                    Button::North =>       strikeline.pressed |= 1 << 2,
                    Button::East =>        strikeline.pressed |= 1 << 3,
                    Button::LeftTrigger => strikeline.pressed |= 1 << 4,
                    _ => {}
                }
            }
            ControllerEventType::ButtonReleased(button) => {
                match button {
                    Button::West =>        strikeline.pressed &= 255 ^ (1 << 0),
                    Button::South =>       strikeline.pressed &= 255 ^ (1 << 1),
                    Button::North =>       strikeline.pressed &= 255 ^ (1 << 2),
                    Button::East =>        strikeline.pressed &= 255 ^ (1 << 3),
                    Button::LeftTrigger => strikeline.pressed &= 255 ^ (1 << 4),
                    _ => {}
                }
            }
            _ => {}
        }

        // check for hits
        let mut i = notes.len() - 1;
        while i > 0 {
            let note = &notes[i];
            let time = note.note.time as f32 - event_time;

            if time < HIT_FRONT && time > -HIT_BACK {
                if note.note.frets_masked == strikeline.pressed {
                    for i in 0..5 {
                        if note.note.frets_masked >> i & 1 == 1 || note.note.frets >> 7 & 1 == 1 {
                            strikeline.frets[i].height = 1.0;
                        }
                    }
                    notes.remove(i);
                    hits += 1;
                }
                break;
            }

            if time > HIT_FRONT { break; }

            i -= 1;
        }
    }
    if hits > 0 { println!("hit {hits} note this frame"); }

    if notes[notes.len()-1].t > NEAR_T {
        notes.remove(notes.len()-1);
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
            shell: load_texture(&format!("{folder}/frets/{i}_shell.png")).await.unwrap(),
        });
    }

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

fn render_fret(assets: &Assets, fret: usize, state: FretState, pressed: bool) {
    let textures = if fret == 0 || fret == 4 {
        &assets.frets[0]
    } else if fret == 1 || fret == 3 {
        &assets.frets[1]
    } else {
        &assets.frets[2]
    };

    let flip = fret > 2;

    let x = t_to_x(1.0, fret as f32);
    let y = t_to_y(1.0);

    if state.height <= 0.0 && pressed {
        render_texture(&textures.pressed, x, y, 0.8, flip, 1.0);
    } else {
        render_texture(&textures.shell, x, y, 0.8, flip, 1.0);
        render_texture(&assets.fret_piston, x, y + 8.0, 0.8, false, 1.0);
        render_texture(if pressed { &textures.fret_pressed } else { &textures.fret }, x, y - lerp(0.0, 20.0, state.height), 0.8, flip, 1.0);
        render_texture(&textures.ring, x, y, 0.8, flip, 1.0);
    }
}

fn render_texture(texture: &Texture2D, x: f32, y: f32, scale: f32, flip_x: bool, alpha: f32) {
    let width = texture.width() * scale;
    let height = texture.height() * scale;

    draw_texture_ex(texture, x - width / 2.0, y - height / 2.0, Color::new(1.0, 1.0, 1.0, alpha), DrawTextureParams {
        dest_size: Some(vec2(width, height)),
        flip_x,
        ..Default::default()
    });
}

fn time_to_t(time: f32) -> f32 {
    return 1.0 - time;
}

fn render_note(assets: &Assets, note: &Note, t: f32) {
    if note.frets >> 7 & 1 == 1 {
        if note.is_hopo || note.frets >> 6 & 1 == 1 {
            render_gem(&assets.notes.open_hopo, 6, t);
        } else {
            render_gem(&assets.notes.open, 6, t);
        }
    }
    for i in 0..5 {
        if note.frets >> i & 1 == 1 {
            render_gem(if note.frets >> 6 & 1 == 1 {
                &assets.notes.tap
            } else if note.is_hopo {
                &assets.notes.hopo
            } else {
                &assets.notes.note
            }, i, t);
        }
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 { a + t * (b - a) }

/// Perspective corrects `t` values
const PERSPECTIVE: f32 = 3.0;
fn perspective(t: f32) -> f32 { t / (PERSPECTIVE - (PERSPECTIVE - 1.0) * t) }

/// Expects perspective corrected `t` value
fn t_to_x(t: f32, fret: f32) -> f32 {
    // constants taken from GH3
    let start_x = 576.0 + 32.0 * fret;
    let end_x = 435.2 + 101.4 * fret;
    lerp(start_x, end_x, t)
}
/// Expects perspective corrected `t` value
fn t_to_y(t: f32) -> f32 { lerp(305.0, 655.0, t) }
/// Expects perspective corrected `t` value
fn t_to_scale(t: f32) -> f32 { lerp(0.4, 1.2, t) }

fn render_gem(texture: &Texture2D, fret: usize, t: f32) {
    let alpha = ((t - FAR_T) / FADE_T).min(1.0);
    let t = perspective(t);
    let scale = if fret == 6 { 0.9 } else { 0.629 };
    let fret = if fret == 6 { 2 } else { fret };
    render_texture(
        texture,
        t_to_x(t, fret as f32),
        t_to_y(t),
        t_to_scale(t) * scale,
        false,
        alpha
    );
}

// TODO: textured polygon rendering
//       this will require writing a custom shader

fn triangulate_polygon(vertices: &[Vec2]) -> Vec<[usize; 3]> {
    let mut tris = Vec::new();

    if vertices.len() < 3 {
        return tris;
    }

    // TODO: more complex triangulation for non-convex polygons
    for i in 1..vertices.len() - 1 {
        tris.push([0, i, i + 1]);
    }

    tris
}

fn draw_polygon(vertices: &[Vec2], color: Color) {
    let tris = triangulate_polygon(vertices);

    for tri in tris {
        draw_triangle(vertices[tri[0]], vertices[tri[1]], vertices[tri[2]], color);
    }
}
