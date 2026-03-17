use std::time::Duration;

use mash::{DeviceKind, InputEvent, InputKind, InputThread, Receiver};

use crate::{HIT_BACK, HIT_FRONT, NoteContainer, Strikeline};

/// Gets the value of the first set bit from the right (least significant bit)
#[inline]
fn lsb(x: u8) -> u8 { x & x.wrapping_neg() }

/// Gets the value of the first set bit from the left (most significant bit)
#[inline]
fn msb(x: u8) -> u8 { if x == 0 { 0 } else { 1 << x.ilog2() } }

#[inline]
fn sec_to_ns(t: f64) -> i128 { (t * 1_000_000_000.0) as i128 }

#[allow(dead_code)]
#[inline]
fn ns_to_sec(t: i128) -> f64 { t as f64 / 1_000_000_000.0 }

pub struct InputManager {
    main_device: Option<u32>,

    thread: InputThread,
    rx: Receiver<InputEvent>,

    bot: bool,

    pending_strum: bool,
    strum_time: u128,
    pending_frets: u8,
}

impl InputManager {
    pub fn new(bot: bool) -> Self {
        let (thread, rx) = InputThread::spawn();
        Self {
            main_device: None,

            thread,
            rx,

            bot,

            pending_strum: false,
            strum_time: 0,
            pending_frets: 0,
        }
    }

    pub fn update(&mut self, strikeline: &mut Strikeline, notes: &mut Vec<NoteContainer>, time: f64) {
        if self.bot {
            while !notes.is_empty() && notes[notes.len()-1].t >= 1.0 {
                let note = notes.remove(notes.len() - 1).note;
                strikeline.pressed = note.frets_masked;
                for i in 0..5 {
                    if note.frets_masked >> i & 1 == 1 || note.frets >> 7 & 1 == 1 {
                        strikeline.frets[i].height = 1.0;
                    }
                }
            }
            return;
        }

        let mut inputs = Vec::new();
        for event in self.rx.try_iter() {
            match event {
                InputEvent::Connected(info) => {
                    // take the first connected gamepad and assume it's the main one
                    if info.kind == DeviceKind::Gamepad && self.main_device.is_none() {
                        println!("connected gamepad {info:?}");
                        self.main_device = Some(info.id);
                    }
                }
                InputEvent::Input { device, timestamp, kind } => {
                    if let Some(main_device) = self.main_device && main_device == device {} else { continue; }
                    inputs.push((timestamp, kind));
                }
                _ => ()
            }
        }

        // me when borrow checker
        for (timestamp, kind) in inputs {
            self.handle_input(timestamp, kind, strikeline, notes, time);
        }
    }

    fn handle_input(&mut self, timestamp: u128, kind: InputKind, strikeline: &mut Strikeline, notes: &mut Vec<NoteContainer>, time: f64) {
        let time_offset = sec_to_ns(time) - timestamp as i128;

        match kind {
            InputKind::Button { code, pressed } => {
                let bit = match code {
                    304 => 1 << 0,
                    305 => 1 << 1,
                    308 => 1 << 2,
                    307 => 1 << 3,
                    310 => 1 << 4,
                    _ => 0,
                };
                if pressed { strikeline.pressed |= bit; self.pending_frets |= bit; } else { strikeline.pressed &= !bit; self.pending_frets &= !bit; }
            },
            InputKind::Axis { value, relative, .. } => {
                if !relative && value != 0 {
                    self.pending_strum = true;
                    self.strum_time = timestamp;
                }
            }
        }

        // TODO: make this not shit lol
        for i in (0..notes.len()).rev() {
            let note = &notes[i].note;
            let time = sec_to_ns(note.time) - timestamp as i128 - time_offset;
            if time < sec_to_ns(HIT_FRONT) && time > -sec_to_ns(HIT_BACK) {
                let tappable = note.is_hopo || note.frets >> 6 & 1 == 1;

                // anchoring check
                let lowest_note = lsb(note.frets_masked);
                let highest_fret = msb(strikeline.pressed & !note.frets_masked);
                let anchoring = !note.is_chord || tappable && highest_fret < lowest_note;

                // shift out the "anchoring" frets, those being the ones below the lowest fret in the note
                let lowest_index = if lowest_note == 0 {
                    0
                } else {
                    lowest_note.ilog2()
                };
                let note_shifted = note.frets_masked >> lowest_index;
                let frets_shifted = strikeline.pressed >> lowest_index;

                // either you're anchoring it OR for a strum chord you're hitting the exact frets
                let fretting = anchoring && note_shifted == frets_shifted || note.frets_masked == strikeline.pressed;

                let tapping = self.pending_frets & note.frets_masked > 0 && tappable;
                let should_hit = fretting && (tapping || self.pending_strum);

                if should_hit {
                    self.pending_strum = false;
                    self.pending_frets &= !note.frets_masked;

                    // hit animation
                    for i in 0..5 {
                        if note.frets_masked >> i & 1 == 1 || note.frets >> 7 & 1 == 1 {
                            strikeline.frets[i].height = 1.0;
                        }
                    }
                    notes.remove(notes.len() - 1);
                }
                break;
            }

            if time > sec_to_ns(HIT_FRONT) { break; }
        }
    }

    // pub fn duration_since(&self, earlier: SystemTime) -> Duration {
    //     self.thread.start.duration_since(earlier).unwrap()
    // }

    pub fn elapsed(&self) -> Duration {
        self.thread.elapsed()
    }
}
