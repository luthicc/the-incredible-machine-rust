use super::*;

#[derive(Default)]
pub(super) struct Files {
    pub(super) files: BTreeMap<PathBuf, Vec<u8>>,
    pub(super) dirs: BTreeSet<PathBuf>,
}
