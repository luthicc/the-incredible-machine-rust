use super::*;

impl Editor {
    /// The carried part's position for a cursor, as
    /// place_held_object_at_cursor (1dff:21ec) computes it.
    pub(super) fn carried_position(
        &self,
        index: usize,
        cursor: (i16, i16),
        grab: (i16, i16),
    ) -> (i16, i16) {
        let o = &self.objects[index];
        let (w, h) = (o.width as i16, o.height as i16);
        let (vx, vy) = VIEWPORT;
        if o.flags & PropertyFlag::FreePlacement.mask() == 0 {
            let mut x = ((cursor.0 - grab.0) & !0xf) + vx;
            if x + w <= vx {
                x += 0x10;
            }
            let mut y = ((cursor.1 - grab.1) & !0xf) + vy;
            if y + h <= vy {
                y += 0x10;
            }
            (x, y)
        } else {
            let mut x = cursor.0 - grab.0 + vx;
            if x + w <= vx + 0xc {
                x = vx - w + 0xc;
            }
            x = x.min(vx + 0x235);
            let mut y = cursor.1 - grab.1 + vy;
            if y + h <= vy + 0xc {
                y = vy - h + 0xc;
            }
            y = y.min(vy + 0x165);
            (x, y)
        }
    }

    pub(super) fn move_held_part(&mut self, index: usize, x: i16, y: i16) {
        move_to(&mut self.objects[index], x, y);
        crate::machine::refresh_pulleys(&mut self.objects, None);
    }

    pub(super) fn drag_constraint_fits(&self, index: usize, constraint: &DragConstraint) -> bool {
        match *constraint {
            DragConstraint::Rope { owner, slot } => self.objects[owner].ropes[slot]
                .as_ref()
                .is_some_and(|rope| rope_length(&self.objects, rope).is_some()),
            DragConstraint::Belt {
                owner,
                other,
                max_distance,
            } => {
                self.objects[owner]
                    .belt
                    .is_some_and(|ends| ends.contains(&Some(index)) && ends.contains(&Some(other)))
                    && belt_distance(&self.objects[index], &self.objects[other])
                        <= max_distance + 0.5
            }
        }
    }

    pub(super) fn capture_drag_constraints(&mut self, index: usize) {
        self.drag_constraints.clear();
        for (owner, object) in self.objects.iter().enumerate() {
            for (slot, rope) in object.ropes.iter().enumerate() {
                if let Some(rope) = rope {
                    if rope.endpoints.contains(&Some(index)) {
                        if rope_length(&self.objects, rope).is_some() {
                            self.drag_constraints
                                .push(DragConstraint::Rope { owner, slot });
                        }
                    }
                }
            }
            if let Some([Some(a), Some(b)]) = object.belt {
                if a == index || b == index {
                    self.drag_constraints.push(DragConstraint::Belt {
                        owner,
                        other: if a == index { b } else { a },
                        max_distance: belt_distance(&self.objects[a], &self.objects[b]),
                    });
                }
            }
        }
    }

    /// detect_object_collision against the placed parts.
    pub fn collides(&self, index: usize) -> bool {
        let mut probe = self.objects.clone();
        probe[index].list = ObjectList::Moving;
        crate::contact::detect_object_collision(&probe, index)
    }

    /// Whether the carried part can be dropped without overlapping another part.
    pub fn can_place(&self) -> bool {
        match &self.held {
            Some(Held {
                index,
                carry: Carry::Part { .. },
            }) => !self.collides(*index),
            _ => false,
        }
    }

    /// Draw a valid carried part as it would look after dropping, including
    /// plugs in nearby sockets. Preview links never enter the saved machine.
    pub fn placement_preview(&self) -> Vec<Object> {
        let Some(Held {
            index,
            carry: Carry::Part { .. },
        }) = &self.held
        else {
            let mut objects = self.objects.clone();
            if !self.plug_preview {
                return objects;
            }
            // Puzzle files can place a plug-bearing part beside an outlet
            // without storing an attachment. The native editor shows the
            // nearby plug before the first drag; derive that appearance for
            // this render clone without changing the saved machine links.
            preview_nearby_plugs(&mut objects, None);
            return objects;
        };
        let carried_outlet_plugs = self
            .plug_preview
            .then_some(self.held_outlet_plugs)
            .flatten()
            .filter(|(outlet, _)| outlet == index);
        let mut objects = self.objects.clone();
        if let Some((outlet, plugs)) = carried_outlet_plugs {
            let mask = outlet_socket_mask(objects[outlet].definition);
            objects[outlet].state = (objects[outlet].state & !mask) | plugs;
            refresh_outline(&mut objects[outlet]);
        }
        if !self.plug_preview {
            return objects;
        }
        let mut preview = self.clone();
        preview.place(*index);
        if let Some((outlet, plugs)) = carried_outlet_plugs {
            let mask = outlet_socket_mask(preview.objects[outlet].definition);
            preview.objects[outlet].state = (preview.objects[outlet].state & !mask) | plugs;
            refresh_outline(&mut preview.objects[outlet]);
            preview_nearby_plugs(&mut preview.objects, Some(outlet));
        }
        preview.objects
    }

    /// move_object_to_active_list (find at 1dff:21ec when the drop is clear).
    pub(crate) fn place(&mut self, index: usize) {
        self.drag_constraints.clear();
        let object = &mut self.objects[index];
        object.update_flags &= !UpdateFlag::InInventory.mask();
        if object.update_flags & UpdateFlag::FixedDefinition.mask() == 0 {
            object.update_flags |= UpdateFlag::ActiveMoving.mask();
            object.list = ObjectList::Moving;
        } else {
            object.update_flags |= UpdateFlag::ActiveFixed.mask();
            object.list = ObjectList::Fixed;
        }
        let (x, y) = (object.x, object.y);
        move_to(object, x, y);
        refresh_outline(object);
        // place_held_object_at_cursor (1dff:21ec): a part with a plug looks
        // for a socket; an outlet that moved loses whatever was plugged in.
        if self.objects[index].flags & PropertyFlag::ElectricalPlug.mask() != 0 {
            self.plug_in(index);
        } else if self.objects[index].flags & PropertyFlag::ElectricalOutlet.mask() != 0 {
            self.unplug(index);
            let nearby_devices: Vec<_> =
                crate::spatial::nearby_parts(&self.objects, index, (-8, 8), (-8, 8))
                    .into_iter()
                    .map(|contact| contact.index)
                    .filter(|&device| {
                        self.objects[device].flags & PropertyFlag::ElectricalPlug.mask() != 0
                    })
                    .collect();
            for device in nearby_devices {
                self.plug_in(device);
            }
        }
    }

    /// A left click in the playfield (screen coordinates).
    pub fn click(&mut self, cursor: (i16, i16)) {
        if self.resizing.take().is_some() {
            return;
        }
        match self.held.clone() {
            Some(Held {
                index,
                carry: Carry::Part { .. },
            }) => {
                if !self.collides(index) {
                    self.break_overstretched_connections(index);
                    self.place(index);
                    self.held = None;
                    self.held_outlet_plugs = None;
                }
            }
            Some(Held {
                index,
                carry: Carry::Belt { first },
            }) => self.connect_belt(index, first, cursor),
            Some(Held {
                index,
                carry: Carry::Rope { route },
            }) => self.connect_rope(index, route, cursor),
            None => {
                let handle = self.handle_under(cursor);
                // An attach point beats picking the part up, not its handles.
                if matches!(handle, None | Some((_, Some(Handle::Body) | None))) {
                    if let Some(point) = self.attach_point(cursor) {
                        self.attach(point);
                        return;
                    }
                }
                let Some((part, handle)) = handle else { return };
                self.hovered = Some(part);
                match handle {
                    Some(Handle::Body) => self.pick_up(part, cursor),
                    Some(Handle::Remove) => self.return_to_bin(part),
                    Some(Handle::FlipHorizontal) => self.flip(part, 1),
                    Some(Handle::FlipVertical) => self.flip(part, 2),
                    Some(handle) => {
                        self.resizing = Some(Resize {
                            index: part,
                            handle,
                        })
                    }
                    None => {}
                }
            }
        }
    }

    /// Handle 7: carry a placed part, keeping the cursor's offset in it.
    pub(super) fn pick_up(&mut self, index: usize, cursor: (i16, i16)) {
        if matches!(self.objects[index].definition, Def::Belt | Def::Rope) {
            return;
        }
        // 1000:5704 preserves attachments for the body/carry modes.
        // Returning a part to the bin detaches; picking it up only moves it.
        self.held_outlet_plugs =
            (self.objects[index].flags & PropertyFlag::ElectricalOutlet.mask() != 0).then_some((
                index,
                self.objects[index].state & outlet_socket_mask(self.objects[index].definition),
            ));
        if self.objects[index].flags & PropertyFlag::ELECTRICAL_MASK != 0 {
            self.unplug(index);
        }
        self.capture_drag_constraints(index);
        let o = &self.objects[index];
        let grab = (cursor.0 - (o.x - VIEWPORT.0), cursor.1 - (o.y - VIEWPORT.1));
        self.objects[index].list = ObjectList::Inventory;
        self.hovered = None;
        self.held = Some(Held {
            index,
            carry: Carry::Part { grab },
        });
    }

    pub(super) fn stow(&mut self, index: usize) {
        let o = &mut self.objects[index];
        o.list = ObjectList::Inventory;
        o.update_flags =
            (o.update_flags & !UpdateFlag::ACTIVE_LIST_MASK) | UpdateFlag::InInventory.mask();
        o.belt = o.belt.map(|_| [None, None]);
        if let Some(rope) = o.ropes[0].as_mut() {
            if o.definition == Def::Rope {
                rope.endpoints = [None, None];
                rope.attachment_slots = [0, 0];
                rope.rest_length = 0.0;
            }
        }
        if self.held.as_ref().is_some_and(|h| h.index == index) {
            self.held = None;
        }
        if self.hovered == Some(index) {
            self.hovered = None;
        }
    }

    pub(super) fn splice_pulley(&mut self, pulley: usize) {
        let [Some(next), Some(prev), ..] = self.objects[pulley].linked_objects else {
            return;
        };
        for (part, from, to) in [(next, pulley, prev), (prev, pulley, next)] {
            for slot in 0..4 {
                if self.objects[part].linked_objects[slot] == Some(from) {
                    self.objects[part].linked_objects[slot] = Some(to);
                }
            }
        }
        self.objects[pulley].linked_objects[..4].fill(None);
        self.objects[pulley].pulley_connection = None;
    }

    /// Right click or Escape while carrying: the part goes back to the bin.
    pub fn cancel(&mut self) {
        self.resizing = None;
        self.drag_constraints.clear();
        self.held_outlet_plugs = None;
        if let Some(held) = self.held.take() {
            if matches!(held.carry, Carry::Part { .. }) {
                self.detach(held.index);
            }
            self.stow(held.index);
        }
    }
}
