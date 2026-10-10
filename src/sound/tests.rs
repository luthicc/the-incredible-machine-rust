use super::*;

fn resources() -> Resources {
    Resources::load(crate::test_support::game_data_root()).unwrap()
}

fn loudness(engine: &mut Engine, seconds: f32) -> f32 {
    let mut out = vec![0.0; (seconds * OFFLINE_RATE as f32) as usize];
    engine.render(&mut out);
    out.iter().map(|v| v.abs()).fold(0.0, f32::max)
}

#[test]
fn volume_zero_and_the_switches_silence_their_sounds() {
    let bank = Bank::load(&resources()).unwrap();
    // Music 1001 alone is audible at full volume, silent at 0.
    let mut engine = Engine::new(&bank, 6, OFFLINE_RATE);
    engine.play(MusicTrack::Track1.id());
    assert!(loudness(&mut engine, 2.0) > 0.01);
    engine.command(Command::Volume(0));
    loudness(&mut engine, 0.1);
    // Only the output filter's decaying tail remains.
    assert!(loudness(&mut engine, 1.0) < 1e-6);

    // Music off keeps the effects; effects off keeps the music.
    let mut engine = Engine::new(&bank, 6, OFFLINE_RATE);
    engine.command(Command::Channels {
        music: false,
        effects: true,
    });
    engine.play(MusicTrack::Track1.id());
    loudness(&mut engine, 0.1);
    assert!(loudness(&mut engine, 1.0) < 1e-3, "music should be muted");
    engine.play(SoundEffect::Monkey.id());
    assert!(loudness(&mut engine, 1.0) > 0.01, "effects should sound");
    engine.command(Command::Channels {
        music: true,
        effects: false,
    });
    engine.stop(SoundEffect::Monkey.id());
    engine.play(SoundEffect::Monkey.id());
    loudness(&mut engine, 0.05);
    assert!(
        loudness(&mut engine, 1.0) > 0.01,
        "music should sound again"
    );
}

#[test]
fn bank_decodes_every_effect_and_track() {
    let bank = Bank::load(&resources()).unwrap();
    let mut ids: Vec<u16> = bank.sounds.keys().copied().collect();
    ids.sort();
    assert_eq!(ids, (1..=20).chain(1001..=1016).collect::<Vec<_>>());
    assert!(
        bank.sounds[&MusicTrack::Track1.id()].music
            && bank.sounds[&MusicTrack::Track1.id()].looping
    );
    assert!(
        bank.sounds[&SoundEffect::MotorRunning.id()].looping
            && !bank.sounds[&SoundEffect::MotorRunning.id()].music
    );
    assert!(matches!(
        bank.sounds[&SoundEffect::PuzzleComplete.id()].body,
        Body::Digital { rate: 22050, .. }
    ));
    assert!(matches!(
        bank.sounds[&SoundEffect::BowlingBallRolling.id()].body,
        Body::Digital { rate: 11025, .. }
    ));
    assert_eq!(bank.driver.patches.len(), PATCHES);
}

#[test]
fn music_and_effects_make_sound() {
    let r = resources();
    for id in [
        SoundEffect::SpringRelease.id(),
        SoundEffect::Explosion.id(),
        MusicTrack::Track6.id(),
    ] {
        let out = render_offline(&r, id, 1.5).unwrap();
        let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(peak > 0.01, "sound {id} is silent");
    }
}

#[test]
fn music_keys_follow_the_native_table() {
    assert_eq!(music_for_key('1'), Some(1001));
    assert_eq!(music_for_key('9'), Some(1009));
    assert_eq!(music_for_key('a'), Some(1010));
    assert_eq!(music_for_key('G'), Some(1016));
    assert_eq!(music_for_key('H'), None);
}
