pub(super) struct Reader<'a> {
    pub(super) bytes: &'a [u8],
    pub(super) offset: usize,
}
impl Reader<'_> {
    pub(super) fn word(&mut self) -> Result<u16, String> {
        let value = u16::from_le_bytes(
            self.bytes
                .get(self.offset..self.offset + 2)
                .ok_or("Truncated machine word")?
                .try_into()
                .unwrap(),
        );
        self.offset += 2;
        Ok(value)
    }
    pub(super) fn byte(&mut self) -> Result<u8, String> {
        let value = *self
            .bytes
            .get(self.offset)
            .ok_or("Truncated machine byte")?;
        self.offset += 1;
        Ok(value)
    }
    pub(super) fn reference(&mut self, count: usize) -> Result<Option<usize>, String> {
        let index = self.word()?;
        if index == 0xffff {
            return Ok(None);
        }
        if index as usize >= count {
            return Err(format!(
                "Saved reference {index} exceeds object count {count}"
            ));
        }
        Ok(Some(index as usize))
    }
}
