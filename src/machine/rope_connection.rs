#[derive(Clone, Debug, PartialEq)]
pub struct RopeConnection {
    pub endpoints: [Option<usize>; 2],
    pub attachment_slots: [u8; 2],
    /// Routed chord total retained for cut bookkeeping. Moving response
    /// uses the Rope owner's native integer end allowances instead.
    pub rest_length: f32,
    /// Native attachment +0x0e: slack events consumed before a tilt.
    pub slack_count: i16,
}
