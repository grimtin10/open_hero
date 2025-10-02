use macroquad::prelude::*;

use crate::{config::Config, song::Note, Assets, FretState, FADE_T, FAR_T};

pub fn render_fret(assets: &Assets, fret: usize, state: FretState, pressed: bool) {
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

pub fn render_texture(texture: &Texture2D, x: f32, y: f32, scale: f32, flip_x: bool, alpha: f32) {
    let width = texture.width() * scale;
    let height = texture.height() * scale;

    draw_texture_ex(texture, x - width / 2.0, y - height / 2.0, Color::new(1.0, 1.0, 1.0, alpha), DrawTextureParams {
        dest_size: Some(vec2(width, height)),
        flip_x,
        ..Default::default()
    });
}

/// This function converts a time value (0 being at the strikeline, in seconds) to a position on the highway
/// `notespeed` is in CH notespeed
pub fn time_to_t(time: f32, notespeed: f32) -> f32 { 1.0 - (time * notespeed) / 7.87 }

pub fn render_note(assets: &Assets, config: &Config, note: &Note, t: f32) {
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
                if config.wor_tap {
                    &assets.notes.wor_tap
                } else {
                    &assets.notes.tap
                }
            } else if note.is_hopo {
                &assets.notes.hopo
            } else {
                &assets.notes.note
            }, i, t);
        }
    }
}

/// Linear interpolation from `a` to `b` with `t`
pub fn lerp(a: f32, b: f32, t: f32) -> f32 { a + t * (b - a) }

/// Perspective corrects `t` values
const PERSPECTIVE: f32 = 3.0;
pub fn perspective(t: f32) -> f32 { t / (PERSPECTIVE - (PERSPECTIVE - 1.0) * t) }

/// Expects perspective corrected `t` value
pub fn t_to_x(t: f32, fret: f32) -> f32 {
    // constants taken from GH3
    let start_x = 576.0 + 32.0 * fret;
    let end_x = 435.2 + 101.4 * fret;
    lerp(start_x, end_x, t)
}
/// Expects perspective corrected `t` value
pub fn t_to_y(t: f32) -> f32 { lerp(305.0, 655.0, t) }
/// Expects perspective corrected `t` value
pub fn t_to_scale(t: f32) -> f32 { lerp(0.4, 1.2, t) }

pub fn render_gem(texture: &Texture2D, fret: usize, t: f32) {
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

pub fn triangulate_polygon(vertices: &[Vec2]) -> Vec<[usize; 3]> {
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

pub fn draw_polygon(vertices: &[Vec2], color: Color) {
    let tris = triangulate_polygon(vertices);

    for tri in tris {
        draw_triangle(vertices[tri[0]], vertices[tri[1]], vertices[tri[2]], color);
    }
}
