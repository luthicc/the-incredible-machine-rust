use super::*;

/// A belt or rope anchor on a placed part, in screen coordinates: free (a
/// click attaches one from the bin) or in use by `connection` (a click
/// takes that end off and puts the belt or rope back in hand).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttachPoint {
    pub part: usize,
    pub kind: Attach,
    pub at: (i16, i16),
    pub connection: Option<usize>,
}
