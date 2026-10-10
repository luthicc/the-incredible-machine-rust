//! Native files on desktop; browser-local files and save persistence on the web.
#[cfg(not(target_arch = "wasm32"))]
pub use std::fs::*;
#[cfg(not(target_arch = "wasm32"))]
pub fn exists(path: impl AsRef<std::path::Path>) -> bool {
    path.as_ref().exists()
}

#[cfg(target_arch = "wasm32")]
pub use browser::*;
#[cfg(target_arch = "wasm32")]
mod browser;
