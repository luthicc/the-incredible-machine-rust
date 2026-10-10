use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Object {
    pub list: ObjectList,
    /// Native +0x06 motion/dispatch word; named bits are in UpdateFlag.
    pub update_flags: u16,
    pub contact: Option<ContactRecord>,
    pub definition: PartDefinition,
    pub motion: crate::motion::Motion,
    pub effective_mass: i16, // Native +0x3a. Connected load accumulation remains pending.
    pub collision_impulse: i32, // Native signed pair +0x3c/+0x3e.
    pub saved_position: (i16, i16), // Native reset position; retained for reset/editor rules.
    pub previous_position: (i16, i16), // Native +22/+24.
    pub older_position: (i16, i16), // Native +26/+28.
    pub saved_state: u16,    // Native +90, original switch position.
    pub outline: Option<Vec<(u8, u8)>>,
    pub outline_angles: Vec<u16>,
    pub state: u16,
    pub previous_state: u16,
    pub older_state: u16,
    pub power_state: i16,
    pub callback_timer: i16,
    pub pending_light_power: u8,
    pub activation_timer: i16, // Native +0x96, distinct from +0x9c.
    pub belt_anchor: (u8, u8),
    pub belt_size: i16,
    pub rope_anchors: [(u8, u8); 2],
    pub belt: Option<[Option<usize>; 2]>,
    pub ropes: [Option<RopeConnection>; 2],
    pub linked_objects: [Option<usize>; 6],
    /// Native +0x78: bucket contents chain, rebuilt by collect_bucket_load.
    pub next_part: Option<usize>,
    pub pulley_connection: Option<usize>,
    /// Persistent property word; named bits are in PropertyFlag.
    pub flags: u16,
    pub render_flags: RenderFlags,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
    /// Runtime rectangle extent after callbacks such as wall trimming. DOS
    /// keeps the sprite width/height fields intact and updates +0x44/+0x46.
    pub extent_override: Option<(u16, u16)>,
    /// Native +0x50/+0x52, the saved secondary size (tiled extents).
    pub alt_size: (u16, u16),
}
impl Object {
    /// Wheel corner in world coordinates. The generator initializer stores
    /// (56, 18) even when its composite sprite is mirrored.
    pub fn belt_origin(&self) -> (i32, i32) {
        let (mut x, y) = (i32::from(self.belt_anchor.0), i32::from(self.belt_anchor.1));
        if self.definition == PartDefinition::ElectricalOutlet
            && self.render_flags.contains(RenderFlag::FlipHorizontal)
        {
            x = i32::from(self.width) - 1 - x - i32::from(self.belt_size);
        }
        (i32::from(self.x) + x, i32::from(self.y) + y)
    }

    /// build_unknown50_outline (272c:1435): this motor's wheel changes
    /// sides with its facing. Rebuild after loading as well as after a flip.
    pub fn refresh_belt_anchor(&mut self) {
        if self.definition == PartDefinition::ElectricalMotor {
            self.belt_anchor = (
                if !self.render_flags.contains(RenderFlag::FlipHorizontal) {
                    0
                } else {
                    0x25
                },
                0x0d,
            );
            self.belt_size = 0x12;
        }
    }

    /// Local ignition attachment from the original geometry initializers.
    /// Parts without a native ignition anchor return None. Facing is read at use time.
    pub fn ignition_anchor(&self) -> Option<(u8, u8)> {
        use PartDefinition as Def;
        let flipped = self.render_flags.contains(RenderFlag::FlipHorizontal);
        match self.definition {
            Def::Candle => Some((15, 2)),                              // 272c:0950
            Def::Cannon => Some((if flipped { 62 } else { 1 }, 3)),    // 272c:0b88
            Def::Dynamite => Some((if flipped { 1 } else { 45 }, 15)), // 272c:1261
            Def::Rocket => Some((11, 60)),                             // 272c:3737
            _ => None,
        }
    }
}
