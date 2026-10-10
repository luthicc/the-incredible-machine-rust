//! Coverage refers to whole native callback bodies, not visible animation alone.
use crate::callback_catalogue::{CallbackSlot, NativeCallback, native_callback};
use crate::definition::PartDefinition as Part;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateCoverage {
    Absent,
    NativeNoOp,
    Partial(NativeCallback),
    Unimplemented(NativeCallback),
}

pub fn update_coverage(part: Part) -> UpdateCoverage {
    let Some(callback) = native_callback(part, CallbackSlot::Update) else {
        return UpdateCoverage::Absent;
    };
    if callback
        == (NativeCallback {
            segment: 0x1000,
            offset: 0x02a1,
        })
    {
        return UpdateCoverage::NativeNoOp;
    }
    match part {
        Part::JackInTheBox
        | Part::ElectricalMotor
        | Part::Seesaw
        | Part::Windmill
        | Part::Gear
        | Part::Motor
        | Part::Cannon
        | Part::PowerDistributor
        | Part::Fan
        | Part::Light
        | Part::Rocket
        | Part::Gun
        | Part::Bullet
        | Part::Mouse
        | Part::Cat
        | Part::CreditsProducer
        | Part::Scissors
        | Part::Fish
        | Part::SolarPanel
        | Part::Flashlight
        | Part::Candle
        | Part::Dynamite
        | Part::Detonator
        | Part::Explosion
        | Part::Balloon
        | Part::BoxingGlove
        | Part::MagnifyingGlass
        | Part::CatPoweredDrive
        | Part::ElectricalOutlet
        | Part::Monkey
        | Part::Trampoline
        | Part::Bellows => UpdateCoverage::Partial(callback),
        _ => UpdateCoverage::Unimplemented(callback),
    }
}

#[cfg(test)]
mod tests;
