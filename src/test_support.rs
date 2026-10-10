use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

/// Resource-backed regressions use a local DOS game installation supplied by
/// the developer. The original archives are not part of the source project.
pub fn game_data_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        std::env::var_os("TIM_GAME_DATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".."))
    })
}
