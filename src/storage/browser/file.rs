use super::*;

pub struct File {
    pub(super) path: PathBuf,
    pub(super) data: Vec<u8>,
}

impl io::Write for File {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.data.extend_from_slice(bytes);
        write(&self.path, &self.data)?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
