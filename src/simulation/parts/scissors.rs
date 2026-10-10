use super::*;
use crate::sound::SoundEffect;

native_part_state!(ScissorsSnap {
    Open = 0,
    Snapped = 1,
});

const HANDLE_EDGES: [u16; 4] = [0, 1, 5, 6];
const FLIPPED_HANDLE_EDGES: [u16; 4] = [1, 2, 4, 5];
const BLADE_EDGE: u16 = 3;
const FLIPPED_BLADE_EDGE: u16 = 7;

/// process_scissors_contact, 272c:3824: only handle edges activate the snap.
pub(super) fn process_scissors_contact(
    objects: &mut [Object],
    moving: usize,
    target: usize,
    edge: u16,
) -> CollisionResponse {
    let flipped = objects[target]
        .render_flags
        .contains(crate::machine::RenderFlag::FlipHorizontal);
    if if flipped {
        FLIPPED_HANDLE_EDGES.contains(&edge)
    } else {
        HANDLE_EDGES.contains(&edge)
    } {
        objects[target].power_state = 1;
    } else if edge
        == if flipped {
            FLIPPED_BLADE_EDGE
        } else {
            BLADE_EDGE
        }
        && objects[moving].definition == Def::Balloon
    {
        objects[moving].power_state = 1;
    }
    CollisionResponse::Ordinary
}

/// update_scissors_snap, Ghidra 272c:38fc: cut crossed ropes before closing.
pub(super) fn update_scissors_snap(objects: &mut Vec<Object>, index: usize) -> Result<(), String> {
    if objects[index].power_state != 0
        && ScissorsSnap::from_raw(objects[index].state) == Some(ScissorsSnap::Open)
    {
        rope_cut::split_ropes_at_object(objects, index)?;
        objects[index].state = ScissorsSnap::Snapped.raw();
        crate::sound::play_effect(SoundEffect::ScissorsSnap);
        objects[index].outline = crate::outline::vertices(&objects[index]);
        objects[index].outline_angles = objects[index]
            .outline
            .as_deref()
            .map(crate::outline::edge_angles)
            .unwrap_or_default();
    }
    Ok(())
}
