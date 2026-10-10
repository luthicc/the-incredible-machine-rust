#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeCallback {
    pub segment: u16,
    pub offset: u16,
}
