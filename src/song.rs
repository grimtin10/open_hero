use std::{collections::HashMap, error::Error, fs};

// TODO: in the far future this should be rewritten as a serde serializer/deserializer
//       this would allow for super easy chart editing and saving and honestly it would just be cool

#[derive(Debug, Clone, Default)]
pub struct SongSection {
    pub name: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub year: Option<String>,
    pub charter: Option<String>,
    pub resolution: Option<usize>,
    pub difficulty: Option<usize>,
    pub length: Option<f32>,
    pub offset: Option<f32>,
    pub preview_start: Option<f32>,
    pub preview_end: Option<f32>,
}

#[derive(Debug)]
pub struct TempoEvent {
    pub bpm: f32,

    pub time: f64, // in seconds
}

#[derive(Debug)]
pub enum SyncEvent {
    TimeSignature(usize, usize),
    Tempo(TempoEvent),
}

#[derive(Debug)]
pub enum GlobalEvent {
    Section(String),
    PhraseStart,
    Lyric(String),
    PhraseEnd,
    SongEnd,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum Instrument {
    Single,
    DoubleGuitar,
    DoubleBass,
    DoubleRhythm,
    Unknown,
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
    Expert,
}

// currently only starpower is supported, but more may be in the future
#[derive(Debug)]
pub struct StarpowerEvent {
    pub tick: usize,
    pub length: usize,
}

#[derive(Debug)]
pub enum LocalEvent {
    SoloStart,
    SoloEnd,
}

#[derive(Debug)]
pub struct Note {
    pub tick: usize,
    pub frets: u8,
    pub length: [usize; 8],

    pub time: f64, // in seconds
}

#[derive(Debug)]
pub struct Chart {
    pub notes: Vec<Note>,
    pub starpower_events: Vec<StarpowerEvent>,
}

#[derive(Debug, Default)]
pub struct Song {
    pub metadata: Option<SongSection>,
    pub sync_track: Option<Vec<(usize, SyncEvent)>>,
    pub events: Option<Vec<(usize, GlobalEvent)>>,

    pub charts: HashMap<(Instrument, Difficulty), Chart>
}

pub fn parse(file: String) -> Result<Song, Box<dyn Error>> {
    let file = String::from_utf8(fs::read(file)?)?;
    let file: String = file.trim_start_matches("\u{FEFF}").into(); // strip BOM

    let mut song = Song::default();

    let lines: Vec<String> = file.split("\n").map(|s| s.to_string()).collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        if line.starts_with("[") {
            i += 1;
            if lines[i].trim() == "{" { i += 1; }

            // look i know the function is called `remove_quotes` but just trust me
            let section_type = remove_quotes(line.into()).to_lowercase();
            match section_type.as_str() {
                "song"      => song.metadata = Some(parse_song(&lines, &mut i)),
                "synctrack" => song.sync_track = Some(parse_sync(&lines, &mut i)),
                "events"    => song.events = Some(parse_events(&lines, &mut i)),
                _ => {
                    if ["expert", "hard", "medium", "easy"].iter().any(|s| section_type.starts_with(s)) {
                        let chart = parse_chart(&lines, &mut i, section_type);
                        if song.charts.insert((chart.0, chart.1), chart.2).is_some() {
                            println!("Chart contains duplicate note data for `{line}`!");
                        }
                    } else {
                        println!("Unhandled section type `{line}`")
                    }
                }
            }
        }
        i += 1;
    }

    // TODO: better error handling
    let mut time = 0.0;
    let mut last_tick = 0;
    let mut last_bpm = 120.0;
    let resolution = song.metadata.as_ref().expect("Chart had no Song metadata!").resolution.expect("Chart had no resolution!");
    let mut bpm_events = Vec::new(); // this is just to make it 0.1% faster
    for event in song.sync_track.as_mut().expect("Chart had no SyncTrack!") {
        let tick = event.0;
        match &mut event.1 {
            SyncEvent::Tempo(tempo) => {
                time += ticks_to_seconds(tick - last_tick, last_bpm, resolution);
                tempo.time = time;
                last_tick = tick;
                last_bpm = tempo.bpm;
                bpm_events.push((tick, time, tempo.bpm));
            }
            _ => {}
        }
    }

    for (_, chart) in &mut song.charts {
        calculate_note_times(chart, &bpm_events, resolution);
    }

    Ok(song)
}

fn calculate_note_times(chart: &mut Chart, bpm_events: &Vec<(usize, f64, f32)>, resolution: usize) {
    let mut last_bpm = 0;
    for note in &mut chart.notes {
        while last_bpm + 1 < bpm_events.len() && bpm_events[last_bpm + 1].0 <= note.tick {
            last_bpm += 1;
        }
        
        let bpm = &bpm_events[last_bpm];
        note.time = bpm.1 + ticks_to_seconds(note.tick - bpm.0, bpm.2, resolution);
    }
}

fn ticks_to_seconds(ticks: usize, bpm: f32, resolution: usize) -> f64 {
    ticks as f64 / resolution as f64 * (60.0 / bpm as f64)
}

fn parse_song(lines: &Vec<String>, i: &mut usize) -> SongSection {
    let mut res = SongSection::default();

    loop {
        let line = lines[*i].trim();
        if line == "}" { break; }
        
        let split: Vec<&str> = line.split(" = ").collect();
        match split[0].to_lowercase().as_str() {
            "name"         => res.name = Some(remove_quotes(split[1].into())),
            "artist"       => res.artist = Some(remove_quotes(split[1].into())),
            "album"        => res.album = Some(remove_quotes(split[1].into())),
            "genre"        => res.genre = Some(remove_quotes(split[1].into())),
            "year"         => res.year = Some(remove_quotes(split[1].into()).trim_start_matches(", ").into()),
            "charter"      => res.charter = Some(remove_quotes(split[1].into())),
            "resolution"   => res.resolution = Some(split[1].parse().unwrap()),
            "difficulty"   => res.difficulty = Some(split[1].parse().unwrap()),
            "length"       => res.length = Some(split[1].parse().unwrap()),
            "offset"       => res.offset = Some(split[1].parse().unwrap()),
            "previewstart" => res.preview_start = Some(split[1].parse().unwrap()),
            "previewend"   => res.preview_end = Some(split[1].parse().unwrap()),
            _ => println!("Unknown song metadata `{}` with value `{}`", split[0], split[1]),
        }

        *i += 1;
    }

    res
}

fn parse_sync(lines: &Vec<String>, i: &mut usize) -> Vec<(usize, SyncEvent)> {
    let mut res = Vec::new();

    loop {
        let line = lines[*i].trim();
        if line == "}" { break; }
        
        let split: Vec<&str> = line.split(" ").collect();
        let tick = split[0].parse().unwrap();
        match split[2].to_lowercase().as_str() {
            "ts" => res.push((
                tick,
                SyncEvent::TimeSignature(
                    split[3].parse().unwrap(),
                    if let Some(denom) = split.get(4) {
                        2usize.pow(denom.parse().unwrap())
                    } else {
                        4
                    }
                )
            )),
            "b" => res.push((tick, SyncEvent::Tempo(TempoEvent { bpm: split[3].parse::<usize>().unwrap() as f32 / 1000.0, time: 0.0 }))),
            _ => {}
        }

        *i += 1;
    }

    res
}

fn parse_events(lines: &Vec<String>, i: &mut usize) -> Vec<(usize, GlobalEvent)> {
    let mut res = Vec::new();

    loop {
        let line = lines[*i].trim();
        if line == "}" { break; }
        
        let split: Vec<&str> = line.split(" = ").collect();
        let tick: usize = split[0].parse().unwrap();

        // variable naming be damned (i'm tired okay)
        let val = remove_chars(split[1].into(), 3, 1);
        let val: Vec<&str> = val.split([' ', '_']).collect();

        let event_type = val[0].to_lowercase();
        let val = val[1..].join(" ");

        match event_type.as_str() {
            "section"      => res.push((tick, GlobalEvent::Section(val))),
            "phrase_start" => res.push((tick, GlobalEvent::PhraseStart)),
            "lyric"        => res.push((tick, GlobalEvent::Lyric(val))),
            "phrase_end"   => res.push((tick, GlobalEvent::PhraseEnd)),
            "end"          => res.push((tick, GlobalEvent::SongEnd)),
            _ => {}
        }

        *i += 1;
    }

    res
}

fn parse_chart(lines: &Vec<String>, i: &mut usize, chart_type: String) -> (Instrument, Difficulty, Chart) {
    let difficulty = if chart_type.starts_with("easy") {
        Difficulty::Easy
    } else if chart_type.starts_with("medium") {
        Difficulty::Medium
    } else if chart_type.starts_with("hard") {
        Difficulty::Hard
    } else {
        Difficulty::Expert
    };

    let instrument = if chart_type.ends_with("single") {
        Instrument::Single
    } else if chart_type.ends_with("doubleguitar") {
        Instrument::DoubleGuitar
    } else if chart_type.ends_with("doublebass") {
        Instrument::DoubleBass
    } else if chart_type.ends_with("doublerhythm") {
        Instrument::DoubleRhythm
    } else {
        Instrument::Unknown
    };

    let mut last_tick = 0;
    let mut cur_frets = 0;
    let mut cur_length = [0; 8];
    let mut notes = Vec::new();
    let mut starpower_events = Vec::new();

    loop {
        let line = lines[*i].trim();
        if line == "}" { break; }

        let split: Vec<&str> = line.split(" = ").collect();
        let tick: usize = split[0].parse().unwrap();

        // variable naming be damned (i'm tired okay)
        let val = split[1].to_string();
        let val: Vec<&str> = val.split(' ').collect();

        let note_type = val[0].to_lowercase();

        match note_type.as_str() {
            "n" => {
                let fret: u8 = val[1].parse().unwrap();
                let length = val[2].parse().unwrap();
                cur_frets = cur_frets | (1 << fret);
                cur_length[fret as usize] = length;

                if last_tick != tick {
                    notes.push(Note {
                        tick,
                        frets: cur_frets,
                        length: cur_length,

                        // calculated later
                        time: 0.0
                    });

                    cur_frets = 0;
                    cur_length = [0; 8];
                }

                last_tick = tick;
            }
            _ => {}
        }

        *i += 1;
    }

    (instrument, difficulty, Chart {
        notes,
        starpower_events,
    })
}

fn remove_quotes(s: String) -> String {
    remove_chars(s, 1, 1)
}

// RUSTTTTTT
#[inline]
fn remove_chars(s: String, start: usize, end: usize) -> String {
    s.chars().skip(start).take(s.chars().count() - start - end).collect()
}
