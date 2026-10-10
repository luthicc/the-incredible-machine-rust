use super::*;

/// One OPL2 chip, its nine voices and the sequences driving them.
pub(super) struct Synth {
    pub(super) chip: oplon::Opl2,
    pub(super) voices: [Voice; VOICES],
    /// Last 0xb0 write per voice (key, block, F-number high bits).
    pub(super) key_registers: [u8; VOICES],
    pub(super) players: Vec<Player>,
    pub(super) clock: u64,
    /// Output frames rendered, and an optional log of register writes
    /// against them (for comparing emulators offline).
    pub(super) frames: u64,
    pub(super) log: Option<Vec<(u64, u8, u8)>>,
}

impl Synth {
    pub(super) fn new(rate: u32) -> Self {
        let mut chip = oplon::Opl2::new(rate);
        chip.write_reg(0x01, 0x20);
        chip.write_reg(0xbd, 0);
        Self {
            chip,
            voices: [Voice::default(); VOICES],
            key_registers: [0; VOICES],
            players: Vec::new(),
            clock: 0,
            frames: 0,
            log: None,
        }
    }

    pub(super) fn start(
        &mut self,
        uid: u64,
        id: u16,
        sound: &Sound,
        data: &Arc<[u8]>,
        tracks: &[(usize, usize)],
    ) {
        let tracks = tracks
            .iter()
            .filter_map(|&(start, end)| {
                // Two header bytes: channel, then voices (low) and priority.
                let channel = *data.get(start)?;
                (channel != 0xfe).then_some(Track {
                    start,
                    end,
                    position: start + 2,
                    next: 0,
                    status: 0,
                    done: false,
                })
            })
            .collect::<Vec<_>>();
        let channels = [Channel::default(); 16];
        let mut player = Player {
            uid,
            id,
            looping: sound.looping,
            data: data.clone(),
            tracks,
            channels,
            tick: 0,
            loop_tick: None,
        };
        rewind(&mut player);
        self.players.push(player);
    }

    pub(super) fn write(&mut self, register: u8, value: u8) {
        self.chip.write_reg(register, value);
        if let Some(log) = &mut self.log {
            log.push((self.frames, register, value));
        }
    }

    pub(super) fn stop(&mut self, uid: u64) {
        self.players.retain(|p| p.uid != uid);
        for v in 0..VOICES {
            if self.voices[v].owner.is_some_and(|(owner, _)| owner == uid) {
                self.key_off(v);
                self.voices[v].owner = None;
            }
        }
    }

    pub(super) fn playing(&self, id: u16) -> Option<u64> {
        self.players.iter().find(|p| p.id == id).map(|p| p.uid)
    }

    /// Advance every sequence by one SCI tick.
    pub(super) fn tick(&mut self, driver: &Driver, master: u8) {
        let mut finished = Vec::new();
        for p in 0..self.players.len() {
            let mut events = Vec::new();
            let player = &mut self.players[p];
            step_player(player, &mut events, false);
            let uid = player.uid;
            let silent = player.tracks.iter().all(|t| t.done);
            for event in events {
                self.apply(driver, master, p, uid, event);
            }
            if silent {
                let player = &mut self.players[p];
                if player.looping {
                    restart(player);
                } else {
                    finished.push(uid);
                }
            }
        }
        for uid in finished {
            self.stop(uid);
        }
    }

    pub(super) fn apply(
        &mut self,
        driver: &Driver,
        master: u8,
        player: usize,
        uid: u64,
        event: Event,
    ) {
        match event {
            Event::NoteOn(ch, note, 0) | Event::NoteOff(ch, note) => {
                let hold = self.players[player].channels[usize::from(ch)].hold;
                for v in 0..VOICES {
                    let voice = self.voices[v];
                    if voice.owner == Some((uid, ch)) && voice.note == note && voice.on {
                        if hold {
                            self.voices[v].sustained = true;
                        } else {
                            self.key_off(v);
                        }
                    }
                }
            }
            Event::NoteOn(ch, note, velocity) => {
                self.note_on(driver, master, player, uid, ch, note, velocity)
            }
            Event::Controller(ch, controller, value) => {
                let channel = &mut self.players[player].channels[usize::from(ch)];
                match controller {
                    7 => channel.volume = value >> 1,
                    // Hold pedal: release the notes it kept on.
                    0x40 => {
                        channel.hold = value != 0;
                        if value == 0 {
                            for v in 0..VOICES {
                                if self.voices[v].owner == Some((uid, ch))
                                    && self.voices[v].sustained
                                {
                                    self.key_off(v);
                                }
                            }
                        }
                    }
                    // All notes off.
                    0x7b => {
                        for v in 0..VOICES {
                            if self.voices[v].owner == Some((uid, ch)) {
                                self.key_off(v);
                            }
                        }
                    }
                    _ => {}
                }
                if controller == 7 {
                    for v in 0..VOICES {
                        if self.voices[v].owner == Some((uid, ch)) && self.voices[v].on {
                            self.set_level(driver, master, player, v);
                        }
                    }
                }
            }
            Event::Program(ch, program) => {
                self.players[player].channels[usize::from(ch)].program = program
            }
            Event::Pitch(ch, value) => {
                self.players[player].channels[usize::from(ch)].pitch = value;
                for v in 0..VOICES {
                    if self.voices[v].owner == Some((uid, ch)) && self.voices[v].on {
                        self.set_frequency(driver, player, v, true);
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn note_on(
        &mut self,
        driver: &Driver,
        master: u8,
        player: usize,
        uid: u64,
        ch: u8,
        note: u8,
        velocity: u8,
    ) {
        let channel = self.players[player].channels[usize::from(ch)];
        let patch = if ch == 9 {
            if !(FIRST_RHYTHM_KEY..FIRST_RHYTHM_KEY + RHYTHM_KEYS as u8).contains(&note) {
                return;
            }
            MELODIC_PATCHES + usize::from(note - FIRST_RHYTHM_KEY)
        } else {
            usize::from(channel.program) % MELODIC_PATCHES
        };
        // Retrigger the same note, else a free voice (one already holding the
        // patch first), else steal the oldest. The channel headers' voice
        // counts are not limits: one-voice channels play chords.
        let owned = |v: &Voice| v.owner == Some((uid, ch));
        let pick = (0..VOICES)
            .find(|&v| owned(&self.voices[v]) && self.voices[v].note == note)
            .or_else(|| {
                (0..VOICES)
                    .filter(|&v| !self.voices[v].on)
                    .min_by_key(|&v| (self.voices[v].patch != Some(patch), self.voices[v].age))
            })
            .or_else(|| (0..VOICES).min_by_key(|&v| self.voices[v].age));
        let Some(v) = pick else { return };
        self.key_off(v);
        self.clock += 1;
        self.voices[v] = Voice {
            owner: Some((uid, ch)),
            note,
            velocity: velocity >> 1,
            patch: self.voices[v].patch,
            on: true,
            sustained: false,
            age: self.clock,
        };
        if self.voices[v].patch != Some(patch) {
            self.load_patch(driver, v, patch);
        }
        self.set_level(driver, master, player, v);
        self.set_frequency(driver, player, v, true);
    }

    pub(super) fn load_patch(&mut self, driver: &Driver, v: usize, index: usize) {
        let patch = driver.patches[index];
        let base = OPERATORS[v];
        for (offset, op) in [(base, patch.modulator), (base + 3, patch.carrier)] {
            let flags = (u8::from(op.am) << 7)
                | (u8::from(op.vibrato) << 6)
                | (u8::from(op.sustaining) << 5)
                | (u8::from(op.ksr) << 4);
            self.write(0x20 + offset, flags | op.mult);
            self.write(0x40 + offset, (op.ksl << 6) | op.total_level);
            self.write(0x60 + offset, (op.attack << 4) | op.decay);
            self.write(0x80 + offset, (op.sustain_level << 4) | op.release);
            self.write(0xe0 + offset, op.waveform);
        }
        let modulator = patch.modulator;
        self.write(
            0xc0 + v as u8,
            (modulator.feedback << 1) | u8::from(modulator.additive),
        );
        self.voices[v].patch = Some(index);
    }

    /// Output level from channel volume, note velocity and master volume
    /// through the driver's two curves; both operators sound when additive.
    pub(super) fn set_level(&mut self, driver: &Driver, master: u8, player: usize, v: usize) {
        let voice = self.voices[v];
        let (Some(index), Some((_, ch))) = (voice.patch, voice.owner) else {
            return;
        };
        let patch = driver.patches[index];
        let volume = u32::from(self.players[player].channels[usize::from(ch)].volume) + 1;
        let level =
            volume * (u32::from(driver.velocity_curve[usize::from(voice.velocity & 63)]) + 1) / 64;
        let level = (level * (u32::from(master) + 1) / 16).saturating_sub(1) as usize;
        let level = u32::from(driver.volume_curve[level.min(63)]);
        let base = OPERATORS[v];
        let attenuate = |op: &Operator| 63 - (level * (63 - u32::from(op.total_level)) / 63) as u8;
        self.write(
            0x43 + base,
            (patch.carrier.ksl << 6) | attenuate(&patch.carrier),
        );
        if patch.modulator.additive {
            self.write(
                0x40 + base,
                (patch.modulator.ksl << 6) | attenuate(&patch.modulator),
            );
        }
    }

    /// Quarter-semitone F-number steps; the pitch wheel spans two semitones.
    pub(super) fn set_frequency(&mut self, driver: &Driver, player: usize, v: usize, key: bool) {
        let voice = self.voices[v];
        let Some((_, ch)) = voice.owner else { return };
        let mut note = i32::from(voice.note);
        if ch == 9 {
            note = i32::from(driver.rhythm_keys[usize::from(voice.note - FIRST_RHYTHM_KEY)]);
        }
        // 1eef: |wheel - 0x2000| / 0xab quarter steps (±12 semitones), the
        // result capped at step 0x1fc.
        let wheel = i32::from(self.players[player].channels[usize::from(ch)].pitch) - 0x2000;
        let bend = wheel.signum() * (wheel.abs() / 0xab);
        let step = (note * 4 + bend).clamp(0, 0x1fc);
        let block = (step.div_euclid(48) - 1).clamp(0, 7) as u8;
        let fnum = driver.frequencies[step.rem_euclid(48) as usize];
        self.write(0xa0 + v as u8, fnum as u8);
        self.key_registers[v] = (u8::from(key) << 5) | (block << 2) | (fnum >> 8) as u8;
        self.write(0xb0 + v as u8, self.key_registers[v]);
    }

    pub(super) fn key_off(&mut self, v: usize) {
        if self.voices[v].on {
            self.voices[v].on = false;
            self.voices[v].sustained = false;
            self.key_registers[v] &= !0x20;
            self.write(0xb0 + v as u8, self.key_registers[v]);
        }
    }
}
