//! Launch control-panel artwork, following 1dff:3642 and 24de:000c.
//! Text, puzzle preview, and input dispatch remain separate.
use crate::resources::{self, Resources, Screen};
fn blit(dst: &mut Screen, src: &Screen, x: i32, y: i32, clip: (i32, i32, i32, i32)) {
    for sy in 0..src.height as i32 {
        for sx in 0..src.width as i32 {
            let (dx, dy) = (x + sx, y + sy);
            if dx < clip.0 || dy < clip.1 || dx >= clip.2 || dy >= clip.3 {
                continue;
            }
            let source = ((sy as usize * src.width as usize) + sx as usize) * 4;
            if src.rgba[source + 3] == 0 {
                continue;
            }
            let target = (dy as usize * dst.width as usize + dx as usize) * 4;
            dst.rgba[target..target + 4].copy_from_slice(&src.rgba[source..source + 4]);
        }
    }
}
pub fn artwork(resources: &Resources) -> Result<Screen, String> {
    puzzle_artwork(resources, 1)
}

/// Control panel for puzzle `level` (resource `L{level}.LEV`).
pub fn puzzle_artwork(resources: &Resources, level: u16) -> Result<Screen, String> {
    let name = level_name(level);
    let cp = resources::sprites(resources, "CP.BMP", "TIM.PAL")?;
    let border = resources::sprites(resources, "GP_BORD.BMP", "TIM.PAL")?;
    if cp.len() <= 58 || border.len() <= 42 {
        return Err("Incomplete control panel sprites".into());
    }
    let mut dst = Screen {
        width: 640,
        height: 480,
        rgba: vec![0; 640 * 480 * 4],
    };
    for pixel in dst.rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[192, 192, 192, 255]);
    }
    let all = (0, 0, 640, 480);
    // Main panel: clipped original 128 by 64 interior tiles.
    for y in (32..344).step_by(64) {
        for x in (32..544).step_by(128) {
            blit(&mut dst, &border[42], x, y, (32, 32, 544, 344));
        }
    }
    for x in (32..544).step_by(8) {
        blit(&mut dst, &border[18], x, 28, all);
        blit(&mut dst, &border[19], x, 344, all);
    }
    for y in (32..344).step_by(8) {
        blit(&mut dst, &border[16], 28, y, all);
        blit(&mut dst, &border[17], 544, y, all);
    }
    for (frame, x, y) in [(12, 25, 25), (13, 527, 25), (14, 25, 327), (15, 527, 327)] {
        blit(&mut dst, &border[frame], x, y, all);
    }
    let preview = crate::machine::level_preview(resources, &name)?;
    for y in 72..232usize {
        for x in 272..528usize {
            let offset = (y * 640 + x) * 4;
            dst.rgba[offset..offset + 4].copy_from_slice(&preview.rgba[offset..offset + 4]);
        }
    }
    // Two inset control/text panels from 1dff:3642 and 1dff:37fd.
    for (x, y, w, h) in [(44, 66, 208, 265), (272, 255, 256, 76)] {
        for ty in (0..h).step_by(64) {
            for tx in (0..w).step_by(64) {
                blit(&mut dst, &cp[58], x + tx, y + ty, (x, y, x + w, y + h));
            }
        }
        for ty in ((y + 19)..(y + h)).step_by(8) {
            blit(&mut dst, &cp[14], x - 2, ty, all);
        }
        for tx in ((x + 16)..(x + w)).step_by(8) {
            blit(&mut dst, &cp[15], tx, y + h - 4, all);
        }
        for (frame, px, py) in [
            (10, x - 7, y - 4),
            (11, x + w - 16, y - 4),
            (12, x - 7, y + h - 16),
            (13, x + w - 19, y + h - 14),
        ] {
            blit(&mut dst, &cp[frame], px, py, all);
        }
    }
    // Unpressed controls and labels proven by the original drawing routines.
    for (frame, x, y) in [
        (16, 58, 91),
        (18, 216, 96),
        (31, 188, 92),
        (41, 109, 133),
        (27, 57, 134),
        (37, 88, 93),
        (39, 88, 111),
        (20, 110, 96),
        (7, 65, 200),
        (9, 61, 229),
        (8, 65, 276),
        (9, 61, 305),
        (3, 83, 66),
        (5, 100, 178),
        (4, 91, 254),
    ] {
        blit(&mut dst, &cp[frame], x, y, all);
    }
    let header = crate::machine::collection_metadata(resources, &name)?;
    let title = &header.title;
    let description = &header.description;
    let air = header.air as i32;
    let gravity = header.gravity as i32;
    blit(&mut dst, &cp[6], 61 + gravity * 160 / 512, 224, all);
    blit(&mut dst, &cp[6], 61 + air * 160 / 128, 300, all);
    let font = crate::font::Font::load(resources)?;
    // Title plaque and centered embossed text, 24de:0224.
    blit(&mut dst, &cp[0], 60, 39, all);
    for x in (84..480).step_by(8) {
        blit(&mut dst, &cp[1], x, 41, all);
    }
    blit(&mut dst, &cp[2], 480, 39, all);
    let heading = format!("Puzzle {level}: {title}");
    let x = 60 + (444 - font.width(&heading)) / 2;
    font.draw(&mut dst, &heading, x - 1, 45, [252, 252, 252, 255]);
    font.draw(&mut dst, &heading, x, 44, [168, 0, 168, 255]);
    font.wrapped(&mut dst, description, 276, 260, 248, [0, 0, 0, 255]);
    Ok(dst)
}

pub fn level_name(level: u16) -> String {
    format!("L{level}.LEV")
}

/// Number of consecutive puzzles L1.LEV, L2.LEV, ... present in the archives.
pub fn level_count(resources: &Resources) -> u16 {
    (1..)
        .take_while(|&level| resources.get(&level_name(level)).is_ok())
        .count() as u16
}
