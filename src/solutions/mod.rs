//! Named per-puzzle snapshots and incremental, transactional hints.
mod solution_menu;
pub use solution_menu::SolutionMenu;

use crate::machine::UpdateFlag;
use crate::{
    definition::PartDefinition as Def,
    editor::Editor,
    machine::{Object, ObjectList, RenderFlag, RenderFlags},
};
use std::path::{Path, PathBuf};

pub fn directory(root: &Path, level: u16) -> PathBuf {
    root.join("SAVES")
        .join("SOLUTIONS")
        .join(format!("PUZZLE{level:02}"))
}

fn connection(o: &Object) -> bool {
    matches!(o.definition, Def::Rope | Def::Belt)
}
fn placed(o: &Object) -> bool {
    o.list != ObjectList::Inventory
}
fn locked(o: &Object) -> bool {
    o.update_flags & UpdateFlag::Locked.mask() != 0
}
// Socket occupancy and pulley direction are rebuilt from connections, not
// authored orientation. Keep the distributor's switch/orientation bit.
fn placement_state(o: &Object) -> u16 {
    match o.definition {
        Def::Pulley => 0,
        Def::PowerDistributor | Def::SolarPanel => o.saved_state & !3,
        Def::ElectricalOutlet => o.saved_state & !0x0c,
        _ => o.saved_state,
    }
}
fn geometry(a: &Object, b: &Object) -> bool {
    a.definition == b.definition
        && (
            a.x,
            a.y,
            a.width,
            a.height,
            placement_state(a),
            a.render_flags & RenderFlags::FACING_MASK.bits(),
        ) == (
            b.x,
            b.y,
            b.width,
            b.height,
            placement_state(b),
            b.render_flags & RenderFlags::FACING_MASK.bits(),
        )
}

/// Match exact placements first, then reuse unmatched placed parts before
/// taking from the bin. Every destination is consumed once; record ordering
/// may change when TIM serialization groups fixed/moving/inventory objects.
fn body_mapping(current: &[Object], target: &[Object]) -> Vec<Option<usize>> {
    let mut map = vec![None; target.len()];
    let mut used = vec![false; current.len()];
    for exact in [true, false] {
        for (t, wanted) in target
            .iter()
            .enumerate()
            .filter(|(_, o)| placed(o) && !connection(o))
        {
            if map[t].is_some() {
                continue;
            }
            let candidate = current
                .iter()
                .enumerate()
                .filter(|(i, o)| {
                    !used[*i] && o.definition == wanted.definition && locked(o) == locked(wanted)
                })
                .filter(|(_, o)| {
                    if exact {
                        placed(o) && geometry(o, wanted)
                    } else {
                        !locked(o)
                    }
                })
                .min_by_key(|(_, o)| {
                    (
                        u8::from(!placed(o)),
                        (i32::from(o.x) - i32::from(wanted.x)).abs()
                            + (i32::from(o.y) - i32::from(wanted.y)).abs(),
                    )
                });
            if let Some((i, _)) = candidate {
                map[t] = Some(i);
                used[i] = true;
            }
        }
    }
    map
}

/// Add or correct exactly one part/link, committing only after validation.
pub fn hint(editor: &mut Editor, target: &[Object]) -> Result<bool, String> {
    if editor.held.is_some() || editor.resizing.is_some() {
        return Err("Finish placing the held item first.".into());
    }
    let map = body_mapping(&editor.objects, target);
    let mut blocked = false;
    for (t, wanted) in target
        .iter()
        .enumerate()
        .filter(|(_, o)| placed(o) && !connection(o))
    {
        let Some(i) = map[t] else {
            return Err("Solution parts do not match this puzzle.".into());
        };
        if placed(&editor.objects[i]) && geometry(&editor.objects[i], wanted) {
            continue;
        }
        let mut probe = editor.clone();
        let o = &mut probe.objects[i];
        o.x = wanted.x;
        o.y = wanted.y;
        o.width = wanted.width;
        o.height = wanted.height;
        o.alt_size = wanted.alt_size;
        o.state = wanted.saved_state;
        o.saved_state = wanted.saved_state;
        o.render_flags = ((o.render_flags & !RenderFlags::FACING_MASK.bits())
            | (wanted.render_flags & RenderFlags::FACING_MASK.bits()))
        .into();
        o.rope_anchors = wanted.rope_anchors;
        o.belt_anchor = wanted.belt_anchor;
        o.refresh_belt_anchor();
        // Collision must use the hinted size/tilt, not the bin item's old
        // outline. In Puzzle 4 the opposite seesaw tilt hits the resting ball.
        o.outline = crate::outline::vertices(o);
        o.outline_angles = o
            .outline
            .as_deref()
            .map(crate::outline::edge_angles)
            .unwrap_or_default();
        if probe.collides(i) {
            blocked = true;
            continue;
        }
        probe.place(i);
        crate::machine::finalize_objects(&mut probe.objects)?;
        *editor = probe;
        return Ok(true);
    }
    if blocked {
        return Err("A hint position is blocked. Move the obstructing item and try again.".into());
    }
    let source = Editor::new(target.to_vec());
    for (t, wanted) in target
        .iter()
        .enumerate()
        .filter(|(_, o)| placed(o) && connection(o))
    {
        if wanted.definition == Def::Rope {
            let route = source.rope_route(t);
            if route.len() < 2 {
                continue;
            }
            let route: Vec<_> = route
                .into_iter()
                .map(|(p, s)| map[p].map(|i| (i, s)))
                .collect::<Option<_>>()
                .ok_or("Missing rope endpoint.")?;
            let same = |r: usize| {
                let existing = editor.rope_route(r);
                existing == route || existing.iter().copied().rev().collect::<Vec<_>>() == route
            };
            if editor
                .objects
                .iter()
                .enumerate()
                .any(|(i, o)| placed(o) && o.definition == Def::Rope && same(i))
            {
                continue;
            }
            if route.iter().any(|&(i, s)| {
                if editor.objects[i].definition == Def::Pulley {
                    editor.objects[i].pulley_connection.is_some()
                } else {
                    s > 1 || editor.objects[i].linked_objects[s as usize].is_some()
                }
            }) {
                return Err("A rope anchor is already occupied. Free it and try again.".into());
            }
            let i = editor
                .objects
                .iter()
                .position(|o| o.definition == Def::Rope && !placed(o))
                .ok_or("No spare rope in the bin.")?;
            let mut probe = editor.clone();
            probe.wire_rope_route(i, &route);
            crate::machine::finalize_objects(&mut probe.objects)?;
            *editor = probe;
        } else {
            let Some([Some(a), Some(b)]) = wanted.belt else {
                continue;
            };
            let (a, b) = (
                map[a].ok_or("Missing belt endpoint.")?,
                map[b].ok_or("Missing belt endpoint.")?,
            );
            if editor.objects.iter().any(|o| {
                placed(o)
                    && o.definition == Def::Belt
                    && (o.belt == Some([Some(a), Some(b)]) || o.belt == Some([Some(b), Some(a)]))
            }) {
                continue;
            }
            if [a, b].iter().any(|&i| {
                editor.objects[i]
                    .render_flags
                    .contains(RenderFlag::BeltConnected)
            }) {
                return Err("A belt wheel is already occupied. Free it and try again.".into());
            }
            let i = editor
                .objects
                .iter()
                .position(|o| o.definition == Def::Belt && !placed(o))
                .ok_or("No spare belt in the bin.")?;
            let mut probe = editor.clone();
            probe.objects[i].belt = Some([Some(a), Some(b)]);
            for end in [a, b] {
                probe.objects[end]
                    .render_flags
                    .insert(RenderFlag::BeltConnected);
            }
            probe.place(i);
            crate::machine::finalize_objects(&mut probe.objects)?;
            *editor = probe;
        }
        return Ok(true);
    }
    Ok(false)
}

/// A solution must contain this puzzle's inventory and authored fixtures.
pub fn validate(base: &[Object], target: &[Object]) -> Result<(), String> {
    let counts = |objects: &[Object]| {
        let mut counts = [0usize; 58];
        for o in objects {
            counts[o.definition.id() as usize] += 1;
        }
        counts
    };
    if counts(base) != counts(target) {
        return Err("This solution belongs to a different puzzle.".into());
    }
    let map = body_mapping(base, target);
    if target
        .iter()
        .enumerate()
        .any(|(i, o)| locked(o) && !connection(o) && map[i].is_none())
        || base.iter().filter(|o| locked(o)).count() != target.iter().filter(|o| locked(o)).count()
    {
        return Err("The solution changes this puzzle's fixed parts.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
