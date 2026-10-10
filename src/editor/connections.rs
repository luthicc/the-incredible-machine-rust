use super::*;

impl Editor {
    /// Connected spans that should be colored while a part is being dragged.
    /// `Some(slot)` identifies a rope record; `None` identifies a belt.
    pub fn held_connection_states(&self) -> Vec<(usize, Option<usize>, bool)> {
        let Some(Held {
            index,
            carry: Carry::Part { .. },
        }) = &self.held
        else {
            return Vec::new();
        };
        self.drag_constraints
            .iter()
            .map(|constraint| match *constraint {
                DragConstraint::Rope { owner, slot, .. } => (
                    owner,
                    Some(slot),
                    self.drag_constraint_fits(*index, constraint),
                ),
                DragConstraint::Belt { owner, .. } => {
                    (owner, None, self.drag_constraint_fits(*index, constraint))
                }
            })
            .collect()
    }

    pub(super) fn break_overstretched_connections(&mut self, index: usize) {
        // Ropes follow their anchors while editing. Rebuild their lengths at
        // launch; moving or flipping an endpoint never unwires its other end.
        let broken_belts: Vec<usize> = self
            .drag_constraints
            .iter()
            .filter_map(|constraint| match *constraint {
                DragConstraint::Belt {
                    owner,
                    other,
                    max_distance,
                } if belt_distance(&self.objects[index], &self.objects[other])
                    > max_distance + 0.5 =>
                {
                    Some(owner)
                }
                _ => None,
            })
            .collect();
        for belt in broken_belts {
            if let Some(ends) = self.objects[belt].belt {
                for end in ends.into_iter().flatten() {
                    self.objects[end]
                        .render_flags
                        .remove(crate::machine::RenderFlag::BeltConnected);
                }
            }
            self.stow(belt);
        }
    }

    /// Temporary native rope route, including the current cursor direction.
    pub fn preview_rope(&self, objects: &mut [Object], cursor: (i16, i16)) {
        let Some(Held {
            carry: Carry::Rope { route },
            ..
        }) = &self.held
        else {
            return;
        };
        for (n, &(part, slot)) in route.iter().enumerate() {
            let prev = n.checked_sub(1).map(|p| route[p].0);
            let next = route.get(n + 1).map(|p| p.0);
            if objects[part].definition == Def::Pulley {
                objects[part].linked_objects[0] = next;
                objects[part].linked_objects[1] = prev;
            } else {
                objects[part].linked_objects[slot as usize] = next;
            }
        }
        // 1dff:2136/2149 rotates the last pulley with the raw cursor;
        // 1dff:2140 selects its outgoing tangent (anchor 1) for the guide.
        crate::machine::refresh_pulleys(objects, Some(screen_to_world(cursor)));
    }

    /// rebind_held_object_attachment (1000:50a6): plug a part into the first
    /// fixed outlet within 8 pixels that is already its outlet or has a free
    /// socket (wall outlet, generator, solar panel).
    pub(super) fn plug_in(&mut self, device: usize) {
        let current = self.objects[device].linked_objects[4];
        let outlet = crate::spatial::nearby_parts(&self.objects, device, (-8, 8), (-8, 8))
            .into_iter()
            .map(|c| c.index)
            .find(|&o| {
                let part = &self.objects[o];
                part.list == ObjectList::Fixed
                    && part.flags & PropertyFlag::ElectricalOutlet.mask() != 0
                    && (Some(o) == current
                        || part.linked_objects[4].is_none()
                        || part.linked_objects[5].is_none())
            });
        if outlet == current {
            return;
        }
        self.unplug(device);
        let Some(outlet) = outlet else { return };
        let socket = if self.objects[outlet].linked_objects[4].is_none() {
            4
        } else {
            5
        };
        self.objects[outlet].linked_objects[socket] = Some(device);
        self.objects[device].linked_objects[4] = Some(outlet);
        refresh_outlet(&mut self.objects[outlet]);
    }

    /// detach_object_attachments (1000:51cb): pull a part's plug, or every
    /// plug out of an outlet.
    pub(super) fn unplug(&mut self, index: usize) {
        if self.objects[index].flags & PropertyFlag::ElectricalOutlet.mask() == 0 {
            let Some(outlet) = self.objects[index].linked_objects[4].take() else {
                return;
            };
            if self.objects[outlet].flags & PropertyFlag::ElectricalOutlet.mask() == 0 {
                return;
            }
            for socket in 4..6 {
                if self.objects[outlet].linked_objects[socket] == Some(index) {
                    self.objects[outlet].linked_objects[socket] = None;
                }
            }
            refresh_outlet(&mut self.objects[outlet]);
        } else {
            for socket in 4..6 {
                if let Some(device) = self.objects[index].linked_objects[socket].take() {
                    self.objects[device].linked_objects[4] = None;
                }
            }
            refresh_outlet(&mut self.objects[index]);
        }
    }

    /// compute_object_connection_flags (1000:4830 helper): resize axes from
    /// render flags 0x80/0x100, flips from update flags 0x400/0x200 when the
    /// flipped part would not collide.
    pub fn connection_flags(&self, index: usize) -> u16 {
        let o = &self.objects[index];
        if matches!(o.definition, Def::Belt | Def::Rope) {
            return 0;
        }
        let mut flags = u16::from(
            o.render_flags
                .contains(crate::machine::RenderFlag::ResizeHorizontal),
        );
        if o.render_flags
            .contains(crate::machine::RenderFlag::ResizeVertical)
        {
            flags |= 2;
        }
        for (bit, mode, mask) in [(4, 1, 0x400), (8, 2, 0x200)] {
            if o.update_flags & mask != 0 {
                let mut probe = self.clone();
                probe.flip(index, mode);
                if !probe.collides(index) {
                    flags |= bit;
                }
            }
        }
        flags
    }

    /// Belt ends: render flag 1 accepts a belt, flag 2 means it has one
    /// (are_belt_endpoints_adjacent, 1000:4b8f).
    pub(super) fn belt_target(&self, cursor: (i16, i16)) -> Option<usize> {
        self.part_at(cursor).filter(|&i| {
            let o = &self.objects[i];
            o.render_flags.contains(RenderFlag::BeltAttachable)
                && !o.render_flags.contains(RenderFlag::BeltConnected)
        })
    }

    pub(super) fn valid_belt_target(
        &self,
        cursor: (i16, i16),
        first: Option<usize>,
    ) -> Option<usize> {
        let target = self.belt_target(cursor)?;
        if let Some(a) = first {
            let (pa, pb) = (&self.objects[a], &self.objects[target]);
            if target == a || (pa.x - pb.x).abs() >= 0x8d || (pa.y - pb.y).abs() >= 0x8d {
                return None;
            }
        }
        Some(target)
    }

    pub(super) fn valid_rope_target(
        &self,
        cursor: (i16, i16),
        route: &[(usize, u8)],
    ) -> Option<(usize, u8)> {
        let target = self.rope_target(cursor, route.last().map(|&(i, _)| i))?;
        if route.iter().any(|&(i, _)| i == target.0)
            || (route.is_empty() && self.objects[target.0].definition == Def::Pulley)
        {
            return None;
        }
        Some(target)
    }

    /// Exact anchor of a connection that clicking would accept, in world coordinates.
    pub fn connection_preview_target(&self, cursor: (i16, i16)) -> Option<(i32, i32)> {
        match &self.held.as_ref()?.carry {
            Carry::Belt { first } => {
                let target = self.valid_belt_target(cursor, *first)?;
                let o = &self.objects[target];
                let (x, y) = o.belt_origin();
                let half = i32::from(o.belt_size / 2);
                Some((x + half, y + half))
            }
            Carry::Rope { route } => {
                let (target, slot) = self.valid_rope_target(cursor, route)?;
                Some(crate::machine::rope_point(
                    &self.objects[target],
                    slot as usize,
                ))
            }
            Carry::Part { .. } => None,
        }
    }

    /// Rope ends: render flag 4 accepts a rope, flag 8 offers two slots and
    /// the nearer anchor wins (select_rope_endpoint_under_cursor, 1000:45b8).
    pub(super) fn rope_target(
        &self,
        cursor: (i16, i16),
        exclude: Option<usize>,
    ) -> Option<(usize, u8)> {
        let i = self.part_at(cursor)?;
        if Some(i) == exclude {
            return None;
        }
        let o = &self.objects[i];
        if !o.render_flags.contains(RenderFlag::RopeAttachable) {
            return None;
        }
        let slot = if !o.render_flags.contains(RenderFlag::TwoRopeAnchors) {
            0
        } else {
            let left = o.x - VIEWPORT.0;
            let d0 = (cursor.0 - (left + i16::from(o.rope_anchors[0].0))).abs();
            let d1 = (cursor.0 - (left + i16::from(o.rope_anchors[1].0))).abs();
            u8::from(d1 <= d0)
        };
        let taken = if o.definition == Def::Pulley {
            o.linked_objects[0].is_some()
        } else {
            self.rope_on(i, slot)
        };
        (!taken).then_some((i, slot))
    }

    pub(super) fn rope_on(&self, part: usize, slot: u8) -> bool {
        self.objects
            .iter()
            .flat_map(|o| o.ropes.iter().flatten())
            .any(|r| {
                r.endpoints
                    .iter()
                    .zip(r.attachment_slots)
                    .any(|(e, s)| *e == Some(part) && s == slot)
            })
    }

    /// A belt or rope end under the cursor: a free anchor a part from the
    /// bin could start on (the tests of connect_held_belt_to_target and
    /// select_rope_endpoint_under_cursor, applied to each anchor), or one in
    /// use that can be taken off again. A port shortcut, not in the original.
    pub fn attach_point(&self, cursor: (i16, i16)) -> Option<AttachPoint> {
        if !self.connection_hints || self.held.is_some() || self.resizing.is_some() {
            return None;
        }
        let mut best: Option<(i16, AttachPoint)> = None;
        for part in 0..self.objects.len() {
            for point in self.attach_points_on(part) {
                let distance = (point.at.0 - cursor.0)
                    .abs()
                    .max((point.at.1 - cursor.1).abs());
                if distance <= ATTACH_REACH && best.as_ref().is_none_or(|(d, _)| distance < *d) {
                    best = Some((distance, point));
                }
            }
        }
        best.map(|(_, point)| point)
    }

    /// Visible usable anchors on a part, including a rope's actual knot.
    pub fn attach_points_on(&self, part: usize) -> Vec<AttachPoint> {
        let o = &self.objects[part];
        if !self.connection_hints
            || o.list == ObjectList::Inventory
            || o.render_flags.contains(crate::machine::RenderFlag::Hidden)
            || o.width == 0
        {
            return Vec::new();
        }
        let belts = self.bin_count(Def::Belt) > 0;
        let ropes = self.bin_count(Def::Rope) > 0;
        let mut candidates = Vec::new();
        if o.render_flags.contains(RenderFlag::BeltAttachable) {
            // A belt anchor is the corner of its wheel; aim at the middle.
            let half = o.belt_size / 2;
            let (wx, wy) = o.belt_origin();
            let centre = (
                (wx - i32::from(o.x)) as i16 + half,
                (wy - i32::from(o.y)) as i16 + half,
            );
            if !o.render_flags.contains(RenderFlag::BeltConnected) {
                if belts {
                    candidates.push((Attach::Belt, centre, None));
                }
            } else if let Some(belt) = self.belt_on(part) {
                candidates.push((Attach::Belt, centre, Some(belt)));
            }
        }
        // Ropes neither start nor come off at a pulley.
        if o.render_flags.contains(RenderFlag::RopeAttachable) && o.definition != Def::Pulley {
            let slots: &[u8] = if !o.render_flags.contains(RenderFlag::TwoRopeAnchors) {
                &[0]
            } else {
                &[0, 1]
            };
            for &slot in slots {
                // Where the knot is drawn (rope_point), relative to the part.
                let (px, py) = crate::machine::rope_point(o, slot as usize);
                let anchor = ((px - i32::from(o.x)) as i16, (py - i32::from(o.y)) as i16);
                match self.rope_at(part, slot) {
                    None if ropes => candidates.push((Attach::Rope(slot), anchor, None)),
                    None => {}
                    Some(rope) => candidates.push((Attach::Rope(slot), anchor, Some(rope))),
                }
            }
        }
        candidates
            .into_iter()
            .filter_map(|(kind, anchor, connection)| {
                // Original puzzle connections remain locked; free anchors on
                // original puzzle parts can still accept new connections.
                connection
                    .is_none_or(|existing| self.unlocked(existing))
                    .then_some(AttachPoint {
                        part,
                        kind,
                        connection,
                        at: (o.x - VIEWPORT.0 + anchor.0, o.y - VIEWPORT.1 + anchor.1),
                    })
            })
            .collect()
    }

    /// The placed belt with an end on `part`.
    pub(super) fn belt_on(&self, part: usize) -> Option<usize> {
        (0..self.objects.len()).find(|&b| {
            self.objects[b].list != ObjectList::Inventory
                && self.objects[b]
                    .belt
                    .is_some_and(|ends| ends.contains(&Some(part)))
        })
    }

    /// The placed rope with an end on `part`'s anchor `slot`.
    pub(super) fn rope_at(&self, part: usize, slot: u8) -> Option<usize> {
        (0..self.objects.len()).find(|&r| {
            self.objects[r].definition == Def::Rope
                && self.objects[r].list != ObjectList::Inventory
                && self.objects[r].ropes[0].as_ref().is_some_and(|c| {
                    c.endpoints
                        .iter()
                        .zip(c.attachment_slots)
                        .any(|(e, s)| *e == Some(part) && s == slot)
                })
        })
    }

    /// A rope's hops from its first end over any pulleys to its far end,
    /// as connect_rope built them.
    pub(crate) fn rope_route(&self, rope: usize) -> Vec<(usize, u8)> {
        let Some(connection) = self.objects[rope].ropes[0].clone() else {
            return Vec::new();
        };
        let [Some(a), Some(b)] = connection.endpoints else {
            return Vec::new();
        };
        let mut route = vec![(a, connection.attachment_slots[0])];
        let mut current = a;
        let mut slot = connection.attachment_slots[0] as usize;
        for _ in 0..self.objects.len() {
            let o = &self.objects[current];
            let next = if o.definition == Def::Pulley && route.len() > 1 {
                o.linked_objects[0]
            } else {
                o.linked_objects[slot]
            };
            let Some(next) = next else { break };
            if next == b {
                break;
            }
            route.push((next, 0));
            current = next;
            slot = 0;
        }
        route.push((b, connection.attachment_slots[1]));
        route
    }

    /// Act on an attach point: take a belt or rope from the bin with its
    /// first end there, or take that end off a connected one and hold it
    /// with its other end (and any pulleys) still in place.
    pub fn attach(&mut self, point: AttachPoint) -> bool {
        if self.held.is_some() || point.connection.is_some_and(|index| !self.unlocked(index)) {
            return false;
        }
        let (index, carry) = match point.connection {
            None => {
                let definition = match point.kind {
                    Attach::Belt => Def::Belt,
                    Attach::Rope(_) => Def::Rope,
                };
                if !self.take_definition(definition) {
                    return false;
                }
                let carry = match point.kind {
                    Attach::Belt => Carry::Belt {
                        first: Some(point.part),
                    },
                    Attach::Rope(slot) => Carry::Rope {
                        route: vec![(point.part, slot)],
                    },
                };
                (self.held.as_ref().unwrap().index, carry)
            }
            Some(belt) if point.kind == Attach::Belt => {
                let Some(ends) = self.objects[belt].belt else {
                    return false;
                };
                let other = ends.into_iter().flatten().find(|&e| e != point.part);
                for end in ends.into_iter().flatten() {
                    self.objects[end]
                        .render_flags
                        .remove(crate::machine::RenderFlag::BeltConnected);
                }
                self.stow(belt);
                (belt, Carry::Belt { first: other })
            }
            Some(rope) => {
                let mut route = self.rope_route(rope);
                // Keep the far end first, then the pulleys back towards us.
                if route.first().is_some_and(|&(p, _)| p == point.part) {
                    route.reverse();
                }
                route.pop();
                self.unwire_rope(rope);
                self.stow(rope);
                (rope, Carry::Rope { route })
            }
        };
        self.held = Some(Held { index, carry });
        self.hovered = None;
        true
    }

    /// Remove belts and ropes attached to a part; a pulley is spliced out of
    /// its rope instead.
    pub(super) fn detach(&mut self, index: usize) {
        if self.objects[index].flags & PropertyFlag::ELECTRICAL_MASK != 0 {
            self.unplug(index);
        }
        let belts: Vec<usize> = (0..self.objects.len())
            .filter(|&b| {
                self.objects[b]
                    .belt
                    .is_some_and(|ends| ends.contains(&Some(index)))
            })
            .collect();
        for b in belts {
            if let Some(ends) = self.objects[b].belt {
                for end in ends.into_iter().flatten() {
                    self.objects[end]
                        .render_flags
                        .remove(crate::machine::RenderFlag::BeltConnected);
                }
            }
            self.stow(b);
        }
        if self.objects[index].definition == Def::Pulley {
            self.splice_pulley(index);
            return;
        }
        let ropes: Vec<usize> = (0..self.objects.len())
            .filter(|&r| {
                self.objects[r].definition == Def::Rope
                    && self.objects[r].ropes[0]
                        .as_ref()
                        .is_some_and(|c| c.endpoints.contains(&Some(index)))
            })
            .collect();
        for r in ropes {
            self.unwire_rope(r);
            self.stow(r);
        }
    }

    pub(super) fn unwire_rope(&mut self, rope: usize) {
        let Some(connection) = self.objects[rope].ropes[0].clone() else {
            return;
        };
        let [Some(a), _] = connection.endpoints else {
            return;
        };
        let mut slot = connection.attachment_slots[0] as usize;
        let mut current = a;
        let mut previous = None;
        for _ in 0..self.objects.len() {
            let next = if self.objects[current].definition == Def::Pulley && previous.is_some() {
                let next = self.objects[current].linked_objects[0];
                self.objects[current].linked_objects[..4].fill(None);
                self.objects[current].pulley_connection = None;
                next
            } else {
                let next = self.objects[current].linked_objects[slot];
                self.objects[current].linked_objects[slot] = None;
                self.objects[current].linked_objects[slot + 2] = None;
                next
            };
            let Some(next) = next else { break };
            if Some(next) == connection.endpoints[1] {
                let end_slot = connection.attachment_slots[1] as usize;
                self.objects[next].linked_objects[end_slot] = None;
                self.objects[next].linked_objects[end_slot + 2] = None;
                break;
            }
            previous = Some(current);
            current = next;
            slot = 0;
        }
    }

    /// connect_held_belt_to_target, 1dff:1e94.
    pub(super) fn connect_belt(&mut self, belt: usize, first: Option<usize>, cursor: (i16, i16)) {
        let Some(target) = self.valid_belt_target(cursor, first) else {
            return;
        };
        let Some(a) = first else {
            self.held = Some(Held {
                index: belt,
                carry: Carry::Belt {
                    first: Some(target),
                },
            });
            return;
        };
        self.objects[a]
            .render_flags
            .insert(RenderFlag::BeltConnected);
        self.objects[target]
            .render_flags
            .insert(RenderFlag::BeltConnected);
        let o = &mut self.objects[belt];
        o.belt = Some([Some(a), Some(target)]);
        self.place(belt);
        self.held = None;
    }

    /// connect_held_rope_to_target, 1dff:1f90: first end, then any free
    /// pulleys, then the far end completes the rope.
    pub(super) fn connect_rope(
        &mut self,
        rope: usize,
        mut route: Vec<(usize, u8)>,
        cursor: (i16, i16),
    ) {
        let Some((target, slot)) = self.valid_rope_target(cursor, &route) else {
            return;
        };
        let is_pulley = self.objects[target].definition == Def::Pulley;
        route.push((target, slot));
        if is_pulley {
            self.held = Some(Held {
                index: rope,
                carry: Carry::Rope { route },
            });
            return;
        }
        if route.len() < 2 {
            self.held = Some(Held {
                index: rope,
                carry: Carry::Rope { route },
            });
            return;
        }
        self.wire_rope_route(rope, &route);
        self.held = None;
    }

    pub(crate) fn wire_rope_route(&mut self, rope: usize, route: &[(usize, u8)]) {
        // Link each hop: an end's slot points to its neighbour; a pulley's
        // slot 0 points toward the far end and slot 1 back toward the first.
        for (n, &(part, part_slot)) in route.iter().enumerate() {
            let prev = n.checked_sub(1).map(|p| route[p].0);
            let next = route.get(n + 1).map(|&(p, _)| p);
            let o = &mut self.objects[part];
            if o.definition == Def::Pulley && n > 0 && n + 1 < route.len() {
                o.linked_objects[0] = next;
                o.linked_objects[1] = prev;
                o.linked_objects[2] = next;
                o.linked_objects[3] = prev;
                o.pulley_connection = Some(rope);
            } else {
                let neighbour = if n == 0 { next } else { prev };
                o.linked_objects[part_slot as usize] = neighbour;
                o.linked_objects[part_slot as usize + 2] = neighbour;
            }
        }
        let (a, sa) = route[0];
        let (b, sb) = *route.last().unwrap();
        self.objects[rope].ropes[0] = Some(RopeConnection {
            endpoints: [Some(a), Some(b)],
            attachment_slots: [sa, sb],
            rest_length: 0.0,
            slack_count: 0,
        });
        crate::machine::refresh_pulleys(&mut self.objects, None);
        self.place(rope);
    }
}
