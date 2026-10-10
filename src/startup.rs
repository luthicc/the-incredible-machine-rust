//! Sierra introduction frames from the table read by 1dff:04ce.
use crate::resources::{self, Resources, Screen};
use crate::sound::SoundEffect;
#[path = "tables/sierra_frames.rs"]
mod sierra_frames;

pub fn animation_frames(resources: &Resources) -> Result<Vec<Screen>, String> {
    let background = resources::screen(resources, "SIERRA.SCR", "SIERRA.PAL")?;
    let sprites = resources::sprites(resources, "SIERRA.BMP", "SIERRA.PAL")?;
    sierra_frames::FRAMES
        .iter()
        .map(|placements| {
            let mut screen = Screen {
                width: background.width,
                height: background.height,
                rgba: background.rgba.clone(),
            };
            // Original redraw region: x=448..639, y=415..479.
            for y in 415..480usize {
                for x in 448..640usize {
                    let offset = (y * screen.width as usize + x) * 4;
                    screen.rgba[offset..offset + 4]
                        .copy_from_slice(&background.rgba[offset..offset + 4]);
                }
            }
            for &(x, y, frame) in placements {
                let sprite = sprites
                    .get(frame as usize)
                    .ok_or("Invalid Sierra sprite frame")?;
                for sy in 0..sprite.height as i32 {
                    for sx in 0..sprite.width as i32 {
                        let dx = x as i32 + sx;
                        let dy = 415 + y as i32 + sy;
                        if dx < 448
                            || dx >= screen.width as i32
                            || dy < 415
                            || dy >= screen.height as i32
                        {
                            continue;
                        }
                        let source = ((sy as usize * sprite.width as usize) + sx as usize) * 4;
                        if sprite.rgba[source + 3] == 0 {
                            continue;
                        }
                        let target = (dy as usize * screen.width as usize + dx as usize) * 4;
                        screen.rgba[target..target + 4]
                            .copy_from_slice(&sprite.rgba[source..source + 4]);
                    }
                }
            }
            Ok(screen)
        })
        .collect()
}

/// Sounds the introduction plays as each table entry is drawn (1dff:05f1,
/// 1dff:0650): 20 when the walking sprite shows frame 0, 19 after the last.
pub fn sound_cues() -> Vec<Vec<SoundEffect>> {
    let last = sierra_frames::FRAMES.len() - 1;
    sierra_frames::FRAMES
        .iter()
        .enumerate()
        .map(|(i, placements)| {
            let mut cues = Vec::new();
            if placements[0].2 == 0 {
                cues.push(SoundEffect::BowlingBallRolling);
            }
            if i == last {
                cues.push(SoundEffect::PuzzleComplete);
            }
            cues
        })
        .collect()
}
