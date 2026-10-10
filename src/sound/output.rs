use super::*;

pub(super) struct Output {
    pub(super) engine: Engine,
    pub(super) commands: mpsc::Receiver<Command>,
    pub(super) buffer: Vec<f32>,
    pub(super) position: usize,
}

impl Iterator for Output {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.position == self.buffer.len() {
            while let Ok(command) = self.commands.try_recv() {
                self.engine.command(command);
            }
            self.engine.render(&mut self.buffer);
            self.position = 0;
        }
        self.position += 1;
        Some(self.buffer[self.position - 1])
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl rodio::Source for Output {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        1
    }
    fn sample_rate(&self) -> u32 {
        self.engine.rate
    }
    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}
