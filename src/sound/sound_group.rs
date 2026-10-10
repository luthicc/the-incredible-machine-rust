use super::{MusicTrack, SoundEffect};

/// Native stop_sound_group: stop all effects, one effect, or one music track.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundGroup {
    AllEffects,
    Effect(SoundEffect),
    Music(MusicTrack),
}
