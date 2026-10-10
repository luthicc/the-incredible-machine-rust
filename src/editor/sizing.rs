use super::*;

impl Editor {
    /// Definition flip callbacks (definition table +0x2c), mode 1 or 2.
    pub fn flip(&mut self, index: usize, mode: u8) {
        let o = &mut self.objects[index];
        match o.definition {
            // toggle_seesaw_tilt, 272c:41bb.
            Def::Seesaw => {
                o.state = if o.state == 0 { 2 } else { 0 };
                o.saved_state = o.state;
                if let Some(anchors) = crate::outline::seesaw_anchors(o.state) {
                    o.rope_anchors = anchors;
                }
            }
            // toggle_light_state, 272c:2bc5: the pull cord moves with the
            // lamp (set_light_rope_anchor_for_state).
            Def::Light => {
                crate::simulation::toggle_light_state(o);
                o.saved_state = o.state;
            }
            // toggle_curved_pipe_orientation, 272c:37e5.
            Def::CurvedPipe => {
                o.state ^= if mode == 1 { 1 } else { 2 };
                o.saved_state = o.state;
            }
            // toggle_power_distributor_state, 272c:15fc.
            Def::PowerDistributor => {
                o.state = if o.state < 4 { 4 } else { 0 };
                o.saved_state = o.state;
            }
            // flip_eye_hook_direction, 272c:19fa.
            Def::EyeHook => o
                .render_flags
                .toggle(crate::machine::RenderFlag::FlipVertical),
            _ => {
                o.render_flags
                    .toggle(crate::machine::RenderFlag::FlipHorizontal);
                let flipped = o
                    .render_flags
                    .contains(crate::machine::RenderFlag::FlipHorizontal);
                match o.definition {
                    // flip_cat_powered_drive_direction, 272c:2fba.
                    Def::CatPoweredDrive => o.belt_anchor.0 = if flipped { 3 } else { 0x1e },
                    // build_gun_outline, 272c:23b1.
                    Def::Gun => o.rope_anchors[0] = (if flipped { 0x2a } else { 0x12 }, 0x12),
                    // build_monkey_outline, 272c:2cce: pedal belt and rope
                    // hand mirror with the monkey.
                    Def::Monkey => {
                        o.rope_anchors[0] = (if flipped { 0x10 } else { 0x4b }, 0x2d);
                        o.belt_anchor = (if flipped { 0x24 } else { 0x2f }, 0x3c);
                    }
                    _ => {}
                }
            }
        }
        o.refresh_belt_anchor();
        refresh_outline(o);
        crate::machine::refresh_pulleys(&mut self.objects, None);
    }

    /// Drag a resize handle (1dff:2826 and the snap helpers): sizes snap to
    /// 16 pixels within the definition bounds and shrink on collision.
    pub(super) fn drag_resize(&mut self, resize: Resize, cursor: (i16, i16)) {
        let index = resize.index;
        let before = self.objects[index].clone();
        let ((min_w, max_w), (min_h, max_h)) = size_limits(before.definition);
        let (wx, wy) = screen_to_world(cursor);
        let snap = |v: i16| ((v - VIEWPORT.0) & !0xf) + VIEWPORT.0;
        let (mut x, mut y, mut w, mut h) = (
            before.x,
            before.y,
            before.width as i16,
            before.height as i16,
        );
        match resize.handle {
            Handle::ResizeRight => w = (snap(wx) + 0x10 - x).clamp(min_w, max_w),
            Handle::ResizeLeft => {
                let right = x + w;
                w = (right - snap(wx)).clamp(min_w, max_w);
                x = right - w;
            }
            Handle::ResizeBottom => h = (snap(wy) + 0x10 - y).clamp(min_h, max_h),
            Handle::ResizeTop => {
                let bottom = y + h;
                h = (bottom - snap(wy)).clamp(min_h, max_h);
                y = bottom - h;
            }
            _ => return,
        }
        if (x, y, w, h)
            == (
                before.x,
                before.y,
                before.width as i16,
                before.height as i16,
            )
        {
            return;
        }
        loop {
            self.apply_size(index, x, y, w, h);
            if !self.collides(index) {
                break;
            }
            // Native 1dff:2a10/2aea and the opposite-side helpers retry
            // each 16-pixel extent, keeping the opposite edge fixed.
            match resize.handle {
                Handle::ResizeRight if w > min_w => w -= 16,
                Handle::ResizeLeft if w > min_w => {
                    x += 16;
                    w -= 16;
                }
                Handle::ResizeBottom if h > min_h => h -= 16,
                Handle::ResizeTop if h > min_h => {
                    y += 16;
                    h -= 16;
                }
                _ => {
                    self.objects[index] = before;
                    crate::machine::refresh_pulleys(&mut self.objects, None);
                    break;
                }
            }
        }
    }

    /// Definition size callbacks (definition table +0x30).
    pub(super) fn apply_size(&mut self, index: usize, x: i16, y: i16, w: i16, h: i16) {
        let o = &mut self.objects[index];
        move_to(o, x, y);
        o.width = w as u16;
        o.height = h as u16;
        o.alt_size = (w as u16, h as u16);
        match o.definition {
            // setup_incline_from_position, 272c:2789.
            Def::Incline => {
                o.state = (w / 0x10 - 1) as u16;
                o.saved_state = o.state;
            }
            // setup_motor_from_position, 272c:261d; anchors at DS:3330.
            Def::Motor => {
                let step = ((w - 0x20) / 0x10) as usize;
                o.state = (step * 7) as u16;
                o.saved_state = o.state;
                o.belt_anchor.0 = [9, 23, 38, 44, 59][step.min(4)];
            }
            _ => {}
        }
        refresh_outline(o);
        crate::machine::refresh_pulleys(&mut self.objects, None);
    }
}
