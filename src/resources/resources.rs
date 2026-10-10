use super::*;

pub struct Resources {
    pub(super) entries: BTreeMap<String, Vec<u8>>,
}

impl Resources {
    pub fn load(root: &Path) -> Result<Self> {
        let mut entries = BTreeMap::new();
        for archive in 1..=4 {
            let file = root.join(format!("RESOURCE.{archive:03}"));
            let bytes = fs::read(&file).map_err(|e| format!("{}: {e}", file.display()))?;
            let mut offset = 0;
            while offset < bytes.len() {
                let name = bytes
                    .get(offset..offset + 13)
                    .ok_or("Truncated archive name")?;
                let end = name.iter().position(|b| *b == 0).unwrap_or(13);
                let name = String::from_utf8(name[..end].to_vec()).map_err(|e| e.to_string())?;
                let size = word(&bytes, offset + 13)? as usize;
                offset += 17;
                let payload = bytes
                    .get(offset..offset + size)
                    .ok_or("Truncated archive entry")?;
                entries.insert(name, payload.to_vec());
                offset += size;
            }
        }
        Ok(Self { entries })
    }
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    pub fn get(&self, name: &str) -> Result<&[u8]> {
        self.entries
            .get(name)
            .map(Vec::as_slice)
            .ok_or_else(|| format!("Missing resource {name}"))
    }
}
