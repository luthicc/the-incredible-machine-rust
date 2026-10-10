use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Progress {
    pub resolution: u8,
    pub unlock_all: bool,
    pub reached: u16,
    pub volume: u16,
    /// Draw moving parts between simulation updates (presentation only).
    pub smooth_motion: bool,
    pub connection_hints: bool,
    pub plug_preview: bool,
    pub z_flip: bool,
    pub slow_scroll: bool,
    pub wheel_scroll: bool,
    pub stretch_screen: bool,
    pub pixel_scaling: bool,
    pub puzzle_hints: bool,
    pub double_click_copy: bool,
    pub shift_copy: bool,
    pub music: bool,
    pub effects: bool,
    /// Index into SPEEDS.
    pub speed: u8,
    /// Puzzles solved at least once.
    pub completed: std::collections::BTreeSet<u16>,
    /// The puzzle open when the game was last played; the next launch
    /// starts there (the original always starts at puzzle 1).
    pub last_puzzle: u16,
}

impl Progress {
    /// read_game_progress_from_config (1dff:4bb7).
    pub fn load(root: &Path) -> Self {
        let mut progress = match crate::storage::read(config_path(root)) {
            Ok(bytes) if bytes.len() >= 4 => Self {
                reached: u16::from_le_bytes([bytes[0], bytes[1]]).max(1),
                volume: u16::from_le_bytes([bytes[2], bytes[3]]).min(6),
                ..Self::default()
            },
            _ => Self::default(),
        };
        // TIM-BEVY.CFG: "key=value" lines.
        if let Ok(text) = crate::storage::read_to_string(options_path(root)) {
            for line in text.lines() {
                let Some((key, value)) = line.split_once('=') else {
                    continue;
                };
                let value = value.trim();
                match key.trim() {
                    "resolution" => {
                        if let Some(index) = RESOLUTIONS
                            .iter()
                            .position(|&(w, h)| value == format!("{w}x{h}"))
                        {
                            progress.resolution = index as u8;
                        }
                    }
                    "unlock_all" => progress.unlock_all = value == "on",
                    "smooth_motion" => progress.smooth_motion = value != "off",
                    "connection_hints" => progress.connection_hints = value != "off",
                    "plug_preview" => progress.plug_preview = value != "off",
                    "z_flip" => progress.z_flip = value != "off",
                    "slow_scroll" => progress.slow_scroll = value != "off",
                    "wheel_scroll" => progress.wheel_scroll = value != "off",
                    "stretch_screen" => progress.stretch_screen = value != "off",
                    "pixel_scaling" => progress.pixel_scaling = value != "off",
                    "puzzle_hints" => progress.puzzle_hints = value != "off",
                    "double_click_copy" => progress.double_click_copy = value != "off",
                    "shift_copy" => progress.shift_copy = value != "off",
                    "music" => progress.music = value != "off",
                    "effects" => progress.effects = value != "off",
                    // "speed" held the older, faster scale; it is ignored.
                    "sim_speed" => {
                        if let Ok(speed) = value.parse::<u8>() {
                            progress.speed = speed.min(SPEEDS.len() as u8 - 1);
                        }
                    }
                    "last_puzzle" => {
                        if let Ok(level) = value.parse::<u16>() {
                            progress.last_puzzle = level.max(1);
                        }
                    }
                    "completed" => {
                        progress.completed = value
                            .split(',')
                            .filter_map(|v| v.trim().parse().ok())
                            .collect();
                    }
                    _ => {}
                }
            }
        }
        progress
    }

    /// Multiplier of the native update rate for a running machine.
    pub fn speed(&self) -> f32 {
        SPEEDS[usize::from(self.speed)].0 * ONE_X
    }

    pub fn save(&self, root: &Path) -> std::io::Result<()> {
        let mut bytes = self.reached.to_le_bytes().to_vec();
        bytes.extend_from_slice(&self.volume.to_le_bytes());
        crate::storage::write(config_path(root), bytes)?;
        let (width, height) = RESOLUTIONS[usize::from(self.resolution)];
        let on = |v: bool| if v { "on" } else { "off" };
        let completed: Vec<String> = self.completed.iter().map(u16::to_string).collect();
        crate::storage::write(
            options_path(root),
            format!(
                "resolution={width}x{height}\nunlock_all={}\nconnection_hints={}\nplug_preview={}\nz_flip={}\nslow_scroll={}\nwheel_scroll={}\nstretch_screen={}\npixel_scaling={}\npuzzle_hints={}\ndouble_click_copy={}\nshift_copy={}\nsmooth_motion={}\nmusic={}\neffects={}\nsim_speed={}\nlast_puzzle={}\ncompleted={}\n",
                on(self.unlock_all),
                on(self.connection_hints),
                on(self.plug_preview),
                on(self.z_flip),
                on(self.slow_scroll),
                on(self.wheel_scroll),
                on(self.stretch_screen),
                on(self.pixel_scaling),
                on(self.puzzle_hints),
                on(self.double_click_copy),
                on(self.shift_copy),
                on(self.smooth_motion),
                on(self.music),
                on(self.effects),
                self.speed,
                self.last_puzzle,
                completed.join(","),
            ),
        )
    }
}

impl Progress {
    pub fn qol_value(&self, option: QolOption) -> bool {
        match option {
            QolOption::UnlockAll => self.unlock_all,
            QolOption::PixelScaling => self.pixel_scaling,
            QolOption::PuzzleHints => self.puzzle_hints,
            QolOption::ConnectionHints => self.connection_hints,
            QolOption::PlugPreview => self.plug_preview,
            QolOption::ZFlip => self.z_flip,
            QolOption::SmoothMotion => self.smooth_motion,
            QolOption::SlowScroll => self.slow_scroll,
            QolOption::WheelScroll => self.wheel_scroll,
            QolOption::StretchScreen => self.stretch_screen,
            QolOption::DoubleClickCopy => self.double_click_copy,
            QolOption::ShiftCopy => self.shift_copy,
        }
    }
    pub fn toggle_qol(&mut self, option: QolOption) {
        let value = match option {
            QolOption::UnlockAll => &mut self.unlock_all,
            QolOption::PixelScaling => &mut self.pixel_scaling,
            QolOption::PuzzleHints => &mut self.puzzle_hints,
            QolOption::ConnectionHints => &mut self.connection_hints,
            QolOption::PlugPreview => &mut self.plug_preview,
            QolOption::ZFlip => &mut self.z_flip,
            QolOption::SmoothMotion => &mut self.smooth_motion,
            QolOption::SlowScroll => &mut self.slow_scroll,
            QolOption::WheelScroll => &mut self.wheel_scroll,
            QolOption::StretchScreen => &mut self.stretch_screen,
            QolOption::DoubleClickCopy => &mut self.double_click_copy,
            QolOption::ShiftCopy => &mut self.shift_copy,
        };
        *value = !*value;
    }
}

impl Default for Progress {
    fn default() -> Self {
        // initialize_game_system (1dff:013f) starts the volume at 6.
        Self {
            // The browser starts as large as the page allows.
            resolution: if cfg!(target_arch = "wasm32") {
                (super::RESOLUTIONS.len() - 1) as u8
            } else {
                2
            },
            unlock_all: false,
            reached: 1,
            volume: 6,
            smooth_motion: true,
            connection_hints: true,
            plug_preview: true,
            z_flip: true,
            slow_scroll: true,
            wheel_scroll: true,
            stretch_screen: false,
            pixel_scaling: true,
            puzzle_hints: true,
            double_click_copy: true,
            shift_copy: true,
            music: true,
            effects: true,
            speed: NORMAL_SPEED,
            completed: Default::default(),
            last_puzzle: 1,
        }
    }
}
