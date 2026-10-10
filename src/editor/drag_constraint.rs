#[derive(Clone, Copy, Debug)]
pub(super) enum DragConstraint {
    Rope {
        owner: usize,
        slot: usize,
    },
    Belt {
        owner: usize,
        other: usize,
        max_distance: f32,
    },
}
