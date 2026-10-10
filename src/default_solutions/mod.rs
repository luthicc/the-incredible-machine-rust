//! Saved layouts bundled as hint sources, never as active puzzle boards.
use crate::storage as fs;
use std::path::{Path, PathBuf};
pub const LAYOUTS: &[(u16, &str, &[u8])] = &[
    (1, "Default solution #1", include_bytes!("data/01-000.TIM")),
    (2, "Default solution #1", include_bytes!("data/02-001.TIM")),
    (3, "Default solution #1", include_bytes!("data/03-002.TIM")),
    (4, "Default solution #1", include_bytes!("data/04-003.TIM")),
    (5, "Default solution #1", include_bytes!("data/05-004.TIM")),
    (6, "Default solution #1", include_bytes!("data/06-005.TIM")),
    (7, "Default solution #1", include_bytes!("data/07-006.TIM")),
    (8, "Default solution #1", include_bytes!("data/08-007.TIM")),
    (9, "Default solution #1", include_bytes!("data/09-008.TIM")),
    (10, "Default solution #1", include_bytes!("data/10-009.TIM")),
    (11, "Default solution #1", include_bytes!("data/11-010.TIM")),
    (12, "Default solution #1", include_bytes!("data/12-011.TIM")),
    (13, "Default solution #1", include_bytes!("data/13-012.TIM")),
    (14, "Default solution #1", include_bytes!("data/14-013.TIM")),
    (15, "Default solution #1", include_bytes!("data/15-014.TIM")),
    (16, "Default solution #1", include_bytes!("data/16-015.TIM")),
    (17, "Default solution #1", include_bytes!("data/17-016.TIM")),
    (18, "Default solution #1", include_bytes!("data/18-017.TIM")),
    (19, "Default solution #1", include_bytes!("data/19-018.TIM")),
    (20, "Default solution #1", include_bytes!("data/20-019.TIM")),
    (21, "Default solution #1", include_bytes!("data/21-020.TIM")),
    (22, "Default solution #1", include_bytes!("data/22-021.TIM")),
    (23, "Default solution #1", include_bytes!("data/23-022.TIM")),
    (24, "Default solution #1", include_bytes!("data/24-023.TIM")),
    (25, "Default solution #1", include_bytes!("data/25-024.TIM")),
    (26, "Default solution #1", include_bytes!("data/26-025.TIM")),
    (27, "Default solution #1", include_bytes!("data/27-026.TIM")),
    (28, "Default solution #1", include_bytes!("data/28-027.TIM")),
    (29, "Default solution #1", include_bytes!("data/29-028.TIM")),
    (30, "Default solution #1", include_bytes!("data/30-029.TIM")),
    (31, "Default solution #1", include_bytes!("data/31-030.TIM")),
    (32, "Default solution #1", include_bytes!("data/32-031.TIM")),
    (33, "Default solution #1", include_bytes!("data/33-032.TIM")),
    (34, "Default solution #1", include_bytes!("data/34-033.TIM")),
    (35, "Default solution #1", include_bytes!("data/35-034.TIM")),
    (36, "Default solution #1", include_bytes!("data/36-035.TIM")),
    (37, "Default solution #1", include_bytes!("data/37-036.TIM")),
    (38, "Default solution #1", include_bytes!("data/38-037.TIM")),
    (39, "Default solution #1", include_bytes!("data/39-038.TIM")),
    (40, "Default solution #1", include_bytes!("data/40-039.TIM")),
    (41, "Default solution #1", include_bytes!("data/41-040.TIM")),
    (42, "Default solution #1", include_bytes!("data/42-041.TIM")),
    (43, "Default solution #1", include_bytes!("data/43-042.TIM")),
    (44, "Default solution #1", include_bytes!("data/44-043.TIM")),
    (45, "Default solution #1", include_bytes!("data/45-044.TIM")),
    (46, "Default solution #1", include_bytes!("data/46-045.TIM")),
    (47, "Default solution #1", include_bytes!("data/47-046.TIM")),
    (48, "Default solution #1", include_bytes!("data/48-047.TIM")),
    (49, "Default solution #1", include_bytes!("data/49-048.TIM")),
    (50, "Default solution #1", include_bytes!("data/50-049.TIM")),
    (51, "Default solution #1", include_bytes!("data/51-050.TIM")),
    (52, "Default solution #1", include_bytes!("data/52-051.TIM")),
    (53, "Default solution #1", include_bytes!("data/53-052.TIM")),
    (54, "Default solution #1", include_bytes!("data/54-053.TIM")),
    (55, "Default solution #1", include_bytes!("data/55-054.TIM")),
    (56, "Default solution #1", include_bytes!("data/56-055.TIM")),
    (57, "Default solution #1", include_bytes!("data/57-056.TIM")),
    (58, "Default solution #1", include_bytes!("data/58-057.TIM")),
    (59, "Default solution #1", include_bytes!("data/59-058.TIM")),
    (60, "Default solution #1", include_bytes!("data/60-059.TIM")),
    (61, "Default solution #1", include_bytes!("data/61-060.TIM")),
    (62, "Default solution #1", include_bytes!("data/62-061.TIM")),
    (63, "Default solution #1", include_bytes!("data/63-062.TIM")),
    (64, "Default solution #1", include_bytes!("data/64-063.TIM")),
    (65, "Default solution #1", include_bytes!("data/65-064.TIM")),
    (66, "Default solution #1", include_bytes!("data/66-065.TIM")),
    (67, "Default solution #1", include_bytes!("data/67-066.TIM")),
    (68, "Default solution #1", include_bytes!("data/68-067.TIM")),
    (69, "Default solution #1", include_bytes!("data/69-068.TIM")),
    (70, "Default solution #1", include_bytes!("data/70-069.TIM")),
    (71, "Default solution #1", include_bytes!("data/71-070.TIM")),
    (72, "Default solution #1", include_bytes!("data/72-071.TIM")),
    (73, "Default solution #1", include_bytes!("data/73-072.TIM")),
    (74, "Default solution #1", include_bytes!("data/74-073.TIM")),
    (75, "Default solution #1", include_bytes!("data/75-074.TIM")),
    (76, "Default solution #1", include_bytes!("data/76-075.TIM")),
    (77, "Default solution #1", include_bytes!("data/77-076.TIM")),
    (78, "Default solution #1", include_bytes!("data/78-077.TIM")),
    (79, "Default solution #1", include_bytes!("data/79-078.TIM")),
    (80, "Default solution #1", include_bytes!("data/80-079.TIM")),
    (81, "Default solution #1", include_bytes!("data/81-080.TIM")),
    (82, "Default solution #1", include_bytes!("data/82-081.TIM")),
    (83, "Default solution #1", include_bytes!("data/83-082.TIM")),
    (84, "Default solution #1", include_bytes!("data/84-083.TIM")),
    (85, "Default solution #1", include_bytes!("data/85-084.TIM")),
    (86, "Default solution #1", include_bytes!("data/86-085.TIM")),
    (87, "Default solution #1", include_bytes!("data/87-086.TIM")),
];

/// This namespace belongs to the bundled library. User variants use their own names.
pub fn is_default(path: &Path) -> bool {
    path.file_stem()
        .is_some_and(|name| name.to_string_lossy().starts_with("Default solution #"))
}

/// Keep older layouts as user variants when the bundled library is refreshed.
fn archive(path: &Path) -> Result<(), String> {
    let stem = path.file_stem().unwrap().to_string_lossy();
    let mut destination = path.with_file_name(format!("Previous {stem}.TIM"));
    let mut number = 1;
    while fs::exists(&destination) {
        destination = path.with_file_name(format!("Previous {stem} ({number}).TIM"));
        number += 1;
    }
    fs::rename(path, destination).map_err(|e| e.to_string())
}

pub fn seed(root: &Path) -> Result<(), String> {
    let library = root.join("SAVES/SOLUTIONS");
    let marker = library.join("DEFAULTS-v3");
    if fs::exists(&marker) {
        return Ok(());
    }
    for &(level, name, bytes) in LAYOUTS {
        let dir = crate::solutions::directory(root, level);
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let destination = dir.join(format!("{name}.TIM"));
        let selected = fs::read_to_string(dir.join("SELECTED")).unwrap_or_default();
        let paths: Vec<PathBuf> = fs::read_dir(&dir)
            .map_err(|e| e.to_string())?
            .flatten()
            .map(|entry| entry.path())
            .collect();
        let mut select_default = false;
        for path in paths {
            if !path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("tim"))
            {
                continue;
            }
            let legacy = path
                .file_stem()
                .is_some_and(|s| s == "Saved solution" || s == "Imported save");
            let identical = fs::read(&path).ok().as_deref() == Some(bytes);
            if path == destination && identical {
                continue;
            }
            if is_default(&path) || (legacy && identical) {
                select_default |= path
                    .file_name()
                    .is_some_and(|s| s.to_string_lossy() == selected.trim());
                if legacy && identical {
                    fs::remove_file(path).map_err(|e| e.to_string())?;
                } else {
                    archive(&path)?;
                }
            }
        }
        if fs::read(&destination).ok().as_deref() != Some(bytes) {
            fs::write(&destination, bytes).map_err(|e| e.to_string())?;
        }
        if select_default {
            fs::write(dir.join("SELECTED"), format!("{name}.TIM")).map_err(|e| e.to_string())?;
        }
    }
    fs::write(
        marker,
        b"Verified saved solutions installed; older defaults preserved as Previous variants.\n",
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests;
