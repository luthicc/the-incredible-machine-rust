use super::*;

/// play_sound_event (3619:38b9) and its two chips: music on one, effects on
/// the other, digital samples mixed beside them.
pub(super) struct Engine {
    pub(super) bank: Arc<HashMap<u16, Sound>>,
    pub(super) driver: Arc<Driver>,
    pub(super) music: Synth,
    pub(super) effects: Synth,
    pub(super) samples: Vec<Sample>,
    pub(super) master: u8,
    /// Port options: mute the music chip, or the effects chip and samples.
    pub(super) music_on: bool,
    pub(super) effects_on: bool,
    pub(super) next_uid: u64,
    pub(super) rate: u32,
    pub(super) tick_phase: u32,
    /// DC-blocking filter state (the OPL's rectified waveforms carry DC).
    pub(super) dc: (f32, f32),
    /// The card's analog output stage after the DAC.
    pub(super) lowpass: Lowpass,
}

impl Engine {
    pub(super) fn new(bank: &Bank, volume: u16, rate: u32) -> Self {
        Self {
            bank: Arc::new(bank.sounds.clone()),
            driver: bank.driver.clone(),
            music: Synth::new(rate),
            effects: Synth::new(rate),
            samples: Vec::new(),
            master: MASTER_VOLUME[usize::from(volume.min(6))],
            music_on: true,
            effects_on: true,
            next_uid: 1,
            rate,
            tick_phase: 0,
            dc: (0.0, 0.0),
            lowpass: Lowpass::new(rate),
        }
    }

    pub(super) fn command(&mut self, command: Command) {
        match command {
            Command::Play(id) => self.play(id),
            Command::Stop(id) => self.stop(id),
            Command::StopEffects => {
                for effect in SoundEffect::ALL {
                    self.stop(effect.id());
                }
            }
            Command::Volume(setting) => self.master = MASTER_VOLUME[usize::from(setting.min(6))],
            Command::Channels { music, effects } => {
                (self.music_on, self.effects_on) = (music, effects)
            }
        }
    }

    pub(super) fn play(&mut self, id: u16) {
        let Some(sound) = self.bank.get(&id).cloned() else {
            return;
        };
        // An effect already sounding is not restarted; music replaces music.
        if sound.music {
            let others: Vec<u16> = self
                .music
                .players
                .iter()
                .map(|p| p.id)
                .filter(|&p| p != id)
                .collect();
            for other in others {
                self.stop(other);
            }
        }
        if self.music.playing(id).is_some()
            || self.effects.playing(id).is_some()
            || self.samples.iter().any(|s| s.id == id)
        {
            return;
        }
        let uid = self.next_uid;
        self.next_uid += 1;
        match &sound.body {
            Body::Midi { data, tracks } => {
                let synth = if sound.music {
                    &mut self.music
                } else {
                    &mut self.effects
                };
                synth.start(uid, id, &sound, data, tracks);
            }
            Body::Digital { rate, samples } => self.samples.push(Sample {
                id,
                looping: sound.looping,
                rate: *rate,
                samples: samples.clone(),
                position: 0,
            }),
        }
    }

    pub(super) fn stop(&mut self, id: u16) {
        for synth in [&mut self.music, &mut self.effects] {
            while let Some(uid) = synth.playing(id) {
                synth.stop(uid);
            }
        }
        self.samples.retain(|s| s.id != id);
    }

    pub(super) fn render(&mut self, out: &mut [f32]) {
        for frame in out.iter_mut() {
            self.tick_phase += TICK_RATE;
            if self.tick_phase >= self.rate {
                self.tick_phase -= self.rate;
                let driver = self.driver.clone();
                self.music.tick(&driver, self.master);
                self.effects.tick(&driver, self.master);
            }
            let (music, _) = self.music.chip.render_frame();
            let (effects, _) = self.effects.chip.render_frame();
            self.music.frames += 1;
            self.effects.frames += 1;
            // Effects use one or two voices against the music's nine; +6 dB
            // keeps them audible over it.
            // The chips keep running while muted so music stays in time.
            let music = if self.music_on { music } else { 0 };
            let effects = if self.effects_on { effects } else { 0 };
            let mut mix = (music + 2 * effects) as f32 * OUTPUT_GAIN;
            let gain = if self.effects_on {
                f32::from(self.master) / 15.0 * SAMPLE_GAIN
            } else {
                0.0
            };
            for sample in &mut self.samples {
                // Linear interpolation between the 8-bit samples.
                let index = (sample.position >> 16) as usize;
                let fraction = (sample.position & 0xffff) as f32 / 65536.0;
                let at = |i: usize| {
                    sample
                        .samples
                        .get(i)
                        .map_or(0.0, |&v| (f32::from(v) - 128.0) / 128.0)
                };
                if index < sample.samples.len() {
                    mix += (at(index) * (1.0 - fraction) + at(index + 1) * fraction) * gain;
                }
                sample.position += (u64::from(sample.rate) << 16) / u64::from(self.rate);
                if sample.looping && (sample.position >> 16) as usize >= sample.samples.len() {
                    sample.position = 0;
                }
            }
            self.samples
                .retain(|s| s.looping || ((s.position >> 16) as usize) < s.samples.len());
            // The driver's level curve never fully silences a note, so
            // volume 0 mutes the output outright.
            let mix = if self.master == 0 { 0.0 } else { mix };
            let mix = self.lowpass.process(mix);
            let filtered = mix - self.dc.0 + 0.995 * self.dc.1;
            self.dc = (mix, filtered);
            // Soft limit rather than clip when everything sounds at once.
            *frame = if filtered.abs() < 0.75 {
                filtered
            } else {
                filtered.signum() * (0.75 + 0.25 * ((filtered.abs() - 0.75) / 0.25).tanh())
            };
        }
    }
}
