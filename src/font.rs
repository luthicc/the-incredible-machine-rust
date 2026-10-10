//! Original proportional MEMOFNT8 font, loaded by 2c25:6e2d.
use crate::resources::{Resources, Screen, decompress};
pub struct Font {
    height: usize,
    spacing: i32,
    first: u8,
    count: usize,
    data: Vec<u8>,
}
impl Font {
    pub fn load(resources: &Resources) -> Result<Self, String> {
        let file = resources.get("MEMOFNT8.FNT")?;
        if file.get(..4) != Some(b"FNT:") || file.get(8) != Some(&255) {
            return Err("Unsupported font".into());
        }
        let data = decompress(file.get(16..).ok_or("Truncated font")?)?;
        let count = file[13] as usize;
        if data.len() < count * 3 {
            return Err("Truncated font tables".into());
        }
        Ok(Self {
            height: file[10] as usize,
            spacing: file[9] as i32,
            first: file[12],
            count,
            data,
        })
    }
    fn glyph(&self, c: u8) -> Option<(usize, usize)> {
        let index = c.checked_sub(self.first)? as usize;
        if index >= self.count {
            return None;
        }
        let offset = u16::from_le_bytes([self.data[index * 2], self.data[index * 2 + 1]]) as usize;
        Some((
            self.count * 3 + offset,
            self.data[self.count * 2 + index] as usize,
        ))
    }
    pub fn width(&self, text: &str) -> i32 {
        text.bytes()
            .map(|c| self.glyph(c).map_or(0, |(_, w)| w as i32))
            .sum()
    }
    pub fn draw(&self, dst: &mut Screen, text: &str, mut x: i32, y: i32, color: [u8; 4]) {
        for c in text.bytes() {
            if let Some((offset, width)) = self.glyph(c) {
                let stride = width.div_ceil(8);
                for sy in 0..self.height {
                    for sx in 0..width {
                        let Some(bits) = self.data.get(offset + sy * stride + sx / 8) else {
                            continue;
                        };
                        if bits & (0x80 >> (sx % 8)) == 0 {
                            continue;
                        }
                        let (dx, dy) = (x + sx as i32, y + sy as i32);
                        if dx < 0 || dy < 0 || dx >= dst.width as i32 || dy >= dst.height as i32 {
                            continue;
                        }
                        let target = (dy as usize * dst.width as usize + dx as usize) * 4;
                        dst.rgba[target..target + 4].copy_from_slice(&color);
                    }
                }
                x += width as i32;
            }
        }
    }
    pub fn wrapped(
        &self,
        dst: &mut Screen,
        text: &str,
        x: i32,
        mut y: i32,
        width: i32,
        color: [u8; 4],
    ) {
        let mut line = String::new();
        for word in text.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && self.width(&candidate) > width {
                self.draw(dst, &line, x, y, color);
                y += self.spacing;
                line = word.to_owned();
            } else {
                line = candidate;
            }
        }
        self.draw(dst, &line, x, y, color);
    }
}
