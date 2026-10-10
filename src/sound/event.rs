pub(super) enum Event {
    NoteOff(u8, u8),
    NoteOn(u8, u8, u8),
    Controller(u8, u8, u8),
    Program(u8, u8),
    Pitch(u8, u16),
}
