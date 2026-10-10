//! Part-definition IDs from TIM's 58-entry definition table.
//! Names derived from behavior or artwork are descriptive labels, not claims
//! about the original source names. Unused definition slots retain numeric suffixes.
//! Descriptive identities are grounded in evidence/part-atlas.png and native
//! callbacks; they are not claimed to reproduce the original internal symbols.
use std::fmt;

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PartDefinition {
    BowlingBall = 0,
    BrickWall = 1,
    Incline = 2,
    Seesaw = 3,
    Balloon = 4,
    Motor = 5,
    CatPoweredDrive = 6,
    Pulley = 7,
    Belt = 8,
    Basketball = 9,
    Rope = 10,
    BirdCage = 11,
    Cat = 12,
    JackInTheBox = 13,
    Gear = 14,
    Fish = 15,
    Bellows = 16,
    Bucket = 17,
    Cannon = 18,
    Dynamite = 19,
    Bullet = 20,
    PowerDistributor = 21,
    Detonator = 22,
    EyeHook = 23,
    Fan = 24,
    Flashlight = 25,
    ElectricalOutlet = 26,
    Gun = 27,
    Baseball = 28,
    Light = 29,
    MagnifyingGlass = 30,
    Monkey = 31,
    JackOLantern = 32,
    HeartBalloon = 33,
    ChristmasTree = 34,
    BoxingGlove = 35,
    Rocket = 36,
    Scissors = 37,
    SolarPanel = 38,
    Trampoline = 39,
    Windmill = 40,
    Explosion = 41,
    Mouse = 42,
    CannonProjectile = 43,
    TennisBall = 44,
    Candle = 45,
    StraightPipe = 46,
    CurvedPipe = 47,
    WoodWall = 48,
    RopeEnd = 49,
    ElectricalMotor = 50,
    /// Unused slots: no PART51..PART54 artwork, zero-sized allocation defaults,
    /// identical definition records and no-op native callbacks.
    Reserved51 = 51,
    Reserved52 = 52,
    Reserved53 = 53,
    Reserved54 = 54,
    TitleThe = 55,
    TitleSign = 56,
    CreditsProducer = 57,
}

impl PartDefinition {
    pub const COUNT: usize = 58;

    pub const fn id(self) -> u16 {
        self as u16
    }
}

impl TryFrom<u16> for PartDefinition {
    type Error = String;

    fn try_from(id: u16) -> Result<Self, Self::Error> {
        Ok(match id {
            0 => Self::BowlingBall,
            1 => Self::BrickWall,
            2 => Self::Incline,
            3 => Self::Seesaw,
            4 => Self::Balloon,
            5 => Self::Motor,
            6 => Self::CatPoweredDrive,
            7 => Self::Pulley,
            8 => Self::Belt,
            9 => Self::Basketball,
            10 => Self::Rope,
            11 => Self::BirdCage,
            12 => Self::Cat,
            13 => Self::JackInTheBox,
            14 => Self::Gear,
            15 => Self::Fish,
            16 => Self::Bellows,
            17 => Self::Bucket,
            18 => Self::Cannon,
            19 => Self::Dynamite,
            20 => Self::Bullet,
            21 => Self::PowerDistributor,
            22 => Self::Detonator,
            23 => Self::EyeHook,
            24 => Self::Fan,
            25 => Self::Flashlight,
            26 => Self::ElectricalOutlet,
            27 => Self::Gun,
            28 => Self::Baseball,
            29 => Self::Light,
            30 => Self::MagnifyingGlass,
            31 => Self::Monkey,
            32 => Self::JackOLantern,
            33 => Self::HeartBalloon,
            34 => Self::ChristmasTree,
            35 => Self::BoxingGlove,
            36 => Self::Rocket,
            37 => Self::Scissors,
            38 => Self::SolarPanel,
            39 => Self::Trampoline,
            40 => Self::Windmill,
            41 => Self::Explosion,
            42 => Self::Mouse,
            43 => Self::CannonProjectile,
            44 => Self::TennisBall,
            45 => Self::Candle,
            46 => Self::StraightPipe,
            47 => Self::CurvedPipe,
            48 => Self::WoodWall,
            49 => Self::RopeEnd,
            50 => Self::ElectricalMotor,
            51 => Self::Reserved51,
            52 => Self::Reserved52,
            53 => Self::Reserved53,
            54 => Self::Reserved54,
            55 => Self::TitleThe,
            56 => Self::TitleSign,
            57 => Self::CreditsProducer,
            _ => return Err(format!("Invalid TIM part definition {id}")),
        })
    }
}

impl fmt::Display for PartDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id())
    }
}

#[cfg(test)]
mod tests;
