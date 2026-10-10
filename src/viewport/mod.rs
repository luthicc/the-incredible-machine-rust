//! Centered, aspect-preserving viewport shared by presentation and pointer
//! mapping. The image fills the window at any scale; to keep pixel art
//! sharp it is first enlarged by a whole `prescale` factor with nearest
//! pixels, then filtered down to the exact size ("sharp bilinear"), so every
//! source pixel stays the same size and only its edges are smoothed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    /// Logical window pixels per source pixel.
    pub scale: f32,
    pub scale_y: f32,
    pub origin: (f32, f32),
    pub source: (u32, u32),
    /// Whole-number enlargement before the filtered fit (at least 1).
    pub prescale: u32,
}
impl Viewport {
    /// Window size is logical; the fit and origin are aligned in physical pixels.
    pub fn fit(window: (f32, f32), dpi: f32, source: (u32, u32)) -> Self {
        assert!(dpi.is_finite() && dpi > 0.0);
        assert!(source.0 > 0 && source.1 > 0);
        let physical_scale = (window.0 * dpi / source.0 as f32)
            .min(window.1 * dpi / source.1 as f32)
            .max(0.0);
        // A scale within a hair of a whole number is drawn exactly at it.
        let rounded = physical_scale.round();
        let physical_scale = if rounded >= 1.0 && (physical_scale - rounded).abs() < 1e-3 {
            rounded
        } else {
            physical_scale
        };
        let prescale = (physical_scale.ceil() as u32).clamp(1, 8);
        let scale = physical_scale / dpi;
        Self {
            scale,
            scale_y: scale,
            origin: (
                ((window.0 - source.0 as f32 * scale) * dpi / 2.0).floor() / dpi,
                ((window.1 - source.1 as f32 * scale) * dpi / 2.0).floor() / dpi,
            ),
            source,
            prescale,
        }
    }
    pub fn configured(window: (f32, f32), dpi: f32, source: (u32, u32), stretch: bool) -> Self {
        let mut v = Self::fit(window, dpi, source);
        if stretch {
            v.scale = window.0 / source.0 as f32;
            v.scale_y = window.1 / source.1 as f32;
            v.origin = (0.0, 0.0);
            v.prescale = ((v.scale.max(v.scale_y) * dpi).ceil() as u32).clamp(1, 8);
        }
        v
    }
    /// With enlargement disabled, one source pixel is one physical pixel.
    pub fn presentation(
        window: (f32, f32),
        dpi: f32,
        source: (u32, u32),
        stretch: bool,
        pixel_scaling: bool,
    ) -> Self {
        if pixel_scaling {
            return Self::configured(window, dpi, source, stretch);
        }
        let scale = 1.0 / dpi;
        Self {
            scale,
            scale_y: scale,
            source,
            prescale: 1,
            origin: (
                ((window.0 * dpi - source.0 as f32) / 2.0).floor() / dpi,
                ((window.1 * dpi - source.1 as f32) / 2.0).floor() / dpi,
            ),
        }
    }
    /// Reject letterbox clicks; return native coordinates inside the image.
    pub fn source_point(self, point: (f32, f32)) -> Option<(f32, f32)> {
        if self.scale <= 0.0 || self.scale_y <= 0.0 {
            return None;
        }
        let x = (point.0 - self.origin.0) / self.scale;
        let y = (point.1 - self.origin.1) / self.scale_y;
        (x >= 0.0 && y >= 0.0 && x < self.source.0 as f32 && y < self.source.1 as f32)
            .then_some((x, y))
    }
}

/// A pinch or wheel zoom of one rectangular region of the source image,
/// such as the playfield. `center` is the source point shown at the middle
/// of the region; the view never leaves the region.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zoom {
    /// (left, top, right, bottom) in source pixels.
    pub region: (f32, f32, f32, f32),
    pub factor: f32,
    pub center: (f32, f32),
}
impl Zoom {
    pub const MAX: f32 = 4.0;
    pub fn new(region: (f32, f32, f32, f32)) -> Self {
        Self {
            region,
            factor: 1.0,
            center: Self::middle(region),
        }
    }
    fn middle(r: (f32, f32, f32, f32)) -> (f32, f32) {
        ((r.0 + r.2) / 2.0, (r.1 + r.3) / 2.0)
    }
    pub fn active(&self) -> bool {
        self.factor > 1.0
    }
    pub fn contains(&self, point: (f32, f32)) -> bool {
        let r = self.region;
        point.0 >= r.0 && point.1 >= r.1 && point.0 < r.2 && point.1 < r.3
    }
    /// The zoomed source point shown at unzoomed source point `point`.
    pub fn map(&self, point: (f32, f32)) -> (f32, f32) {
        if !self.active() || !self.contains(point) {
            return point;
        }
        let m = Self::middle(self.region);
        (
            self.center.0 + (point.0 - m.0) / self.factor,
            self.center.1 + (point.1 - m.1) / self.factor,
        )
    }
    /// Zoom to `factor`, keeping the content under unzoomed `point` fixed.
    pub fn zoom_at(&mut self, point: (f32, f32), factor: f32) {
        let shown = self.map(point);
        let m = Self::middle(self.region);
        self.factor = factor.clamp(1.0, Self::MAX);
        self.center = (
            shown.0 - (point.0 - m.0) / self.factor,
            shown.1 - (point.1 - m.1) / self.factor,
        );
        self.clamp();
    }
    /// Move the view with a drag of `delta` unzoomed source pixels.
    pub fn pan(&mut self, delta: (f32, f32)) {
        self.center.0 -= delta.0 / self.factor;
        self.center.1 -= delta.1 / self.factor;
        self.clamp();
    }
    /// Ease back towards the whole region; `keep` is the fraction of the
    /// remaining zoom to keep (0 snaps straight out).
    pub fn ease_out(&mut self, keep: f32) {
        let m = Self::middle(self.region);
        let keep = keep.clamp(0.0, 1.0);
        self.factor = 1.0 + (self.factor - 1.0) * keep;
        if self.factor < 1.002 {
            *self = Self::new(self.region);
            return;
        }
        self.center = (
            m.0 + (self.center.0 - m.0) * keep,
            m.1 + (self.center.1 - m.1) * keep,
        );
        self.clamp();
    }
    fn clamp(&mut self) {
        let r = self.region;
        let axis = |c: f32, lo: f32, hi: f32| {
            let half = (hi - lo) / 2.0 / self.factor;
            c.clamp(lo + half, hi - half)
        };
        self.center = (axis(self.center.0, r.0, r.2), axis(self.center.1, r.1, r.3));
    }
}

/// Enlarge an RGBA image by a whole factor with nearest pixels.
pub fn upscale(rgba: &[u8], width: u32, factor: u32) -> Vec<u8> {
    if factor <= 1 {
        return rgba.to_vec();
    }
    let (row_bytes, k) = (width as usize * 4, factor as usize);
    let mut out = Vec::with_capacity(rgba.len() * k * k);
    let mut row = Vec::with_capacity(row_bytes * k);
    for line in rgba.chunks_exact(row_bytes) {
        row.clear();
        for pixel in line.chunks_exact(4) {
            for _ in 0..k {
                row.extend_from_slice(pixel);
            }
        }
        for _ in 0..k {
            out.extend_from_slice(&row);
        }
    }
    out
}

#[cfg(test)]
mod tests;
