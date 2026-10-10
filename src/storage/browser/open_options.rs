use super::*;

#[derive(Default)]
pub struct OpenOptions {
    pub(super) create_new: bool,
}

impl OpenOptions {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn write(&mut self, _: bool) -> &mut Self {
        self
    }
    pub fn create_new(&mut self, b: bool) -> &mut Self {
        self.create_new = b;
        self
    }
    pub fn open(&self, path: impl AsRef<Path>) -> io::Result<File> {
        let path = path.as_ref();
        if self.create_new && exists(path) {
            return Err(io::Error::new(io::ErrorKind::AlreadyExists, "File exists"));
        }
        write(path, [])?;
        Ok(File {
            path: path.into(),
            data: Vec::new(),
        })
    }
}
