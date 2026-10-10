use super::*;

#[derive(Resource)]
pub(super) struct Launcher {
    pub(super) storage: PathBuf,
    pub(super) status: String,
    pub(super) job: Option<Mutex<mpsc::Receiver<Result<Option<PathBuf>, String>>>>,
}
