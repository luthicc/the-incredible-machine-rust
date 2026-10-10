/// Two-pole Butterworth low-pass standing in for the AdLib / Sound Blaster
/// output filter; without it the bright patches (rectified and pulse
/// waveforms with feedback) sound harsher than on the hardware.
pub(super) struct Lowpass {
    pub(super) b: [f32; 3],
    pub(super) a: [f32; 2],
    pub(super) x: [f32; 2],
    pub(super) y: [f32; 2],
}

impl Lowpass {
    const CUTOFF: f32 = 8_000.0;

    pub(super) fn new(rate: u32) -> Self {
        let w = std::f32::consts::TAU * Self::CUTOFF / rate as f32;
        let alpha = w.sin() / std::f32::consts::SQRT_2;
        let a0 = 1.0 + alpha;
        let b1 = (1.0 - w.cos()) / a0;
        Self {
            b: [b1 / 2.0, b1, b1 / 2.0],
            a: [-2.0 * w.cos() / a0, (1.0 - alpha) / a0],
            x: [0.0; 2],
            y: [0.0; 2],
        }
    }

    pub(super) fn process(&mut self, input: f32) -> f32 {
        let output = self.b[0] * input + self.b[1] * self.x[0] + self.b[2] * self.x[1]
            - self.a[0] * self.y[0]
            - self.a[1] * self.y[1];
        self.x = [input, self.x[0]];
        self.y = [output, self.y[0]];
        output
    }
}
