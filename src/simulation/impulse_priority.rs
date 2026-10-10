/// Native 1000:7b6f: reject an equal/weaker impulse already recorded for
/// the target; retain accepted entries in descending signed impulse order.
#[derive(Clone, Debug, Default)]
pub(super) struct ImpulsePriority(pub(super) Vec<(usize, i32)>);

impl ImpulsePriority {
    pub(super) fn accept(&mut self, target: usize, impulse: i32) -> bool {
        let key = ((impulse >> 16) as i16, impulse as u16);
        if self.0.iter().any(|&(part, old)| {
            let old_key = ((old >> 16) as i16, old as u16);
            part == target && key <= old_key
        }) {
            return false;
        }
        let at = self
            .0
            .iter()
            .position(|&(_, old)| {
                let old_key = ((old >> 16) as i16, old as u16);
                old_key <= key
            })
            .unwrap_or(self.0.len());
        self.0.insert(at, (target, impulse));
        true
    }
}
