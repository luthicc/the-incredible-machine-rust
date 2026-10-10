//! Import only the original game archives; saves and options live beside
//! their imported copy. No executable, WAV file, or external unpacker runs.
use crate::{resources::Resources, sound};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
type Result<T> = std::result::Result<T, String>;
const FILES: [&str; 4] = [
    "RESOURCE.001",
    "RESOURCE.002",
    "RESOURCE.003",
    "RESOURCE.004",
];
const LIMIT: u64 = 32 * 1024 * 1024;

pub fn validate(root: &Path) -> Result<()> {
    let resources = Resources::load(root)?;
    for name in [
        "L1.LEV",
        "TITLE.GKC",
        "CREDITS.GKC",
        "SIERRA.SCR",
        "TIM.PAL",
    ] {
        resources.get(name)?;
    }
    sound::Bank::load(&resources)?;
    crate::font::Font::load(&resources)?;
    Ok(())
}

/// The file the user launched: the `.AppImage` itself when running from one,
/// otherwise the current executable.
pub fn launched_executable() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("APPIMAGE") {
        return Ok(PathBuf::from(path));
    }
    std::env::current_exe().map_err(|e| e.to_string())
}

pub fn storage_directory() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("TIM_PORTABLE_HOME") {
        return Ok(PathBuf::from(path));
    }
    let exe = launched_executable()?;
    let beside = exe.parent().ok_or("Executable has no parent directory")?;
    // Writing inside a macOS bundle would invalidate its signature.
    let in_app_bundle = cfg!(target_os = "macos") && beside.ends_with("Contents/MacOS");
    let probe = beside.join(format!(".tim-write-probe-{}", std::process::id()));
    if !in_app_bundle
        && fs::create_dir_all(beside.join("games")).is_ok()
        && fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&probe)
            .is_ok()
    {
        let _ = fs::remove_file(probe);
        return Ok(beside.into());
    }
    user_data_directory()
        .map(|base| base.join("tim-bevy"))
        .ok_or_else(|| "Cannot find a writable game-data directory".into())
}

fn user_data_directory() -> Option<PathBuf> {
    let home = || std::env::var_os("HOME").map(PathBuf::from);
    if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        home().map(|p| p.join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| home().map(|p| p.join(".local/share")))
    }
}

pub fn selected_game(storage: &Path) -> Option<PathBuf> {
    let relative = fs::read_to_string(storage.join("CURRENT_GAME")).ok()?;
    let relative = Path::new(relative.trim());
    if relative.is_absolute()
        || relative
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return None;
    }
    let root = storage.join(relative);
    validate(&root).ok()?;
    Some(root)
}

fn read_zip(source: &Path, stage: &Path) -> Result<()> {
    let file = fs::File::open(source).map_err(|e| e.to_string())?;
    let zip = zip::ZipArchive::new(file).map_err(|e| format!("Cannot open ZIP: {e}"))?;
    for (name, data) in extract_archives(zip)? {
        fs::write(stage.join(name), data).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Shared, bounded ZIP decoding for desktop imports and browser uploads.
pub fn archives_from_zip(bytes: &[u8]) -> Result<BTreeMap<String, Vec<u8>>> {
    let zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| format!("Cannot open ZIP: {e}"))?;
    extract_archives(zip)
}
fn extract_archives<R: Read + std::io::Seek>(
    mut zip: zip::ZipArchive<R>,
) -> Result<BTreeMap<String, Vec<u8>>> {
    if zip.len() > 10000 {
        return Err("Too many files in ZIP".into());
    }
    let mut groups: BTreeMap<PathBuf, BTreeMap<String, usize>> = BTreeMap::new();
    for i in 0..zip.len() {
        let file = zip.by_index(i).map_err(|e| e.to_string())?;
        let path = file.enclosed_name().ok_or("ZIP contains an unsafe path")?;
        if file.is_dir() {
            continue;
        }
        if file.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            return Err("ZIP contains a symbolic link".into());
        }
        let name = path
            .file_name()
            .ok_or("Missing ZIP filename")?
            .to_string_lossy()
            .to_ascii_uppercase();
        if !FILES.contains(&name.as_str()) {
            continue;
        }
        if file.size() > LIMIT {
            return Err(format!("{name} exceeds import limit"));
        }
        let group = groups
            .entry(path.parent().unwrap_or(Path::new("")).into())
            .or_default();
        if group.insert(name, i).is_some() {
            return Err("Duplicate game archive in ZIP".into());
        }
    }
    let candidates: Vec<_> = groups
        .values()
        .filter(|g| FILES.iter().all(|n| g.contains_key(*n)))
        .collect();
    if candidates.len() != 1 {
        return Err(if candidates.is_empty() {
            "ZIP must contain RESOURCE.001 through RESOURCE.004 in one folder"
        } else {
            "ZIP contains multiple games; choose a ZIP with just one"
        }
        .into());
    }
    let mut output = BTreeMap::new();
    for name in FILES {
        let mut file = zip
            .by_index(candidates[0][name])
            .map_err(|e| e.to_string())?;
        let mut data = Vec::new();
        (&mut file)
            .take(LIMIT + 1)
            .read_to_end(&mut data)
            .map_err(|e| format!("{name}: {e}"))?;
        if data.len() as u64 > LIMIT || data.len() as u64 != file.size() {
            return Err(format!("Invalid size for {name}"));
        }
        output.insert(name.to_string(), data);
    }
    Ok(output)
}

fn read_directory(source: &Path, stage: &Path) -> Result<()> {
    let mut files = BTreeMap::new();
    for entry in fs::read_dir(source).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_ascii_uppercase();
        if !FILES.contains(&name.as_str()) {
            continue;
        }
        if !entry.file_type().map_err(|e| e.to_string())?.is_file() {
            return Err("Game archives must be regular files".into());
        }
        if entry.metadata().map_err(|e| e.to_string())?.len() > LIMIT {
            return Err("Game archive exceeds import limit".into());
        }
        if files.insert(name, entry.path()).is_some() {
            return Err("Duplicate game archive".into());
        }
    }
    for name in FILES {
        let path = files.get(name).ok_or_else(|| format!("Missing {name}"))?;
        fs::copy(path, stage.join(name)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn import(source: &Path, storage: &Path) -> Result<PathBuf> {
    let games = storage.join("games");
    fs::create_dir_all(&games).map_err(|e| format!("Cannot create game directory: {e}"))?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let stage = games.join(format!(".import-{}-{stamp}", std::process::id()));
    fs::create_dir(&stage).map_err(|e| e.to_string())?;
    let result = (|| {
        if source.is_dir() {
            read_directory(source, &stage)?;
        } else {
            read_zip(source, &stage)?;
        }
        validate(&stage)?;
        let mut hash = 0xcbf29ce484222325u64;
        for name in FILES {
            for byte in fs::read(stage.join(name)).map_err(|e| e.to_string())? {
                hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
            }
        }
        let folder = format!("tim-{hash:016x}");
        let destination = games.join(&folder);
        if destination.exists() {
            // A reimport keeps the player's existing saves. Check the bytes,
            // not just the folder hash, before treating it as the same game.
            for name in FILES {
                if fs::read(destination.join(name)).map_err(|e| e.to_string())?
                    != fs::read(stage.join(name)).map_err(|e| e.to_string())?
                {
                    return Err("Imported game conflicts with existing data".into());
                }
            }
        } else {
            fs::rename(&stage, &destination).map_err(|e| e.to_string())?;
        }
        crate::default_solutions::seed(&destination)?;
        let marker = storage.join(format!(".CURRENT_GAME-{}-{stamp}", std::process::id()));
        let mut file = fs::File::create(&marker).map_err(|e| e.to_string())?;
        writeln!(file, "games/{folder}").map_err(|e| e.to_string())?;
        fs::rename(marker, storage.join("CURRENT_GAME")).map_err(|e| e.to_string())?;
        Ok(destination)
    })();
    let _ = fs::remove_dir_all(stage);
    result
}

/// Verify both synth and digital playback from only imported resources.
pub fn verify_audio(root: &Path) -> Result<Vec<(u16, f32)>> {
    let resources = Resources::load(root)?;
    let mut peaks = Vec::new();
    for id in [
        sound::SoundEffect::SpringRelease.id(),
        sound::MusicTrack::Track1.id(),
        sound::SoundEffect::PuzzleComplete.id(),
        sound::SoundEffect::BowlingBallRolling.id(),
    ] {
        let samples = sound::render_offline(&resources, id, 1.5)?;
        if samples.iter().any(|s| !s.is_finite()) {
            return Err(format!("Sound {id} produced invalid samples"));
        }
        let peak = samples.iter().fold(0.0f32, |peak, s| peak.max(s.abs()));
        if peak <= 0.01 {
            return Err(format!("Sound {id} is silent"));
        }
        peaks.push((id, peak));
    }
    Ok(peaks)
}

#[cfg(test)]
mod tests;
