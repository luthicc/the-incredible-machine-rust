#[derive(Clone, Debug, PartialEq)]
pub enum Carry {
    /// An ordinary part following the cursor with a grab offset.
    Part { grab: (i16, i16) },
    /// connect_held_belt_to_target (1dff:1e94): the first end, once chosen.
    Belt { first: Option<usize> },
    /// connect_held_rope_to_target (1dff:1f90): the route so far, from the
    /// first end through any pulleys.
    Rope { route: Vec<(usize, u8)> },
}
