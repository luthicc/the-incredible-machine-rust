//! Sound effects and music from TIM.SX, played through an emulated AdLib.
//!
//! TIM.SX is a SND: container: INF: lists (id, offset) pairs and each DAT:
//! holds u32 size, u16 id, a device byte, a flags byte (read_sound_bank_entry,
//! 3619:3c10: bit 0 music, bit 1 loop) and a method-3 compressed SCI1 sound
//! resource. Ids 1..20 are effects, 1001..1016 music. The resource lists a
//! track table per synth device; the AdLib (device 0) tracks are SCI MIDI
//! channel streams, or a 0xfe digital sample for ids 19 and 20.
//!
//! The AdLib driver (SX.OVL ADL:, "AdLib Music Synthesizer Card" 2.24) uses
//! patch resource 003: 128 melodic and 62 rhythm instruments of 28 bytes and
//! a 62-entry rhythm key map, its 48-step F-number table and two 64-entry
//! velocity curves.
//!
//! Game code calls the native entry points (play_object_sound, 1000:83ab;
//! select_sound_bank, 1000:8364; stop_sound_group, 1000:83ea). Without an
//! audio device they do nothing.
mod sound_effect;
pub use sound_effect::SoundEffect;
mod music_track;
pub use music_track::MusicTrack;
mod sound_group;
pub use sound_group::SoundGroup;
mod body;
use body::Body;
mod sound;
use sound::Sound;
mod operator;
use operator::Operator;
mod patch;
use patch::Patch;
mod driver;
use driver::Driver;
mod bank;
pub use bank::Bank;
mod channel;
use channel::Channel;
mod track;
use track::Track;
mod player;
use player::Player;
mod voice;
use voice::Voice;
mod synth;
use synth::Synth;
mod event;
use event::Event;
mod sample;
use sample::Sample;
mod command;
use command::Command;
mod engine;
use engine::Engine;
mod lowpass;
use lowpass::Lowpass;
mod output;
use output::Output;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, mpsc};

use crate::resources::Resources;

type Result<T> = std::result::Result<T, String>;

/// Offline rendering rate; live output runs at the device's own rate so no
/// resampler sits between the chip emulation and the speakers.
const OFFLINE_RATE: u32 = 44_100;
/// Overall output level. At the original's default volume 6 (master 15)
/// this leaves headroom for music, effects and samples together.
const OUTPUT_GAIN: f32 = 1.0 / 60_000.0;
const SAMPLE_GAIN: f32 = 0.25;
/// SCI sound ticks per second.
const TICK_RATE: u32 = 60;
const VOICES: usize = 9;
const MELODIC_PATCHES: usize = 128;
const PATCHES: usize = 190;
const RHYTHM_KEYS: usize = 62;
const FIRST_RHYTHM_KEY: u8 = 27;
/// Volume setting 0..6 (DS:4ec1) to the driver's master volume (DS:0116).
const MASTER_VOLUME: [u8; 7] = [0, 3, 5, 8, 10, 13, 15];

fn chunk<'a>(data: &'a [u8], tag: &[u8]) -> Result<&'a [u8]> {
    let mut list = Vec::new();
    crate::resources::chunks(data, &mut list)?;
    list.into_iter()
        .find(|(t, _)| *t == tag)
        .map(|(_, payload)| payload)
        .ok_or_else(|| format!("Missing {} chunk", String::from_utf8_lossy(tag)))
}

fn u16_at(data: &[u8], offset: usize) -> Result<u16> {
    data.get(offset..offset + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or_else(|| "Truncated sound data".to_string())
}

/// The SND: container's DAT: entries.
fn parse_bank(data: &[u8]) -> Result<HashMap<u16, Sound>> {
    let mut list = Vec::new();
    crate::resources::chunks(data, &mut list)?;
    let mut sounds = HashMap::new();
    for (_, entry) in list.into_iter().filter(|(tag, _)| *tag == b"DAT:") {
        let id = u16_at(entry, 0)?;
        let flags = *entry.get(3).ok_or("Truncated sound entry")?;
        let resource = crate::resources::decompress(&entry[4..])?;
        sounds.insert(
            id,
            Sound {
                music: flags & 1 != 0,
                looping: flags & 2 != 0,
                body: parse_resource(resource)?,
            },
        );
    }
    Ok(sounds)
}

/// SCI1 sound resource: two header bytes, then per device a device byte and
/// six-byte track entries (two flag bytes, u16 offset, u16 size) up to 0xff.
/// Offsets count from after the header. Only the AdLib (0) tracks are used.
fn parse_resource(data: Vec<u8>) -> Result<Body> {
    let mut i = 2;
    let mut tracks = Vec::new();
    loop {
        let device = *data.get(i).ok_or("Truncated device table")?;
        i += 1;
        if device == 0xff {
            break;
        }
        while *data.get(i).ok_or("Truncated track table")? != 0xff {
            let start = usize::from(u16_at(&data, i + 2)?) + 2;
            let size = usize::from(u16_at(&data, i + 4)?);
            if device == 0 {
                tracks.push((start, (start + size).min(data.len())));
            }
            i += 6;
        }
        i += 1;
    }
    // A 0xfe channel byte marks a sample: rate, size, loop start, loop end.
    if let Some(&(start, end)) = tracks.first() {
        if data.get(start) == Some(&0xfe) {
            let rate = u32::from(u16_at(&data, start + 2)?);
            let size = usize::from(u16_at(&data, start + 4)?);
            let first = start + 10;
            let samples = data
                .get(first..(first + size).min(end))
                .ok_or("Truncated sample")?;
            return Ok(Body::Digital {
                rate,
                samples: samples.into(),
            });
        }
    }
    Ok(Body::Midi {
        data: data.into(),
        tracks,
    })
}

/// SX.OVL's ADL: driver tables and the 003: patch resource.
fn parse_driver(overlay: &[u8]) -> Result<Driver> {
    let adlib = crate::resources::decompress(chunk(overlay, b"ADL:")?)?;
    if !adlib.windows(5).any(|w| w == b"AdLib") {
        return Err("SX.OVL ADL: is not the AdLib driver".into());
    }
    let table = |offset: usize, len: usize| {
        adlib
            .get(offset..offset + len)
            .ok_or("Truncated AdLib driver")
    };
    let mut frequencies = [0u16; 48];
    for (i, f) in frequencies.iter_mut().enumerate() {
        *f = u16_at(&adlib, 0x3d + i * 2)?;
    }
    let velocity_curve: [u8; 64] = table(0x9d, 64)?.try_into().unwrap();
    let volume_curve: [u8; 64] = table(0xdd, 64)?.try_into().unwrap();
    if frequencies[0] != 0x157 || velocity_curve[1] != 0x0c || volume_curve[1] != 0x14 {
        return Err("Unexpected AdLib driver tables".into());
    }
    let patch = crate::resources::decompress(chunk(overlay, b"003:")?)?;
    let patch = patch.get(2..).ok_or("Truncated patch resource")?;
    if patch.len() != PATCHES * 28 + RHYTHM_KEYS {
        return Err(format!("Unsupported AdLib patch size {}", patch.len()));
    }
    let operator = |b: &[u8], waveform: u8| Operator {
        ksl: b[0] & 3,
        mult: b[1] & 0xf,
        feedback: b[2] & 7,
        attack: b[3] & 0xf,
        sustain_level: b[4] & 0xf,
        sustaining: b[5] != 0,
        decay: b[6] & 0xf,
        release: b[7] & 0xf,
        total_level: b[8] & 0x3f,
        am: b[9] != 0,
        vibrato: b[10] != 0,
        ksr: b[11] != 0,
        additive: b[12] == 0,
        waveform: waveform & 3,
    };
    let patches = patch
        .chunks_exact(28)
        .take(PATCHES)
        .map(|p| Patch {
            modulator: operator(&p[0..13], p[26]),
            carrier: operator(&p[13..26], p[27]),
        })
        .collect();
    Ok(Driver {
        patches,
        rhythm_keys: patch[PATCHES * 28..].try_into().unwrap(),
        frequencies,
        velocity_curve,
        volume_curve,
    })
}

// ---------------------------------------------------------------- playback

const OPERATORS: [u8; VOICES] = [0, 1, 2, 8, 9, 10, 16, 17, 18];

fn read_delta(data: &[u8], position: &mut usize) -> u32 {
    // SCI delta times: each 0xf8 adds 240 ticks to the byte that follows.
    let mut wait = 0;
    while let Some(&b) = data.get(*position) {
        *position += 1;
        if b != 0xf8 {
            return wait + u32::from(b);
        }
        wait += 240;
    }
    wait
}

/// Run one tick of a sequence, collecting its MIDI events. With `silent`,
/// note-ons are dropped (fast-forwarding to a loop point).
fn step_player(player: &mut Player, events: &mut Vec<Event>, silent: bool) {
    let data = player.data.clone();
    let tick = player.tick;
    for track in &mut player.tracks {
        while !track.done && track.next <= tick {
            match read_event(&data, track) {
                None => track.done = true,
                // Program change 0x7f on the control channel marks the loop point.
                Some(Some(Event::Program(15, 0x7f))) => player.loop_tick = Some(tick),
                Some(Some(Event::NoteOn(..))) if silent => {}
                Some(Some(event)) => events.push(event),
                Some(None) => {}
            }
            if !track.done {
                track.next += read_delta(&data, &mut track.position);
            }
        }
    }
    player.tick += 1;
}

/// The next event of a track: Some(None) for one with nothing to play, None
/// at its end. Running status is kept per track.
fn read_event(data: &[u8], track: &mut Track) -> Option<Option<Event>> {
    if track.position >= track.end {
        return None;
    }
    let mut byte = *data.get(track.position)?;
    if byte >= 0x80 {
        track.position += 1;
        match byte {
            0xfc => return None,
            0xf0 => {
                while *data.get(track.position)? != 0xf7 {
                    track.position += 1;
                }
                track.position += 1;
                return Some(None);
            }
            0xf1..=0xff => return Some(None),
            _ => {}
        }
        track.status = byte;
        byte = *data.get(track.position)?;
    }
    if track.status < 0x80 {
        return None;
    }
    let ch = track.status & 0xf;
    track.position += 1;
    let mut second = || {
        let b = data.get(track.position).copied().unwrap_or(0) & 0x7f;
        track.position += 1;
        b
    };
    Some(match track.status & 0xf0 {
        0x80 => {
            second();
            Some(Event::NoteOff(ch, byte))
        }
        0x90 => Some(Event::NoteOn(ch, byte, second())),
        0xa0 => {
            second();
            None
        }
        0xb0 => Some(Event::Controller(ch, byte, second())),
        0xc0 => Some(Event::Program(ch, byte)),
        0xd0 => None,
        _ => Some(Event::Pitch(
            ch,
            u16::from(byte) | (u16::from(second()) << 7),
        )),
    })
}

fn rewind(player: &mut Player) {
    let data = player.data.clone();
    for track in &mut player.tracks {
        track.position = track.start + 2;
        track.status = 0;
        track.done = false;
        track.next = read_delta(&data, &mut track.position);
    }
    player.tick = 0;
}

/// Loop: replay from the start, silently catching up to the loop point.
fn restart(player: &mut Player) {
    rewind(player);
    let target = player.loop_tick.unwrap_or(0);
    player.channels.iter_mut().for_each(|c| c.hold = false);
    let mut discard = Vec::new();
    while player.tick < target {
        step_player(player, &mut discard, true);
        for event in discard.drain(..) {
            match event {
                Event::Program(ch, p) => player.channels[usize::from(ch)].program = p,
                Event::Controller(ch, 7, v) => player.channels[usize::from(ch)].volume = v >> 1,
                Event::Pitch(ch, v) => player.channels[usize::from(ch)].pitch = v,
                _ => {}
            }
        }
    }
}

// ------------------------------------------------------------ audio thread

// ------------------------------------------------------------- game side

static COMMANDS: OnceLock<Mutex<mpsc::Sender<Command>>> = OnceLock::new();
/// DS:4ec1, the volume setting: effects 4, 9, 16, 18, 19 and 20 need it
/// non-zero (play_object_sound).
static VOLUME: Mutex<u16> = Mutex::new(6);
/// status_counter_a..d (run_game_loop, 1000:12ab): looping sounds 12, 9, 2
/// and 1 stop at the end of an update no part refreshed them in.
static LOOPS: Mutex<[u8; 4]> = Mutex::new([0; 4]);
const LOOP_SOUNDS: [SoundEffect; 4] = [
    SoundEffect::ElectricalDriveRunning,
    SoundEffect::FanRunning,
    SoundEffect::Monkey,
    SoundEffect::MotorRunning,
];
const VOLUME_GATED_EFFECTS: [SoundEffect; 6] = [
    SoundEffect::UnusedEffect4,
    SoundEffect::FanRunning,
    SoundEffect::ScissorsSnap,
    SoundEffect::MechanismMovement,
    SoundEffect::PuzzleComplete,
    SoundEffect::BowlingBallRolling,
];

fn send(command: Command) {
    if let Some(sender) = COMMANDS.get() {
        let _ = sender.lock().unwrap().send(command);
    }
}

/// Open the audio device and start the synthesiser. Returns quietly with an
/// error when there is no device; the game then runs silently.
#[cfg(not(target_arch = "wasm32"))]
pub fn init(resources: &Resources, volume: u16) -> Result<()> {
    if COMMANDS.get().is_some() {
        return Ok(());
    }
    let bank = Bank::load(resources)?;
    *VOLUME.lock().unwrap() = volume.min(6);
    let (sender, receiver) = mpsc::channel();
    let rate = {
        use rodio::cpal::traits::{DeviceTrait, HostTrait};
        rodio::cpal::default_host()
            .default_output_device()
            .and_then(|device| device.default_output_config().ok())
            .map_or(48_000, |config| config.sample_rate().0)
    };
    let output = Output {
        engine: Engine::new(&bank, volume, rate),
        commands: receiver,
        buffer: vec![0.0; 512],
        position: 512,
    };
    let (stream, handle) =
        rodio::OutputStream::try_default().map_err(|e| format!("Audio output: {e}"))?;
    handle
        .play_raw(output)
        .map_err(|e| format!("Audio output: {e}"))?;
    // The stream must outlive the game; it is not Send, so it is leaked here.
    std::mem::forget(stream);
    let _ = COMMANDS.set(Mutex::new(sender));
    Ok(())
}

/// play_object_sound, 1000:83ab.
/// Gameplay effects are typed; numeric IDs enter only at the resource/audio boundary.
pub fn play_object_sound(effect: SoundEffect) {
    if VOLUME_GATED_EFFECTS.contains(&effect) && *VOLUME.lock().unwrap() == 0 {
        return;
    }
    if let Some(slot) = LOOP_SOUNDS.iter().position(|&s| s == effect) {
        LOOPS.lock().unwrap()[slot] = 2;
    }
    send(Command::Play(effect.id()));
}

pub fn play_effect(effect: SoundEffect) {
    play_object_sound(effect);
}

pub fn keep_effect_looping(effect: SoundEffect) {
    keep_looping(effect);
}

/// stop_sound_group, 1000:83ea, with the native group selector named.
pub fn stop_sound_group(group: SoundGroup) {
    send(match group {
        SoundGroup::AllEffects => Command::StopEffects,
        SoundGroup::Effect(effect) => Command::Stop(effect.id()),
        SoundGroup::Music(track) => Command::Stop(track.id()),
    });
}

/// select_sound_bank, 1000:8364: switch to another music track (or none).
pub fn select_music(id: Option<u16>) {
    static CURRENT: Mutex<Option<u16>> = Mutex::new(None);
    let mut current = CURRENT.lock().unwrap();
    if *current == id {
        return;
    }
    if let Some(old) = current.take() {
        send(Command::Stop(old));
    }
    if let Some(id) = id {
        send(Command::Play(id));
    }
    *current = id;
}

/// Port option: switch music and sound effects on or off independently.
pub fn set_channels(music: bool, effects: bool) {
    send(Command::Channels { music, effects });
}

pub fn set_volume(setting: u16) {
    *VOLUME.lock().unwrap() = setting.min(6);
    send(Command::Volume(setting));
}

/// A looping part keeps its sound alive this update (sets its counter to 2).
pub fn keep_looping(effect: SoundEffect) {
    if let Some(slot) = LOOP_SOUNDS.iter().position(|&s| s == effect) {
        LOOPS.lock().unwrap()[slot] = 2;
    }
}

/// run_game_loop: counters drop to 1 before the parts update...
pub fn begin_update() {
    for counter in LOOPS.lock().unwrap().iter_mut() {
        if *counter != 0 {
            *counter = 1;
        }
    }
}

/// ...and a counter still at 1 afterwards stops its sound.
pub fn end_update() {
    let mut loops = LOOPS.lock().unwrap();
    for (slot, counter) in loops.iter_mut().enumerate() {
        if *counter == 1 {
            *counter = 0;
            send(Command::Stop(LOOP_SOUNDS[slot].id()));
        }
    }
}

/// The machine stopped: finalize_level_state stops every effect.
pub fn stop_machine() {
    *LOOPS.lock().unwrap() = [0; 4];
    stop_sound_group(SoundGroup::AllEffects);
}

/// Music for the original's number/letter keys during play
/// (FUN_1dff_1b09: scan codes 2..0x0a and A..G).
pub fn music_for_key(key: char) -> Option<u16> {
    match key.to_ascii_uppercase() {
        '1'..='9' => Some(1000 + key.to_digit(10)? as u16),
        'A'..='G' => Some(1010 + (key.to_ascii_uppercase() as u16 - 'A' as u16)),
        _ => None,
    }
}

/// Render `seconds` of a sound offline (tests and WAV dumps).
pub fn render_offline(resources: &Resources, id: u16, seconds: f32) -> Result<Vec<f32>> {
    let bank = Bank::load(resources)?;
    let mut engine = Engine::new(&bank, 6, OFFLINE_RATE);
    engine.play(id);
    let mut out = vec![0.0; (seconds * OFFLINE_RATE as f32) as usize];
    engine.render(&mut out);
    Ok(out)
}

pub const SAMPLE_RATE: u32 = OFFLINE_RATE;

/// The OPL register writes (output frame, register, value) a music track
/// makes, for checking the synthesis against other OPL emulators.
#[doc(hidden)]
pub fn register_log(resources: &Resources, id: u16, seconds: f32) -> Result<Vec<(u64, u8, u8)>> {
    let bank = Bank::load(resources)?;
    let mut engine = Engine::new(&bank, 6, OFFLINE_RATE);
    engine.music.log = Some(vec![(0, 0x01, 0x20), (0, 0xbd, 0)]);
    engine.play(id);
    let mut out = vec![0.0; (seconds * OFFLINE_RATE as f32) as usize];
    engine.render(&mut out);
    Ok(engine.music.log.take().unwrap_or_default())
}

#[cfg(test)]
mod tests;

#[cfg(target_arch = "wasm32")]
static BROWSER_OUTPUT: Mutex<Option<Output>> = Mutex::new(None);
#[cfg(target_arch = "wasm32")]
pub fn init(resources: &Resources, volume: u16) -> Result<()> {
    let bank = Bank::load(resources)?;
    *VOLUME.lock().unwrap() = volume.min(6);
    let (sender, receiver) = mpsc::channel();
    let output = Output {
        engine: Engine::new(&bank, volume, 48_000),
        commands: receiver,
        buffer: vec![0.0; 512],
        position: 512,
    };
    *BROWSER_OUTPUT.lock().unwrap() = Some(output);
    let _ = COMMANDS.set(Mutex::new(sender));
    Ok(())
}
#[cfg(target_arch = "wasm32")]
pub fn browser_samples(count: usize) -> Vec<f32> {
    let mut out = BROWSER_OUTPUT.lock().unwrap();
    match out.as_mut() {
        Some(output) => output.take(count).collect(),
        None => vec![0.0; count],
    }
}
