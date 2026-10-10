use super::*;

impl Editor {
    /// Follow the cursor (screen coordinates) with the carried part, or
    /// track which placed part is highlighted.
    pub fn hover(&mut self, cursor: (i16, i16)) {
        if let Some(resize) = self.resizing {
            self.drag_resize(resize, cursor);
            return;
        }
        if self.held.is_none() {
            self.track_hover(cursor);
            return;
        }
        self.hovered = None;
        let Some(Held {
            index,
            carry: Carry::Part { grab },
        }) = self.held.clone()
        else {
            return;
        };
        let (x, y) = self.carried_position(index, cursor, grab);
        self.move_held_part(index, x, y);
    }

    /// Top-most placed part under the cursor (find_object_at_cursor).
    /// Locked parts can accept connections even though they cannot be moved.
    pub fn part_at(&self, cursor: (i16, i16)) -> Option<usize> {
        let (x, y) = screen_to_world(cursor);
        (0..self.objects.len()).rev().find(|&i| {
            let o = &self.objects[i];
            o.list != ObjectList::Inventory
                && !o.render_flags.contains(crate::machine::RenderFlag::Hidden)
                && o.width > 0
                && o.x < x
                && x < o.x + o.width as i16
                && o.y < y
                && y < o.y + o.height as i16
        })
    }

    /// find_object_at_cursor(selected_object) (1000:4500), made forgiving:
    /// the highlighted part keeps its selection while the cursor is near its
    /// body or handles, or is heading for a handle through the safe
    /// triangle. Otherwise the unlocked part under the cursor is selected.
    pub fn select_at(&self, cursor: (i16, i16)) -> Option<usize> {
        if let Some(i) = self.hovered.filter(|&i| self.highlightable(i)) {
            if self.near_handles(i, cursor) || self.in_safe_triangle(i, cursor) {
                return Some(i);
            }
        }
        self.part_at(cursor).filter(|&i| self.unlocked(i))
    }

    pub(super) fn highlightable(&self, index: usize) -> bool {
        let o = &self.objects[index];
        o.list != ObjectList::Inventory
            && !o.render_flags.contains(crate::machine::RenderFlag::Hidden)
            && self.unlocked(index)
    }

    /// The body widened by hit_test_object_at_cursor's 0xb, plus every
    /// handle widened by HANDLE_OVERSHOOT so a fast move past one is kept.
    pub(super) fn near_handles(&self, index: usize, cursor: (i16, i16)) -> bool {
        let o = &self.objects[index];
        let (x, y) = (o.x - VIEWPORT.0, o.y - VIEWPORT.1);
        let body = (
            x - 0xb,
            y - 0xb,
            x + o.width as i16 + 0xb,
            y + o.height as i16 + 0xb,
        );
        contains(body, cursor)
            || self
                .handle_icons(index)
                .iter()
                .any(|icon| contains(grow(icon.hit(), HANDLE_OVERSHOOT), cursor))
    }

    /// Safe triangle: from where the cursor last was near the part, the
    /// hull towards each handle keeps the selection for a short while, so a
    /// diagonal path to a handle may cross the gap or another part.
    pub(super) fn in_safe_triangle(&self, index: usize, cursor: (i16, i16)) -> bool {
        let Some(anchor) = self.hover_anchor else {
            return false;
        };
        if self.hover_grace == 0 {
            return false;
        }
        self.handle_icons(index)
            .iter()
            .any(|icon| in_hull(anchor, grow(icon.hit(), HANDLE_OVERSHOOT), cursor))
    }

    /// Track the highlighted part for a cursor move without a carried part.
    pub(super) fn track_hover(&mut self, cursor: (i16, i16)) {
        let previous = self.hovered;
        let near = previous.is_some_and(|i| self.highlightable(i) && self.near_handles(i, cursor));
        self.hovered = self.select_at(cursor);
        if self.hovered != previous || near {
            self.hover_grace = SAFE_TRIANGLE_FRAMES;
            self.hover_anchor = self.hovered.map(|_| cursor);
        } else if self.hovered.is_some() && self.last_cursor == Some(cursor) {
            // Resting inside the triangle lets the selection lapse.
            self.hover_grace = self.hover_grace.saturating_sub(1);
        }
        self.last_cursor = Some(cursor);
    }

    /// The highlighted part and the handle under the cursor, if any.
    pub fn handle_under(&self, cursor: (i16, i16)) -> Option<(usize, Option<Handle>)> {
        let index = self.select_at(cursor)?;
        Some((index, self.handle_at(index, cursor)))
    }

    pub(super) fn unlocked(&self, index: usize) -> bool {
        self.objects[index].update_flags & UpdateFlag::Locked.mask() == 0
    }

    /// The handle icons draw_hover_highlight (24de:1429) draws for a part,
    /// at the positions it draws them. Hit-testing uses the same icons.
    pub fn handle_icons(&self, index: usize) -> Vec<HandleIcon> {
        let o = &self.objects[index];
        let (w, h) = (o.width as i16, o.height as i16);
        let x0 = (o.x - VIEWPORT.0).max(8);
        let y0 = (o.y - VIEWPORT.1).max(8);
        let left = x0 - 0xc;
        let mid_x = left + (w >> 1) + 6;
        let right = left + w + 0xc;
        let top = y0 - 0xb;
        let mid_y = top + (h >> 1) + 6;
        let bottom = top + h + 0xc;
        let flags = self.connection_flags(index);
        let icon = |handle, frame, at| HandleIcon { handle, frame, at };
        let mut icons = vec![icon(Handle::Remove, 27, (left, top))];
        if flags & 1 != 0 {
            icons.push(icon(Handle::ResizeLeft, 28, (left, mid_y)));
            icons.push(icon(Handle::ResizeRight, 28, (right, mid_y)));
        }
        if flags & 2 != 0 {
            icons.push(icon(Handle::ResizeTop, 29, (mid_x, top)));
            icons.push(icon(Handle::ResizeBottom, 29, (mid_x, bottom)));
        }
        if flags & 4 != 0 {
            icons.push(icon(Handle::FlipHorizontal, 30, (left, bottom)));
        }
        if flags & 8 != 0 {
            icons.push(icon(Handle::FlipVertical, 31, (right, bottom)));
        }
        icons
    }

    /// compute_object_hover_region (1000:4830) for a placed, unlocked part,
    /// tested against the drawn icons: an icon's painted pixels beat the
    /// body, and its padding only counts outside the body.
    pub fn handle_at(&self, index: usize, cursor: (i16, i16)) -> Option<Handle> {
        let o = &self.objects[index];
        let (x, y) = (o.x - VIEWPORT.0, o.y - VIEWPORT.1);
        let icons = self.handle_icons(index);
        if let Some(icon) = icons.iter().find(|icon| contains(icon.opaque(), cursor)) {
            return Some(icon.handle);
        }
        if contains((x, y, x + o.width as i16, y + o.height as i16), cursor) {
            return Some(Handle::Body);
        }
        icons
            .iter()
            .find(|icon| contains(icon.hit(), cursor))
            .map(|icon| icon.handle)
    }
}
