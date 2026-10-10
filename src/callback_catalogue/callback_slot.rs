#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallbackSlot {
    CollisionResponse,
    Update,
    InitializeGeometry,
    RopeContact,
}
