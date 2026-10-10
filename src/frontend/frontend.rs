use super::*;

pub struct Frontend {
    pub root: PathBuf,
    pub progress: Progress,
    pub passwords: Passwords,
    pub titles: Vec<String>,
    pub(super) descriptions: Vec<String>,
    pub session: PuzzleSession,
    pub view: View,
    pub dialog: Option<Dialog>,
    /// The control held down; actions fire on release over the same control.
    pub pressed: Option<Control>,
    pub select_pressed: Option<SelectHit>,
    pub file_pressed: Option<FileHit>,
    /// A message waiting behind the current dialog (e.g. after SELECT closes).
    pub(super) queued: Option<Dialog>,
    pub quit: bool,
    pub qol_open: bool,
    pub graphics_menu: crate::menus::GraphicsMenu,
    pub solutions: crate::solutions::SolutionMenu,
    /// The puzzle save last written, so unchanged machines are not rewritten.
    pub(super) saved: Option<Vec<u8>>,
}

impl Frontend {
    pub fn new(resources: &Resources, root: PathBuf, start: Option<u16>) -> Result<Self, String> {
        crate::default_solutions::seed(&root)?;
        let progress = Progress::load(&root);
        let root_for_saves = root.clone();
        let count = crate::control_panel::level_count(resources);
        let mut titles = Vec::new();
        let mut descriptions = Vec::new();
        for level in 1..=count {
            let header = crate::machine::collection_metadata(
                resources,
                &crate::control_panel::level_name(level),
            )?;
            titles.push(header.title);
            descriptions.push(header.description);
        }
        // initialize_score_screen_assets starts at puzzle 1; the port reopens
        // the last puzzle played instead, if it is still unlocked.
        let available = if progress.unlock_all {
            count
        } else {
            progress.reached
        };
        let last = progress.last_puzzle.min(available);
        let level = start.unwrap_or(last).clamp(1, count.max(1));
        let mut solutions = crate::solutions::SolutionMenu::default();
        solutions.refresh(&root, level);
        Ok(Self {
            root,
            progress,
            passwords: Passwords::load(resources)?,
            titles,
            descriptions,
            session: open_puzzle(resources, &root_for_saves, level, 0, true)?,
            view: View::Panel,
            dialog: None,
            pressed: None,
            select_pressed: None,
            file_pressed: None,
            queued: None,
            quit: false,
            qol_open: false,
            graphics_menu: Default::default(),
            solutions,
            saved: None,
        })
    }

    pub fn level_count(&self) -> u16 {
        self.titles.len() as u16
    }

    /// Menu access without changing earned progress or completed puzzles.
    pub fn available_levels(&self) -> u16 {
        if self.progress.unlock_all {
            self.level_count()
        } else {
            self.progress.reached
        }
    }

    pub fn description(&self) -> &str {
        self.descriptions
            .get(usize::from(self.session.level).wrapping_sub(1))
            .map_or("", String::as_str)
    }

    /// Esc on the game screen: back to the panel with the machine kept.
    pub fn open_panel(&mut self) {
        self.session.stop();
        self.autosave();
        if self.session.phase != Phase::Editing {
            self.session.phase = Phase::Editing;
        }
        self.session.editor.cancel();
        self.view = View::Panel;
    }

    pub(super) fn load_puzzle(&mut self, resources: &Resources, level: u16, score: i32) {
        self.load_puzzle_from(resources, level, score, true);
    }

    /// Switch puzzles, saving the one being left. `restore` reopens the
    /// machine last built for the new puzzle.
    pub(super) fn load_puzzle_from(
        &mut self,
        resources: &Resources,
        level: u16,
        score: i32,
        restore: bool,
    ) {
        self.autosave();
        match open_puzzle(resources, &self.root, level, score, restore) {
            Ok(session) => {
                self.session = session;
                self.solutions = crate::solutions::SolutionMenu::default();
                self.solutions.refresh(&self.root, level);
                if self.progress.last_puzzle != level {
                    self.progress.last_puzzle = level;
                    self.save_progress();
                }
            }
            Err(error) => {
                self.dialog = Some(Dialog::Message(
                    Message::notice("FILE ERROR", &error),
                    After::Nothing,
                ))
            }
        }
    }

    pub fn save_progress(&mut self) {
        let _ = self.progress.save(&self.root);
    }

    /// Keep the current puzzle's machine in SAVES/, if it changed. Freeform
    /// machines are saved by the player through SAVE instead.
    pub fn autosave(&mut self) {
        if self.session.freeform || self.session.level == 0 {
            return;
        }
        let bytes = self.session.snapshot_bytes();
        if self.saved.as_ref() == Some(&bytes) {
            return;
        }
        let path = crate::progress::puzzle_save_path(&self.root, self.session.level);
        if let Some(dir) = path.parent() {
            let _ = crate::storage::create_dir_all(dir);
        }
        if crate::storage::write(&path, &bytes).is_ok() {
            self.saved = Some(bytes);
        }
    }

    pub fn open_solutions(&mut self) {
        if self.session.freeform || !self.progress.puzzle_hints {
            return;
        }
        self.session.stop();
        if self.session.phase != Phase::Editing {
            return;
        }
        self.solutions.refresh(&self.root, self.session.level);
        self.solutions.status = "1 HINT reveals just one item.".into();
        self.solutions.editing_name = false;
        self.solutions.manage = false;
        self.solutions.open = true;
    }

    /// Menu actions deliberately separate selection, reveal, and full load.
    pub fn solution_action(&mut self, resources: &Resources, action: crate::menus::SolutionAction) {
        use crate::menus::SolutionAction::*;
        let result: Result<(), String> = (|| {
            match action {
                Up | Down => {
                    let count = self.solutions.entries.len();
                    if count > 0 {
                        let selected = self.solutions.selected.unwrap_or(0);
                        self.solutions.selected = Some(if action == Up {
                            (selected + count - 1) % count
                        } else {
                            (selected + 1) % count
                        });
                        self.solutions
                            .remember_selection(&self.root, self.session.level);
                        self.solutions.status = "Selected. Your board is unchanged.".into();
                    }
                }
                Manage => self.solutions.manage = !self.solutions.manage,
                Done => self.solutions.open = false,
                Name => self.solutions.editing_name = true,
                Save | Update => {
                    if action == Update && self.solutions.selected_is_default() {
                        return Err(
                            "Default solutions are read-only. Use SAVE AS for your own copy."
                                .into(),
                        );
                    }
                    if self.session.editor.held.is_some() || self.session.editor.resizing.is_some()
                    {
                        return Err("Finish placing the held item first.".into());
                    }
                    let bytes = self.session.snapshot_bytes();
                    if action == Save {
                        self.solutions
                            .save_new(&self.root, self.session.level, &bytes)?;
                    } else {
                        let path = &self
                            .solutions
                            .selected
                            .and_then(|i| self.solutions.entries.get(i))
                            .ok_or("Select a solution first.")?
                            .1;
                        crate::storage::write(path, bytes).map_err(|e| e.to_string())?;
                    }
                    self.solutions.status = "Solution saved.".into();
                }
                Delete => {
                    if self.solutions.selected_is_default() {
                        return Err(
                            "Default solutions are read-only. Use SAVE AS for your own copy."
                                .into(),
                        );
                    }
                    if !self.solutions.delete_pending {
                        self.solutions.delete_pending = true;
                        self.solutions.status = "Click DELETE again to confirm.".into();
                        return Ok(());
                    }
                    let path = &self
                        .solutions
                        .selected
                        .and_then(|i| self.solutions.entries.get(i))
                        .ok_or("Select a solution first.")?
                        .1;
                    crate::storage::remove_file(path).map_err(|e| e.to_string())?;
                    self.solutions.refresh(&self.root, self.session.level);
                    self.solutions.status = "Solution deleted.".into();
                }
                Hint | Load | Solve => {
                    if self.session.editor.held.is_some() || self.session.editor.resizing.is_some()
                    {
                        return Err("Finish placing the held item first.".into());
                    }
                    let bytes = self.solutions.selected_bytes()?;
                    let (_, target) = crate::machine::parse_collection(&bytes, false)?;
                    let base = crate::machine::collection_objects(resources, &self.session.name)?;
                    crate::solutions::validate(&base, &target)?;
                    if action == Hint {
                        if crate::solutions::hint(&mut self.session.editor, &target)? {
                            self.solutions.status =
                                "One item added. Click 1 HINT for another.".into();
                            self.autosave();
                        } else {
                            self.solutions.status =
                                "All items in this solution are already in place.".into();
                        }
                    } else {
                        self.autosave();
                        // Keep a recovery snapshot before replacing the board.
                        let dir = crate::solutions::directory(&self.root, self.session.level);
                        let mut number = 1;
                        let path = loop {
                            let p = dir.join(format!("Before load {number}.TIM"));
                            if !crate::storage::exists(&p) {
                                break p;
                            }
                            number += 1;
                        };
                        crate::storage::write(path, self.session.snapshot_bytes())
                            .map_err(|e| e.to_string())?;
                        self.session.restore(&bytes)?;
                        self.autosave();
                        self.solutions.open = false;
                        if action == Solve {
                            self.view = View::Game;
                            self.session.start();
                        }
                    }
                }
            }
            if action != Delete {
                self.solutions.delete_pending = false;
            }
            Ok(())
        })();
        if let Err(error) = result {
            self.solutions.manage = true;
            self.solutions.status = error;
        }
    }

    // --- Panel controls -------------------------------------------------

    pub fn panel_press(&mut self, cursor: (i16, i16)) {
        if self.dialog.is_some() {
            self.dialog_press(cursor);
            return;
        }
        self.pressed = crate::menus::control_at(cursor);
        // The meters act while held (1dff:3371 / 1dff:340a).
        if matches!(
            self.pressed,
            Some(Control::TopMeter | Control::BottomMeter | Control::Speed)
        ) {
            self.panel_drag(cursor);
        }
    }

    pub fn panel_drag(&mut self, cursor: (i16, i16)) {
        if self.dialog.is_some() {
            return;
        }
        if self.pressed == Some(Control::Speed) {
            self.progress.speed = crate::menus::speed_at(cursor.0);
            return;
        }
        if !self.session.freeform {
            return;
        }
        match self.pressed {
            Some(Control::TopMeter) => {
                let value = crate::menus::meter_value(cursor.0, 0x200);
                self.session.set_environment(self.session.air, value);
            }
            Some(Control::BottomMeter) => {
                let value = crate::menus::meter_value(cursor.0, 0x80);
                self.session.set_environment(value, self.session.gravity);
            }
            _ => {}
        }
    }

    /// The option row on the game screen: press, drag (the speed slider)
    /// and release, as on the panel. Returns whether the press was taken.
    pub fn option_press(&mut self, cursor: (i16, i16)) -> bool {
        let Some(control) = crate::menus::option_at(cursor) else {
            return false;
        };
        self.pressed = Some(control);
        self.option_drag(cursor);
        true
    }

    pub fn option_drag(&mut self, cursor: (i16, i16)) {
        if self.pressed == Some(Control::Speed) {
            self.progress.speed = crate::menus::speed_at(cursor.0);
        }
    }

    pub fn option_release(&mut self, resources: &Resources, cursor: (i16, i16)) {
        let Some(control) = self.pressed.take() else {
            return;
        };
        if control == Control::Speed || crate::menus::option_at(cursor) == Some(control) {
            self.activate(resources, control);
        }
    }

    pub fn panel_release(&mut self, resources: &Resources, cursor: (i16, i16)) {
        if self.dialog.is_some() {
            self.dialog_release(resources, cursor);
            return;
        }
        let Some(control) = self.pressed.take() else {
            return;
        };
        // The slider keeps its last notch even when released off the track.
        if control != Control::Speed && crate::menus::control_at(cursor) != Some(control) {
            return;
        }
        self.activate(resources, control);
    }

    /// The mode handlers in the CS:34bf table.
    pub fn activate(&mut self, resources: &Resources, control: Control) {
        let freeform = self.session.freeform;
        match control {
            Control::Play => self.view = View::Game,
            Control::VolumeUp | Control::VolumeDown => {
                let up = control == Control::VolumeUp;
                self.progress.volume = if up {
                    (self.progress.volume + 1).min(6)
                } else {
                    self.progress.volume.saturating_sub(1)
                };
                crate::sound::set_volume(self.progress.volume);
                self.save_progress();
            }
            Control::Solutions => self.open_solutions(),
            Control::Qol => {
                self.qol_open = true;
            }
            Control::Music | Control::Effects => {
                if control == Control::Music {
                    self.progress.music = !self.progress.music;
                } else {
                    self.progress.effects = !self.progress.effects;
                }
                crate::sound::set_channels(self.progress.music, self.progress.effects);
                self.save_progress();
            }
            // Set while dragging; saved on release.
            Control::Speed => self.save_progress(),
            Control::Quit => self.ask(
                "QUIT GAME",
                "Are you sure you want to quit the game?",
                After::Quit,
            ),
            Control::Restart => self.ask(
                "RESTART LEVEL",
                "Are you sure you want to clear all parts and restart this level?",
                After::Restart,
            ),
            Control::Freeform if !freeform => self.ask(
                "FREEFORM MODE",
                "Are you sure you want to enter freeform mode?",
                After::EnterFreeform,
            ),
            Control::SelectPuzzle if freeform => self.ask(
                "LEAVE FREEFORM MODE",
                "Are you sure you want to leave freeform mode?",
                After::LeaveFreeform,
            ),
            Control::SelectPuzzle => {
                self.dialog = Some(Dialog::Select(SelectPuzzle::new(self.session.level)))
            }
            Control::Load if freeform => {
                self.dialog = Some(Dialog::File(FileDialog::open(FileMode::Load, &self.root)))
            }
            Control::Save if freeform => {
                self.dialog = Some(Dialog::File(FileDialog::open(FileMode::Save, &self.root)))
            }
            Control::TopMeter if !freeform => self.notice(
                "CAN'T CHANGE GRAVITY",
                "You are only allowed to change the gravitational force in freeform mode.",
            ),
            Control::BottomMeter if !freeform => self.notice(
                "CAN'T CHANGE AIR PRESSURE",
                "You are only allowed to change the air pressure in freeform mode.",
            ),
            _ => {}
        }
        let _ = resources;
    }

    pub(super) fn ask(&mut self, title: &str, text: &str, after: After) {
        self.dialog = Some(Dialog::Message(Message::question(title, text), after));
    }

    pub(super) fn notice(&mut self, title: &str, text: &str) {
        self.dialog = Some(Dialog::Message(
            Message::notice(title, text),
            After::Nothing,
        ));
    }

    pub(super) fn answer(&mut self, resources: &Resources, yes: bool, after: After) {
        self.dialog = self.queued.take();
        if !yes {
            return;
        }
        match after {
            After::Nothing => {}
            After::Quit => self.quit = true,
            // clear_all_object_relationships, then straight to the game screen.
            After::Restart => {
                if self.session.freeform {
                    if let Ok(session) = PuzzleSession::freeform(resources, None) {
                        self.session = session;
                    }
                } else {
                    let level = self.session.level;
                    let _ = crate::storage::remove_file(crate::progress::puzzle_save_path(
                        &self.root, level,
                    ));
                    self.saved = None;
                    // Nothing to keep from the cleared machine.
                    self.session.level = 0;
                    self.load_puzzle_from(resources, level, self.session.score, false);
                }
                self.view = View::Game;
            }
            // 1dff:30fd: default freeform collection, score cleared.
            After::EnterFreeform => match PuzzleSession::freeform(resources, None) {
                Ok(session) => self.session = session,
                Err(error) => self.notice("FILE ERROR", &error),
            },
            // 1dff:315f: the puzzle list follows leaving freeform.
            After::LeaveFreeform => {
                let level = self.progress.reached.min(self.level_count()).max(1);
                self.load_puzzle(resources, level, 0);
                self.dialog = Some(Dialog::Select(SelectPuzzle::new(level)));
            }
        }
    }

    // --- Dialog input ---------------------------------------------------

    pub(super) fn dialog_press(&mut self, cursor: (i16, i16)) {
        match &self.dialog {
            Some(Dialog::Select(select)) => self.select_pressed = select.hit(cursor),
            Some(Dialog::File(file)) => self.file_pressed = file.hit(cursor),
            _ => {}
        }
    }

    pub(super) fn dialog_release(&mut self, resources: &Resources, cursor: (i16, i16)) {
        match self.dialog.clone() {
            Some(Dialog::Message(..)) => {}
            Some(Dialog::Select(select)) => {
                let pressed = self.select_pressed.take();
                if pressed.is_some() && pressed == select.hit(cursor) {
                    self.select_action(resources, select, pressed.unwrap());
                }
            }
            Some(Dialog::File(file)) => {
                let pressed = self.file_pressed.take();
                if pressed.is_some() && pressed == file.hit(cursor) {
                    self.file_action(resources, file, pressed.unwrap());
                }
            }
            None => {}
        }
    }

    /// A click on a message box button (needs the font for button widths).
    pub fn message_click(
        &mut self,
        resources: &Resources,
        font: &crate::font::Font,
        cursor: (i16, i16),
    ) {
        if let Some(Dialog::Message(message, after)) = self.dialog.clone() {
            if let Some(yes) = message.choice(font, cursor) {
                self.answer(resources, yes, after);
            }
        }
    }

    pub(super) fn select_action(
        &mut self,
        resources: &Resources,
        mut select: SelectPuzzle,
        hit: SelectHit,
    ) {
        select.typing = false;
        match hit {
            SelectHit::Entry(level) if level <= self.level_count() => {
                if level > self.available_levels() {
                    self.queued = Some(Dialog::Select(select));
                    self.dialog = Some(Dialog::Message(
                        Message::notice(
                            "NEED PASSWORD",
                            "You need to enter the correct password in order to try this puzzle.",
                        ),
                        After::Nothing,
                    ));
                    return;
                }
                select.selected = level;
            }
            SelectHit::Entry(_) => {}
            SelectHit::PageUp => {
                if select.first > 0x15 {
                    select.first -= 0x15;
                }
            }
            SelectHit::PageDown => {
                if select.first + 0x15 <= self.level_count() {
                    select.first += 0x15;
                }
            }
            SelectHit::Password => select.typing = true,
            SelectHit::Done => {
                self.dialog = None;
                if select.selected != self.session.level || self.session.freeform {
                    let score = if self.session.freeform {
                        0
                    } else {
                        self.session.score
                    };
                    self.load_puzzle(resources, select.selected, score);
                }
                return;
            }
        }
        self.dialog = Some(Dialog::Select(select));
    }

    /// Enter in the password field (1dff:10c0's 0x800 branch).
    pub(super) fn submit_password(&mut self, mut select: SelectPuzzle) {
        select.typing = false;
        match self.passwords.decode(&select.password) {
            None => {
                self.queued = Some(Dialog::Select(select));
                self.dialog = Some(Dialog::Message(
                    Message::notice("BAD PASSWORD", "That is not a valid password."),
                    After::Nothing,
                ));
            }
            Some(result) => {
                let (level, score) = match result {
                    Ok(found) => found,
                    Err(level) => (level, 0),
                };
                let level = level.min(self.level_count());
                self.session.score = score;
                if level > self.progress.reached {
                    self.progress.reached = level;
                    self.save_progress();
                }
                select = SelectPuzzle::new(level);
                if result.is_err() {
                    self.queued = Some(Dialog::Select(select));
                    self.dialog = Some(Dialog::Message(
                        Message::notice(
                            "SCORE CODE INVALID",
                            "That score code is invalid.  Your score will be set to zero.",
                        ),
                        After::Nothing,
                    ));
                } else {
                    self.dialog = Some(Dialog::Select(select));
                }
            }
        }
    }

    pub(super) fn file_action(
        &mut self,
        resources: &Resources,
        mut file: FileDialog,
        hit: FileHit,
    ) {
        file.typing = false;
        match hit {
            FileHit::Entry(row) => {
                file.name = file.files[row]
                    .trim_end_matches(".TIM")
                    .trim_end_matches(".tim")
                    .to_string();
            }
            FileHit::Name => file.typing = true,
            FileHit::Up => file.scroll_by(-1),
            FileHit::Down => file.scroll_by(1),
            FileHit::Cancel => {
                self.dialog = None;
                return;
            }
            FileHit::Confirm => {
                self.dialog = None;
                self.file_confirm(resources, &file);
                return;
            }
        }
        self.dialog = Some(Dialog::File(file));
    }

    pub(super) fn file_confirm(&mut self, resources: &Resources, file: &FileDialog) {
        let Some(name) = file.file_name() else { return };
        // Match an existing file case-insensitively so DOS names load.
        let existing = file
            .files
            .iter()
            .find(|f| f.eq_ignore_ascii_case(&name))
            .cloned();
        let path = self.root.join(existing.unwrap_or(name));
        match file.mode {
            FileMode::Save => {
                if crate::storage::write(&path, self.session.save_bytes()).is_err() {
                    self.notice(
                        "FILE ERROR",
                        "Disk is write protected or there is not enough memory on that disk to save this machine.",
                    );
                }
            }
            FileMode::Load => {
                let loaded = crate::storage::read(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|bytes| PuzzleSession::freeform(resources, Some(&bytes)));
                match loaded {
                    Ok(session) => self.session = session,
                    Err(_) => self.notice(
                        "FILE ERROR",
                        "That file is not in 'The Incredible Machine' format.",
                    ),
                }
            }
        }
    }

    /// Typed characters, Enter, and Escape for the open dialog.
    pub fn type_text(&mut self, resources: &Resources, typed: &str, enter: bool, escape: bool) {
        match self.dialog.clone() {
            Some(Dialog::Select(mut select)) => {
                if escape {
                    self.dialog = None;
                } else if select.typing && enter {
                    self.submit_password(select);
                } else if select.typing {
                    crate::menus::edit_text(&mut select.password, typed, 0x18);
                    self.dialog = Some(Dialog::Select(select));
                } else if enter {
                    self.select_action(resources, select, SelectHit::Done);
                }
            }
            Some(Dialog::File(mut file)) => {
                if escape {
                    self.dialog = None;
                } else if enter {
                    self.dialog = None;
                    self.file_confirm(resources, &file);
                } else if file.typing {
                    crate::menus::edit_text(&mut file.name, typed, 8);
                    self.dialog = Some(Dialog::File(file));
                }
            }
            Some(Dialog::Message(message, after)) => {
                if enter {
                    self.answer(resources, true, after);
                } else if escape {
                    self.answer(resources, message.second.is_none(), after);
                }
            }
            None => {}
        }
    }

    // --- Puzzle completion ----------------------------------------------

    /// Called every update: records progress and the new password when a
    /// puzzle is solved (run_score_screen writes TIM.CFG on advance).
    pub fn update_progress(&mut self) {
        let session = &mut self.session;
        if session.phase == Phase::Complete && session.new_password.is_none() && !session.freeform {
            session.new_password = self.passwords.generate(session.level, session.score);
            let next = (session.level + 1).min(self.titles.len() as u16);
            self.progress.reached = self.progress.reached.max(next);
            self.progress.completed.insert(session.level);
            let _ = self.progress.save(&self.root);
            // Keep the layout that actually reached the goal, separately
            // from recovery snapshots of unfinished boards.
            let dir = crate::solutions::directory(&self.root, session.level);
            if crate::storage::create_dir_all(&dir).is_ok() {
                let _ = crate::storage::write(
                    dir.join("Last successful run.TIM"),
                    session.snapshot_bytes(),
                );
            }
            // The solved machine is what reopening the puzzle shows.
            self.autosave();
        }
    }

    /// ADVANCE: the next puzzle with the banked score, or freeform after the
    /// last one (handle_level_complete's SOLVED ALL PUZZLES branch).
    pub fn advance(&mut self, resources: &Resources) {
        let score = self.session.score;
        if self.session.level >= self.level_count() {
            if let Ok(session) = PuzzleSession::freeform(resources, None) {
                self.session = session;
            }
            self.view = View::Panel;
            return;
        }
        self.load_puzzle(resources, self.session.level + 1, score);
        self.view = View::Panel;
    }
}
