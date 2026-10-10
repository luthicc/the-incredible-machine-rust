use super::*;

pub struct Bank {
    pub(super) sounds: HashMap<u16, Sound>,
    pub(super) driver: Arc<Driver>,
}

impl Bank {
    pub fn load(resources: &Resources) -> Result<Self> {
        Ok(Self {
            sounds: parse_bank(resources.get("TIM.SX")?)?,
            driver: Arc::new(parse_driver(resources.get("SX.OVL")?)?),
        })
    }
}
