use super::*;

/// Persistent saved-machine simulation. Integer motion, directed contact
/// scans and moving rope response follow TIM's named native stages.
pub struct MachineSimulation {
    pub objects: Vec<Object>,
    pub(super) forces: [crate::forces::PartForce; 58],
    pub(super) trace_tick: u32,
    pub(super) initial_objects: Vec<Object>,
    /// Distinct unsupported updates actually dispatched since the last reset.
    pub unsupported_updates: Vec<Def>,
    pub(super) impulse_priority: ImpulsePriority,
}

impl MachineSimulation {
    /// Native run mode performs one simulation update before its first draw.
    /// This lets renderers distinguish the launch frame from a frame whose
    /// callback-derived state (such as a magnifying-glass focus) is current.
    pub fn has_stepped(&self) -> bool {
        self.trace_tick != 0
    }

    pub fn load(resources: &crate::resources::Resources) -> Result<Self, String> {
        Self::load_collection(resources, "TITLE.GKC")
    }

    pub fn load_collection(
        resources: &crate::resources::Resources,
        name: &str,
    ) -> Result<Self, String> {
        let header = crate::machine::collection_metadata(resources, name)?;
        let objects = crate::machine::collection_objects(resources, name)?;
        if std::env::var_os("TIM_TRACE_MOTION").is_some() {
            for (index, object) in objects.iter().enumerate() {
                if object.definition == Def::Seesaw
                    || object.definition == Def::Light
                    || object.definition == Def::Cannon
                    || object.definition == Def::PowerDistributor
                    || object.ropes.iter().any(Option::is_some)
                    || object.linked_objects.iter().any(Option::is_some)
                {
                    eprintln!(
                        "title object {index}: def={:?} list={:?} xy=({}, {}) state={} power={} links={:?} ropes={:?}",
                        object.definition,
                        object.list,
                        object.x,
                        object.y,
                        object.state,
                        object.power_state,
                        object.linked_objects,
                        object.ropes,
                    );
                }
            }
        }
        Ok(Self {
            unsupported_updates: Vec::new(),
            impulse_priority: ImpulsePriority::default(),
            initial_objects: objects.clone(),
            objects,
            forces: crate::forces::initialize(header.air, header.gravity),
            trace_tick: 0,
        })
    }
    /// A simulation of an edited layout, e.g. a puzzle after the player
    /// placed parts. Reset returns to this layout.
    pub fn from_objects(objects: Vec<Object>, forces: [crate::forces::PartForce; 58]) -> Self {
        Self {
            unsupported_updates: Vec::new(),
            impulse_priority: ImpulsePriority::default(),
            initial_objects: objects.clone(),
            objects,
            forces,
            trace_tick: 0,
        }
    }
    /// Restore the complete loaded graph, including anchors, connections,
    /// callback state, and spawned-object removal. Environment forces are retained.
    pub fn reset(&mut self) {
        self.objects.clone_from(&self.initial_objects);
        self.trace_tick = 0;
        self.unsupported_updates.clear();
        self.impulse_priority.0.clear();
    }

    /// simulate_object_update, 1000:1216: callback, integration, contact scan.
    pub(super) fn simulate_object_update(
        &mut self,
        index: usize,
        tick: u32,
    ) -> Result<(), SimulationError> {
        // FUN_1000_1216 checks +0x08 bit 0x2000 before invoking the
        // definition callback. If that callback hides an object (notably a
        // popped Balloon), this update still integrates and scans contacts.
        if self.objects[index]
            .render_flags
            .contains(crate::machine::RenderFlag::Hidden)
        {
            return Ok(());
        }
        let coverage = dispatch_definition_update(&mut self.objects, index, &self.forces, tick)?;
        if matches!(
            coverage,
            crate::callback_coverage::UpdateCoverage::Unimplemented(_)
        ) && !self
            .unsupported_updates
            .contains(&self.objects[index].definition)
        {
            self.unsupported_updates
                .push(self.objects[index].definition);
        }
        self.integrate_object_motion(index);
        // simulate_object_update (1000:1216) clears the response nibble
        // between integration and the contact scan; 1000:0556 itself keeps it.
        self.objects[index].update_flags &= !UpdateFlag::CONTACT_RESPONSE_MASK;
        crate::contact::update_object_contacts(&mut self.objects, index);
        rope_dynamics::update_moving_rope(
            &mut self.objects,
            index,
            &self.forces,
            &mut self.impulse_priority,
        );
        Ok(())
    }

    /// integrate_object_motion, 1000:2c93: one integer integration per tick.
    pub(super) fn integrate_object_motion(&mut self, index: usize) {
        let part = &mut self.objects[index];
        let force = self.forces[part.definition.id() as usize];
        part.motion.integrate(part.update_flags, force.acceleration);
        (part.x, part.y) = part.motion.pixels();
    }

    /// Collision-response pass of process_machine_simulation_update, 1000:0f86.
    pub(super) fn process_object_collision_response(&mut self, index: usize) {
        if self.objects[index].update_flags & UpdateFlag::CollisionResponseHandled.mask() != 0
            || self.objects[index]
                .render_flags
                .contains(crate::machine::RenderFlag::Hidden)
        {
            return;
        }
        let Some(contact) = self.objects[index].contact else {
            return;
        };
        let other = contact.target;
        let outline_edge = contact.outline_edge;
        let speed_limit = self.forces[self.objects[index].definition.id() as usize].speed_limit;
        let definition = self.objects[other].definition;
        let mut seesaw_accepted = false;
        if definition == Def::Seesaw {
            let center = self.objects[index].x.wrapping_add(
                (crate::spatial::native_object_extents(&self.objects[index]).0 >> 1) as i16,
            );
            if let Some(direction) =
                seesaw_contact_direction(&self.objects[other], outline_edge, center)
            {
                let impulse = self.objects[index].collision_impulse;
                if self.impulse_priority.accept(other, impulse) {
                    self.objects[other].power_state = direction;
                    self.objects[other].collision_impulse = impulse;
                    seesaw_accepted = true;
                }
            }
        }
        let response = match definition {
            Def::Bucket => {
                if check_bucket_contact_height(&self.objects[index], &self.objects[other]) {
                    CollisionResponse::Ordinary
                } else {
                    CollisionResponse::SuppressBounce
                }
            }
            Def::Dynamite if self.objects[index].definition == Def::Bullet => {
                // trigger_dynamite_on_impact, Ghidra 272c:1237.
                detonate_dynamite(&mut self.objects, other);
                CollisionResponse::Ordinary
            }
            Def::BoxingGlove | Def::Balloon | Def::Cat => {
                simple_target_contact(&mut self.objects, index, other, outline_edge)
            }
            Def::Seesaw if seesaw_accepted => {
                self.objects[index].contact = None;
                CollisionResponse::SuppressBounce
            }
            Def::Scissors => {
                process_scissors_contact(&mut self.objects, index, other, outline_edge)
            }
            Def::Flashlight => {
                activate_flashlight_contact(&mut self.objects, index, other, outline_edge)
            }
            Def::Detonator => trigger_detonator_contact(
                &mut self.objects,
                index,
                other,
                outline_edge,
                contact.angle,
            ),
            Def::Fish => advance_fish_state(&mut self.objects[other]),
            Def::Gear => gear_contact(&mut self.objects, index, other, outline_edge),
            Def::CatPoweredDrive => {
                activate_contacted_object(&mut self.objects[other]);
                CollisionResponse::Ordinary
            }
            Def::Monkey => startle_monkey(&mut self.objects[other], outline_edge),
            Def::Motor => {
                let power = self.objects[other].power_state;
                apply_motor_contact_thrust(&mut self.objects[index].motion, power, outline_edge)
            }
            Def::PowerDistributor => {
                adjust_power_distributor_level(&mut self.objects, index, other, contact.angle)
            }
            Def::Bellows => bellows_contact(&mut self.objects[other], outline_edge),
            Def::Trampoline
                if apply_trampoline_bounce(
                    &mut self.objects,
                    index,
                    other,
                    outline_edge,
                    speed_limit,
                ) =>
            {
                CollisionResponse::SuppressBounce
            }
            _ => CollisionResponse::Ordinary,
        };
        if response != CollisionResponse::Ordinary {
            return;
        }
        let angle = if matches!(contact.angle, 0 | 0x8000) {
            if !contact.flag_a {
                contact.angle.wrapping_add(0x1000)
            } else if !contact.flag_b {
                contact.angle.wrapping_sub(0x1000)
            } else {
                contact.angle
            }
        } else {
            contact.angle
        };
        let flags = self.objects[index].update_flags;
        let acceleration = self.forces[self.objects[index].definition.id() as usize].acceleration;
        if flags & UpdateFlag::SurfaceContact.mask() != 0 {
            if flags & UpdateFlag::Supported.mask() != 0 {
                let friction = if definition == Def::Motor && self.objects[other].power_state != 0 {
                    256
                } else {
                    part_friction::PART_FRICTION[self.objects[index].definition.id() as usize]
                        .max(part_friction::PART_FRICTION[definition.id() as usize])
                };
                resolve_contact_surface_response(
                    &mut self.objects[index].motion,
                    flags,
                    angle,
                    acceleration,
                    friction,
                    speed_limit,
                );
                self.objects[index].motion.position_y = (i32::from(self.objects[index].y) << 9)
                    + if acceleration >= 0 { 511 } else { 0 };
            } else {
                play_bowling_ball_rolling_sound(&self.objects[index]);
                let restitution = part_restitution::PART_RESTITUTION
                    [self.objects[index].definition.id() as usize]
                    .min(part_restitution::PART_RESTITUTION[definition.id() as usize]);
                reflect_velocity_off_contact_surface(
                    &mut self.objects[index].motion,
                    angle,
                    restitution,
                    speed_limit,
                );
                let p = &mut self.objects[index];
                p.motion.position_x =
                    (i32::from(p.x) << 9) + if p.motion.velocity_x >= 0 { 511 } else { 0 };
                p.motion.position_y =
                    (i32::from(p.y) << 9) + if acceleration >= 0 { 511 } else { 0 };
            }
        } else if flags & 4 != 0 {
            self.resolve_two_body_collision(index, other);
        }
    }

    /// resolve_two_body_collision, 1000:3201, velocity and minimum separation.
    pub(super) fn resolve_two_body_collision(&mut self, a: usize, b: usize) {
        play_bowling_ball_rolling_sound(&self.objects[a]);
        let pa = &self.objects[a];
        let pb = &self.objects[b];
        // Native compute_object_center_angle (1000:3da5) uses the runtime
        // frame extents at +0x44/+0x46, not the persistent sprite size at
        // +0x40/+0x42. A spent bullet keeps a 40x7 sprite but a 64x31 extent.
        let (aw, ah) = crate::spatial::native_object_extents(pa);
        let (bw, bh) = crate::spatial::native_object_extents(pb);
        let dx =
            pa.x.wrapping_add((aw >> 1) as i16)
                .wrapping_sub(pb.x.wrapping_add((bw >> 1) as i16));
        let dy =
            pb.y.wrapping_add((bh >> 1) as i16)
                .wrapping_sub(pa.y.wrapping_add((ah >> 1) as i16));
        let angle = crate::outline::vector_angle(dx, dy).wrapping_sub(0x4000);
        let (av, at) = rotate_vector_by_angle(pa.motion.velocity_x, pa.motion.velocity_y, angle);
        let (bv, bt) = rotate_vector_by_angle(pb.motion.velocity_x, pb.motion.velocity_y, angle);
        let am = i32::from(crate::forces::mass(pa.definition));
        let bm = i32::from(crate::forces::mass(pb.definition));
        if am + bm == 0 {
            return;
        }
        let original_av = av;
        let av =
            ((am * i32::from(av) + 2 * bm * i32::from(bv) - bm * i32::from(av)) / (am + bm)) as i16;
        let bv = ((2 * am * i32::from(original_av) + bm * i32::from(bv) - am * i32::from(bv))
            / (am + bm)) as i16;
        let (ax, ay) = rotate_vector_by_angle(av, at, angle.wrapping_neg());
        let (bx, by) = rotate_vector_by_angle(bv, bt, angle.wrapping_neg());
        self.objects[a].motion.velocity_x = ax >> 1;
        self.objects[a].motion.velocity_y = ay >> 1;
        self.objects[b].motion.velocity_x = bx >> 1;
        self.objects[b].motion.velocity_y = by >> 1;
        if (self.objects[a].motion.velocity_x.unsigned_abs() < 256
            && self.objects[b].motion.velocity_x.unsigned_abs() < 256)
            || self.objects[a].update_flags & UpdateFlag::Supported.mask() != 0
            || self.objects[a].flags & PropertyFlag::CarriedByBucket.mask() != 0
        {
            let left = dx < 0;
            let linked = self.objects[a].flags & PropertyFlag::CarriedByBucket.mask() != 0;
            self.objects[a].motion.velocity_x = if left {
                self.objects[a].motion.velocity_x.min(-512)
            } else {
                self.objects[a].motion.velocity_x.max(512)
            };
            if !linked {
                self.objects[b].motion.velocity_x = if left {
                    self.objects[b].motion.velocity_x.max(512)
                } else {
                    self.objects[b].motion.velocity_x.min(-512)
                };
            }
        }
        for i in [a, b] {
            self.objects[i].update_flags |= UpdateFlag::CollisionResponseHandled.mask();
            let force = self.forces[self.objects[i].definition.id() as usize];
            let p = &mut self.objects[i];
            p.motion.limit_velocity(force.speed_limit);
            p.motion.position_x =
                (i32::from(p.x) << 9) + if p.motion.velocity_x >= 0 { 511 } else { 0 };
            p.motion.position_y =
                (i32::from(p.y) << 9) + if force.acceleration >= 0 { 511 } else { 0 };
        }
    }

    /// A callback error stops the update; reset before retrying a damaged graph.
    /// One update, with run_game_loop's looping-sound counters around it.
    pub fn step(&mut self) -> Result<(), SimulationError> {
        crate::sound::begin_update();
        let result = self.process_machine_simulation_update();
        crate::sound::end_update();
        result
    }

    /// process_machine_simulation_update, 1000:0f86.
    pub fn process_machine_simulation_update(&mut self) -> Result<(), SimulationError> {
        let native_tick = self.trace_tick;
        self.trace_tick = self.trace_tick.wrapping_add(1);
        for object in &mut self.objects {
            object.older_position = object.previous_position;
            object.previous_position = (object.x, object.y);
            object.older_state = object.previous_state;
            object.previous_state = object.state;
        }
        for definition in dispatch_fixed_callbacks_with_priorities(
            &mut self.objects,
            &self.forces,
            native_tick,
            &self.impulse_priority.0,
        )? {
            if !self.unsupported_updates.contains(&definition) {
                self.unsupported_updates.push(definition);
            }
        }
        // Native 7b3e releases prior contact priorities after fixed updates.
        self.impulse_priority.0.clear();
        // 1000:0f86 accelerates the whole moving list before any integration.
        for index in crate::contact::native_list_order(&self.objects)
            .into_iter()
            .filter(|&index| self.objects[index].list == ObjectList::Moving)
            .collect::<Vec<_>>()
        {
            if !self.objects[index]
                .render_flags
                .contains(crate::machine::RenderFlag::Hidden)
            {
                let force = self.forces[self.objects[index].definition.id() as usize];
                self.objects[index]
                    .motion
                    .apply_vertical_acceleration(force.acceleration, force.speed_limit);
                self.objects[index].collision_impulse = self.objects[index]
                    .motion
                    .collision_impulse(self.objects[index].effective_mass);
            }
            // process_machine_simulation_update (1000:0f86) guards only
            // acceleration (1000:2c39) with Hidden. Mass and bucket-carried
            // state are reset on every moving record, including spent Dynamite.
            self.objects[index].effective_mass =
                crate::forces::mass(self.objects[index].definition);
            self.objects[index].flags &= !PropertyFlag::CarriedByBucket.mask();
        }
        let buckets: Vec<_> = acceleration_candidates(&self.objects)
            .into_iter()
            .filter(|&i| self.objects[i].definition == Def::Bucket)
            .collect();
        // The native moving-list pass follows its live linked list. A callback
        // such as update_balloon_pop (272c:018e) can append a RopeEnd
        // during this pass; when the new record sorts after the current node,
        // it receives its update immediately, but it missed the earlier
        // acceleration pass. Keep the initial list order and append newly
        // allocated moving records as they appear.
        let mut motion_updates = ordinary_motion_candidates(&self.objects);
        let mut cursor = 0;
        while cursor < motion_updates.len() {
            let index = motion_updates[cursor];
            let previous_len = self.objects.len();
            self.simulate_object_update(index, native_tick)?;
            cursor += 1;
            for new_index in previous_len..self.objects.len() {
                let object = &self.objects[new_index];
                if object.list == ObjectList::Moving
                    && object.definition != Def::Bucket
                    && !object
                        .render_flags
                        .contains(crate::machine::RenderFlag::Hidden)
                {
                    motion_updates.push(new_index);
                }
            }
        }
        for &bucket in &buckets {
            let contents = collect_bucket_load(&mut self.objects, bucket);
            accumulate_bucket_mass(&mut self.objects, bucket, &contents);
        }
        for &bucket in &buckets {
            collect_bucket_load(&mut self.objects, bucket);
            self.simulate_object_update(bucket, native_tick)?;
        }
        for &bucket in &buckets {
            let contents = collect_bucket_load(&mut self.objects, bucket);
            move_bucket_contents(&mut self.objects, bucket, &contents);
        }
        // 1000:0f86 resolves contacts only after every moving part is integrated.
        for index in acceleration_candidates(&self.objects) {
            self.process_object_collision_response(index);
        }
        if std::env::var_os("TIM_TRACE_MOTION").is_some() && self.trace_tick % 30 == 0 {
            for definition in [
                Def::BowlingBall,
                Def::Seesaw,
                Def::Light,
                Def::Cannon,
                Def::CannonProjectile,
                Def::Balloon,
            ] {
                if let Some(part) = self.objects.iter().find(|o| o.definition == definition) {
                    eprintln!(
                        "trace {:?}: xy=({},{}), v=({},{}), state={}, power={}",
                        definition,
                        part.x,
                        part.y,
                        part.motion.velocity_x,
                        part.motion.velocity_y,
                        part.state,
                        part.power_state
                    );
                }
            }
        }
        Ok(())
    }
}
