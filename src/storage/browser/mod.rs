mod file;
pub use file::File;
mod open_options;
pub use open_options::OpenOptions;
mod dir_entry;
pub use dir_entry::DirEntry;
mod files;
use files::Files;
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    io,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};
use wasm_bindgen::prelude::*;
#[wasm_bindgen(inline_js = r#"
export function restore_files() {
const files = {};
for (let i=0; i<localStorage.length; i++) {
    const key=localStorage.key(i);
    if(key.startsWith('tim-web:')) { try { files[key.slice(8)]=JSON.parse(localStorage.getItem(key)); } catch {} }
}
return JSON.stringify(files);
}
export function persist_file(path,bytes) { localStorage.setItem('tim-web:'+path,JSON.stringify(Array.from(bytes))); }
export function delete_file(path) { localStorage.removeItem('tim-web:'+path); }
"#)]
extern "C" {
    #[wasm_bindgen(catch)]
    fn restore_files() -> Result<String, JsValue>;
    #[wasm_bindgen(catch)]
    fn persist_file(path: &str, bytes: &[u8]) -> Result<(), JsValue>;
    #[wasm_bindgen(catch)]
    fn delete_file(path: &str) -> Result<(), JsValue>;
}
static FILES: OnceLock<Mutex<Files>> = OnceLock::new();
fn files() -> &'static Mutex<Files> {
    FILES.get_or_init(Default::default)
}
fn error(e: JsValue) -> io::Error {
    io::Error::other(format!("Browser save storage: {e:?}"))
}
fn parents(f: &mut Files, p: &Path) {
    for parent in p.ancestors() {
        f.dirs.insert(parent.into());
    }
}
pub fn restore() -> io::Result<()> {
    let saved: BTreeMap<PathBuf, Vec<u8>> =
        serde_json::from_str(&restore_files().map_err(error)?).map_err(io::Error::other)?;
    let mut f = files().lock().unwrap();
    for (path, bytes) in saved {
        if let Some(parent) = path.parent() {
            parents(&mut f, parent);
        }
        f.files.insert(path, bytes);
    }
    Ok(())
}
pub fn exists(path: impl AsRef<Path>) -> bool {
    let f = files().lock().unwrap();
    f.files.contains_key(path.as_ref()) || f.dirs.contains(path.as_ref())
}
pub fn create_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    parents(&mut files().lock().unwrap(), path.as_ref());
    Ok(())
}
pub fn read(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    files()
        .lock()
        .unwrap()
        .files
        .get(path.as_ref())
        .cloned()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, path.as_ref().display().to_string()))
}
pub fn read_to_string(path: impl AsRef<Path>) -> io::Result<String> {
    String::from_utf8(read(path)?).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
pub fn write(path: impl AsRef<Path>, bytes: impl AsRef<[u8]>) -> io::Result<()> {
    let path = path.as_ref();
    // The original archives stay in memory; only player files go to save storage.
    if !path
        .file_name()
        .is_some_and(|n| n.to_string_lossy().starts_with("RESOURCE."))
    {
        persist_file(&path.to_string_lossy(), bytes.as_ref()).map_err(error)?;
    }
    let mut f = files().lock().unwrap();
    if let Some(parent) = path.parent() {
        parents(&mut f, parent);
    }
    f.files.insert(path.into(), bytes.as_ref().to_vec());
    Ok(())
}
pub fn remove_file(path: impl AsRef<Path>) -> io::Result<()> {
    delete_file(&path.as_ref().to_string_lossy()).map_err(error)?;
    files()
        .lock()
        .unwrap()
        .files
        .remove(path.as_ref())
        .map(|_| ())
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "File missing"))
}
pub fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<u64> {
    let bytes = read(from)?;
    write(to, &bytes)?;
    Ok(bytes.len() as u64)
}
pub fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
    write(to, read(&from)?)?;
    remove_file(from)
}
pub fn read_dir(path: impl AsRef<Path>) -> io::Result<std::vec::IntoIter<io::Result<DirEntry>>> {
    let f = files().lock().unwrap();
    let path = path.as_ref();
    if !f.dirs.contains(path) {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Directory missing"));
    }
    let paths: BTreeSet<_> = f
        .files
        .keys()
        .chain(f.dirs.iter())
        .filter(|p| p.parent() == Some(path))
        .cloned()
        .collect();
    Ok(paths
        .into_iter()
        .map(|path| Ok(DirEntry { path }))
        .collect::<Vec<_>>()
        .into_iter())
}
