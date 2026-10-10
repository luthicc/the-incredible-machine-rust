//! TIM.SX effect IDs. Names describe their observed callback uses, rather
//! than claiming to recover the original game's source-level names.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundEffect {
    MotorRunning = 1,
    Monkey = 2,
    /// Shared by the jack popping, trampoline bounce, and glove spring.
    SpringRelease = 3,
    /// Present in TIM.SX, with no static native part-sound caller (1000:83ab xrefs).
    UnusedEffect4 = 4,
    /// Present in TIM.SX, with no static native part-sound caller (1000:83ab xrefs).
    UnusedEffect5 = 5,
    CannonFire = 6,
    CatStartled = 7,
    Explosion = 8,
    FanRunning = 9,
    FishBowlBreak = 10,
    Gunshot = 11,
    ElectricalDriveRunning = 12,
    CatPoweredDrive = 13,
    BalloonPop = 14,
    RocketLaunch = 15,
    ScissorsSnap = 16,
    SwitchToggle = 17,
    /// Shared by bellows motion and seesaw tilt.
    MechanismMovement = 18,
    PuzzleComplete = 19,
    BowlingBallRolling = 20,
}

impl SoundEffect {
    pub const ALL: [Self; 20] = [
        Self::MotorRunning,
        Self::Monkey,
        Self::SpringRelease,
        Self::UnusedEffect4,
        Self::UnusedEffect5,
        Self::CannonFire,
        Self::CatStartled,
        Self::Explosion,
        Self::FanRunning,
        Self::FishBowlBreak,
        Self::Gunshot,
        Self::ElectricalDriveRunning,
        Self::CatPoweredDrive,
        Self::BalloonPop,
        Self::RocketLaunch,
        Self::ScissorsSnap,
        Self::SwitchToggle,
        Self::MechanismMovement,
        Self::PuzzleComplete,
        Self::BowlingBallRolling,
    ];

    pub const fn id(self) -> u16 {
        self as u16
    }
}
