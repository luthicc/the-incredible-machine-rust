use super::*;

impl Editor {
    pub(super) fn in_bin(&self, index: usize) -> bool {
        self.objects[index].list == ObjectList::Inventory
            && self.held.as_ref().is_none_or(|h| h.index != index)
    }

    /// Bin parts in native 0800-list order, grouped as draw_parts_list_panel
    /// counts them. The carried part is not counted.
    pub fn bin_groups(&self) -> Vec<BinGroup> {
        let mut order: Vec<usize> = match &self.templates {
            Some(templates) => templates.clone(),
            None => (0..self.objects.len())
                .filter(|&i| self.objects[i].list == ObjectList::Inventory)
                .collect(),
        };
        order.sort_by_key(|&i| bin_order(self.objects[i].definition));
        let mut groups: Vec<BinGroup> = Vec::new();
        for i in order {
            let definition = self.objects[i].definition;
            let available = self.in_bin(i);
            match groups.last_mut() {
                Some(group) if group.definition == definition => {
                    if available {
                        if group.count == 0 {
                            group.first = i;
                        }
                        group.count += 1;
                    }
                }
                _ => groups.push(BinGroup {
                    definition,
                    count: usize::from(available),
                    first: i,
                }),
            }
        }
        groups.retain(|g| g.count > 0);
        groups
    }

    /// Visible groups with their slot rectangles' top y.
    pub fn visible_bin(&self) -> Vec<(i16, BinGroup)> {
        let mut y = 100;
        let mut rows = Vec::new();
        for group in self.bin_groups().into_iter().skip(self.bin_scroll) {
            if y >= 0x135 {
                break;
            }
            rows.push((y, group));
            y += BIN_ROW;
        }
        rows
    }

    pub fn scroll_bin(&mut self, delta: isize) {
        let groups = self.bin_groups().len();
        let max = groups.saturating_sub(5);
        self.bin_scroll = (self.bin_scroll as isize + delta).clamp(0, max as isize) as usize;
    }

    /// pick_up_object_under_cursor (1dff:2e24) on a bin slot.
    pub fn take_from_bin(&mut self, slot: usize) {
        if self.held.is_some() {
            return;
        }
        let Some((_, group)) = self.visible_bin().into_iter().nth(slot) else {
            return;
        };
        self.take_group(&group);
    }

    /// Parts of a definition left in the bin (freeform templates never run out).
    pub fn bin_count(&self, definition: Def) -> usize {
        self.bin_groups()
            .iter()
            .find(|g| g.definition == definition)
            .map_or(0, |g| g.count)
    }

    /// Put another part of `definition` in the hand, if the bin has one.
    pub fn take_definition(&mut self, definition: Def) -> bool {
        if self.held.is_some() {
            return false;
        }
        let Some(group) = self
            .bin_groups()
            .into_iter()
            .find(|g| g.definition == definition)
        else {
            return false;
        };
        self.take_group(&group);
        true
    }

    pub(super) fn take_group(&mut self, group: &BinGroup) {
        self.held_outlet_plugs = None;
        let index = if self.templates.is_some() {
            self.objects.push(self.objects[group.first].clone());
            self.objects.len() - 1
        } else {
            (0..self.objects.len())
                .find(|&i| self.in_bin(i) && self.objects[i].definition == group.definition)
                .unwrap_or(group.first)
        };
        let carry = match self.objects[index].definition {
            Def::Belt => Carry::Belt { first: None },
            Def::Rope => Carry::Rope { route: Vec::new() },
            _ => Carry::Part { grab: (0, 0) },
        };
        let object = &mut self.objects[index];
        if object.width == 0 && !matches!(carry, Carry::Part { .. }) {
            // Belts and ropes have no body; they are drawn as connections.
        }
        refresh_outline(object);
        self.held = Some(Held { index, carry });
    }

    /// Return a placed part (and anything wired to it) to the bin, as
    /// detach_and_destroy_active_object (1dff) unwires it.
    pub fn return_to_bin(&mut self, index: usize) {
        self.detach(index);
        self.stow(index);
    }
}
