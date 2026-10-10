//! Original TIM.SX music resource ordinals; the bank does not supply track titles.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicTrack {
    Track1 = 1001,
    Track2 = 1002,
    Track3 = 1003,
    Track4 = 1004,
    Track5 = 1005,
    Track6 = 1006,
    Track7 = 1007,
    Track8 = 1008,
    Track9 = 1009,
    Track10 = 1010,
    Track11 = 1011,
    Track12 = 1012,
    Track13 = 1013,
    Track14 = 1014,
    Track15 = 1015,
    Track16 = 1016,
}

impl MusicTrack {
    pub const fn id(self) -> u16 {
        self as u16
    }
}
