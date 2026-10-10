use super::*;

#[derive(Default)]
pub struct SolutionMenu {
    pub open: bool,
    pub entries: Vec<(String, PathBuf)>,
    pub selected: Option<usize>,
    pub name: String,
    pub status: String,
    pub delete_pending: bool,
    pub editing_name: bool,
    pub manage: bool,
}

impl SolutionMenu {
    pub fn refresh(&mut self, root: &Path, level: u16) {
        let selected = self
            .selected
            .and_then(|i| self.entries.get(i))
            .map(|e| e.1.clone())
            .or_else(|| {
                crate::storage::read_to_string(directory(root, level).join("SELECTED"))
                    .ok()
                    .map(|name| directory(root, level).join(name.trim()))
            });
        self.entries.clear();
        let dir = directory(root, level);
        if !crate::storage::exists(&dir) {
            let old = crate::progress::puzzle_save_path(root, level);
            if crate::storage::create_dir_all(&dir).is_ok() && crate::storage::exists(&old) {
                let _ = crate::storage::copy(old, dir.join("Imported save.TIM"));
            }
        }
        if let Ok(files) = crate::storage::read_dir(directory(root, level)) {
            for file in files.flatten() {
                let path = file.path();
                if path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("tim"))
                {
                    let stem = path.file_stem().unwrap().to_string_lossy();
                    // Existing recovery filenames stay intact; make their
                    // purpose clear in the hint/solution picker.
                    let name = stem
                        .strip_prefix("Before load ")
                        .filter(|number| {
                            !number.is_empty() && number.chars().all(|c| c.is_ascii_digit())
                        })
                        .map(|number| format!("Board backup #{number}"))
                        .unwrap_or_else(|| stem.into_owned());
                    self.entries.push((name, path));
                }
            }
        }
        self.entries.sort_by(|a, b| a.0.cmp(&b.0));
        self.selected = selected
            .and_then(|p| self.entries.iter().position(|e| e.1 == p))
            .or_else(|| (!self.entries.is_empty()).then_some(0));
        self.delete_pending = false;
    }

    pub fn remember_selection(&self, root: &Path, level: u16) {
        if let Some((_, path)) = self.selected.and_then(|i| self.entries.get(i)) {
            if let Some(name) = path.file_name() {
                let _ = crate::storage::write(
                    directory(root, level).join("SELECTED"),
                    name.to_string_lossy().as_bytes(),
                );
            }
        }
    }

    pub fn selected_name(&self) -> Option<&str> {
        self.selected
            .and_then(|i| self.entries.get(i))
            .map(|e| e.0.as_str())
    }

    pub fn selected_is_default(&self) -> bool {
        self.selected
            .and_then(|i| self.entries.get(i))
            .is_some_and(|(_, path)| crate::default_solutions::is_default(path))
    }

    pub fn selected_bytes(&self) -> Result<Vec<u8>, String> {
        let (_, path) = self
            .selected
            .and_then(|i| self.entries.get(i))
            .ok_or("Select a solution first.")?;
        crate::storage::read(path).map_err(|e| e.to_string())
    }

    pub fn save_new(&mut self, root: &Path, level: u16, bytes: &[u8]) -> Result<(), String> {
        let name = self.name.trim();
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || " -_".contains(c))
        {
            return Err("Use a name with letters, numbers, spaces or -_.".into());
        }
        let dir = directory(root, level);
        crate::storage::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join(format!("{name}.TIM"));
        if crate::default_solutions::is_default(&path) {
            return Err("Default solutions are read-only. Choose a new name.".into());
        }
        // Save as never overwrites another variant, even on a case-sensitive disk.
        if self.entries.iter().any(|e| e.0.eq_ignore_ascii_case(name)) {
            return Err("That name exists. Use Update or a new name.".into());
        }
        use std::io::Write;
        let mut file = crate::storage::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        file.write_all(bytes).map_err(|e| e.to_string())?;
        self.refresh(root, level);
        self.selected = self.entries.iter().position(|e| e.1 == path);
        self.remember_selection(root, level);
        Ok(())
    }
}
