use super::*;

#[derive(Clone)]
pub(super) enum Body {
    /// AdLib channel tracks: (start, end) of each stream after its header.
    Midi {
        data: Arc<[u8]>,
        tracks: Vec<(usize, usize)>,
    },
    /// Unsigned 8-bit PCM.
    Digital { rate: u32, samples: Arc<[u8]> },
}
