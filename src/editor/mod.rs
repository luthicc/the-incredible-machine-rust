//! Puzzle editing between runs: the parts bin, carrying a part, placing it,
//! flipping, resizing, returning it, and wiring belts and ropes.
//!
//! Follows run_machine_play_loop (1dff:18d2) and its helpers. Positions are
//! world coordinates; the default viewport is (-8, -8)
//! (initialize_game_play_state, 1dff:105e), so screen = world + 8.
mod handle;
pub use handle::Handle;
mod carry;
pub use carry::Carry;
mod held;
pub use held::Held;
mod resize;
pub use resize::Resize;
mod editor;
pub use editor::Editor;
mod drag_constraint;
use drag_constraint::DragConstraint;
mod bin_group;
pub use bin_group::BinGroup;
mod attach;
pub use attach::Attach;
mod attach_point;
pub use attach_point::AttachPoint;
mod handle_icon;
pub use handle_icon::HandleIcon;

use crate::definition::PartDefinition as Def;
use crate::machine::{Object, ObjectList, RenderFlag, RopeConnection};
use crate::machine::{PropertyFlag, UpdateFlag};

pub const VIEWPORT: (i16, i16) = (-8, -8);

/// Parts-bin column (draw_parts_list_panel, 24de:101d): five 52-pixel slots
/// starting at y 100, hit-tested by the UI elements from create_ui_element_list.
pub const BIN_SLOTS: [(i16, i16); 5] = [
    (100, 0x90),
    (0x91, 0xc4),
    (0xc5, 0xf8),
    (0xf9, 0x12c),
    (0x12d, 0x160),
];
pub const BIN_X: (i16, i16) = (0x240, 0x278);
/// draw_parts_list_panel stops drawing groups at y 0x135.
const BIN_ROW: i16 = 0x34;

/// Per-definition resize bounds from the definition table (+0x0a max width,
/// +0x0c max height, +0x0e min width, +0x10 min height). An empty range
/// means the axis cannot be resized.
fn size_limits(definition: Def) -> ((i16, i16), (i16, i16)) {
    match definition {
        Def::BrickWall => ((16, 240), (16, 240)),
        Def::Incline => ((16, 64), (240, 0)),
        Def::Motor => ((32, 96), (240, 0)),
        Def::StraightPipe
        | Def::WoodWall
        | Def::Reserved51
        | Def::Reserved52
        | Def::Reserved53
        | Def::Reserved54 => ((32, 240), (32, 240)),
        _ => ((240, 0), (240, 0)),
    }
}

/// Parts-bin sort key (definition table +0x1e), used by
/// insert_object_into_sorted_list for the 0800 list.
fn bin_order(definition: Def) -> i16 {
    const KEYS: [i16; 58] = [
        0, 40, 44, 6, 5, 12, 37, 17, 10, 1, 15, 34, 35, 13, 11, 38, 7, 33, 28, 29, 50, 20, 32, 16,
        23, 26, 21, 18, 3, 27, 25, 39, 50, 51, 52, 8, 30, 19, 22, 9, 14, 50, 36, 2, 4, 31, 41, 42,
        43, 50, 24, 50, 50, 50, 50, 50, 50, 50,
    ];
    KEYS[definition.id() as usize]
}

pub fn screen_to_world(p: (i16, i16)) -> (i16, i16) {
    (p.0 + VIEWPORT.0, p.1 + VIEWPORT.1)
}

/// How near (in pixels, either axis) the cursor must be to an anchor.
const ATTACH_REACH: i16 = 6;

/// Screen rectangle, half-open: (x0, y0, x1, y1).
pub type Rect = (i16, i16, i16, i16);

/// Extra pixels around a handle icon that still select it.
const HANDLE_PAD: i16 = 2;
/// Extra pixels around a handle icon that keep its part highlighted.
const HANDLE_OVERSHOOT: i16 = 10;
/// Frames the cursor may rest in the safe triangle (about a third of a second).
const SAFE_TRIANGLE_FRAMES: u8 = 20;

fn grow((x0, y0, x1, y1): Rect, by: i16) -> Rect {
    (x0 - by, y0 - by, x1 + by, y1 + by)
}

fn contains((x0, y0, x1, y1): Rect, (x, y): (i16, i16)) -> bool {
    x0 <= x && x < x1 && y0 <= y && y < y1
}

/// Whether `p` lies in the convex hull of `apex` and `rect`: the union of
/// the rectangle and the triangles from the apex to each of its edges.
fn in_hull(apex: (i16, i16), rect: Rect, p: (i16, i16)) -> bool {
    if contains(rect, p) {
        return true;
    }
    let (x0, y0, x1, y1) = rect;
    let corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
    (0..4).any(|i| in_triangle(apex, corners[i], corners[(i + 1) % 4], p))
}

fn in_triangle(a: (i16, i16), b: (i16, i16), c: (i16, i16), p: (i16, i16)) -> bool {
    let cross = |o: (i16, i16), u: (i16, i16), v: (i16, i16)| {
        let (o, u, v) = (
            (i32::from(o.0), i32::from(o.1)),
            (i32::from(u.0), i32::from(u.1)),
            (i32::from(v.0), i32::from(v.1)),
        );
        (u.0 - o.0) * (v.1 - o.1) - (u.1 - o.1) * (v.0 - o.0)
    };
    let (d1, d2, d3) = (cross(a, b, p), cross(b, c, p), cross(c, a, p));
    let negative = d1 < 0 || d2 < 0 || d3 < 0;
    let positive = d1 > 0 || d2 > 0 || d3 > 0;
    !(negative && positive)
}

fn refresh_outline(object: &mut Object) {
    object.outline = crate::outline::vertices(object);
    object.outline_angles = object
        .outline
        .as_deref()
        .map(crate::outline::edge_angles)
        .unwrap_or_default();
}

fn refresh_outlet(outlet: &mut Object) {
    crate::machine::refresh_outlet_state(outlet);
    outlet.saved_state = outlet.state;
    refresh_outline(outlet);
}

fn outlet_socket_mask(definition: Def) -> u16 {
    match definition {
        Def::PowerDistributor | Def::SolarPanel => 0x03,
        Def::ElectricalOutlet => 0x0c,
        _ => 0,
    }
}

/// Add plug sprites for devices within socket reach. This only touches the
/// editor's render copy; saved links are still made by dropping the part.
fn preview_nearby_plugs(objects: &mut [Object], moving_outlet: Option<usize>) {
    let mut occupied: Vec<[bool; 2]> = objects
        .iter()
        .map(|outlet| {
            let first_bit = if outlet.definition == Def::ElectricalOutlet {
                2
            } else {
                0
            };
            [
                outlet.linked_objects[4].is_some() || outlet.state & (1 << first_bit) != 0,
                outlet.linked_objects[5].is_some() || outlet.state & (1 << (first_bit + 1)) != 0,
            ]
        })
        .collect();
    if let Some(outlet) = moving_outlet {
        let mask = outlet_socket_mask(objects[outlet].definition);
        occupied[outlet] = [
            objects[outlet].linked_objects[4].is_some(),
            objects[outlet].linked_objects[5].is_some(),
        ];
        objects[outlet].state &= !mask;
        for socket in 0..2 {
            if occupied[outlet][socket] {
                let bit = if objects[outlet].definition == Def::ElectricalOutlet {
                    socket + 2
                } else {
                    socket
                };
                objects[outlet].state |= 1 << bit;
            }
        }
    }
    for device in 0..objects.len() {
        if objects[device].flags & PropertyFlag::ElectricalPlug.mask() == 0
            || objects[device].list == ObjectList::Inventory
            || objects[device].linked_objects[4].is_some()
        {
            continue;
        }
        let outlet = crate::spatial::nearby_parts(objects, device, (-8, 8), (-8, 8))
            .into_iter()
            .map(|contact| contact.index)
            .find(|&candidate| {
                objects[candidate].list == ObjectList::Fixed
                    && objects[candidate].flags & PropertyFlag::ElectricalOutlet.mask() != 0
                    && !(occupied[candidate][0] && occupied[candidate][1])
            });
        let Some(outlet) = outlet else { continue };
        let socket = usize::from(occupied[outlet][0]);
        occupied[outlet][socket] = true;
        let bit = if objects[outlet].definition == Def::ElectricalOutlet {
            socket + 2
        } else {
            socket
        };
        objects[outlet].state |= 1 << bit;
        refresh_outline(&mut objects[outlet]);
    }
}

fn move_to(object: &mut Object, x: i16, y: i16) {
    object.x = x;
    object.y = y;
    object.motion = crate::motion::Motion::at_pixels(x, y);
    object.previous_position = (x, y);
    object.older_position = (x, y);
    object.saved_position = (x, y);
}

fn rope_length(objects: &[Object], rope: &crate::machine::RopeConnection) -> Option<f32> {
    crate::machine::rope_segments(objects, rope)
        .ok()
        .map(|segments| {
            segments
                .iter()
                .map(|(a, b)| {
                    let dx = (b.0 - a.0) as f32;
                    let dy = (b.1 - a.1) as f32;
                    dx.hypot(dy)
                })
                .sum()
        })
}

fn belt_distance(a: &Object, b: &Object) -> f32 {
    let p = a.belt_origin();
    let q = b.belt_origin();
    ((q.0 - p.0) as f32).hypot((q.1 - p.1) as f32)
}

#[cfg(test)]
mod tests;

mod inventory;

mod placement;

mod selection;

mod connections;

mod sizing;
