use super::*;
use bevy::input::touch::Touches;
use bevy::window::CursorMoved;
use std::collections::VecDeque;
use tim_bevy::viewport::Zoom;

/// A finger that moves further than this (logical pixels) is a drag, not a tap.
const DRAG_DISTANCE: f32 = 8.0;

/// Touch drives the game as a mouse. One finger moves the pointer and taps
/// click; a carried part follows the finger and drops where it lifts. Two
/// fingers pinch-zoom and pan the playfield.
#[derive(Resource, Default)]
pub(super) struct TouchPointer {
    /// Last touch position in logical window pixels, until the mouse is used.
    pub position: Option<Vec2>,
    /// A single finger is down (the loupe follows it).
    pub down: bool,
    primary: Option<Finger>,
    /// Set once a second finger lands; touch stays a gesture until every
    /// finger lifts.
    pub pinching: bool,
    /// Finger distance and midpoint from the previous pinch frame.
    pinch: Option<(f32, Vec2)>,
    /// Button changes, one per frame so the game sees each of them.
    queue: VecDeque<Step>,
    last_touch: f32,
}

struct Finger {
    id: u64,
    start: Vec2,
    last: Vec2,
    kind: Gesture,
    moved: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Gesture {
    /// Holds the left button, like a mouse press.
    Press,
    /// Landed while something was carried: it only moves it, and lifting
    /// clicks to drop.
    Carry,
    /// Landed on empty playfield: dragging pans the zoomed view, and a tap
    /// still clicks.
    Pan,
}

enum Step {
    Press(MouseButton),
    Release(MouseButton),
    /// Put a belt or rope down if the click before this did not use it.
    DropUnusedTool(Option<tim_bevy::editor::Carry>),
}

impl TouchPointer {
    /// The pointer position from whichever device was used last.
    pub fn pointer(&self, window: &Window) -> Option<Vec2> {
        self.position.or_else(|| window.cursor_position())
    }
    /// Touch was used last, so the game draws no mouse cursor.
    pub fn active(&self) -> bool {
        self.position.is_some()
    }
}

/// The playfield zoom.
#[derive(Resource)]
pub(super) struct ZoomState(pub Zoom);

impl Default for ZoomState {
    fn default() -> Self {
        let (l, t, r, b) = game_screen::PLAYFIELD;
        Self(Zoom::new((l as f32, t as f32, r as f32, b as f32)))
    }
}

/// The playfield can zoom while a puzzle is being edited with no menu open.
pub(super) fn zoom_allowed(game: &Game) -> bool {
    let front = &game.front;
    game.mode == Mode::Puzzle
        && front.session.phase == Phase::Editing
        && !front.qol_open
        && !front.solutions.open
}

fn held_carry(game: &Game) -> Option<tim_bevy::editor::Carry> {
    game.front
        .session
        .editor
        .held
        .as_ref()
        .map(|h| h.carry.clone())
}

fn holding_tool(game: &Game) -> bool {
    matches!(
        held_carry(game),
        Some(tim_bevy::editor::Carry::Belt { .. } | tim_bevy::editor::Carry::Rope { .. })
    )
}

/// The finger is on empty playfield, where nothing would react to a press.
fn on_background(window: &Window, game: &Game, zoom: &ZoomState, point: Vec2) -> bool {
    let Some((x, y)) = window_to_source(window, game, zoom, point) else {
        return false;
    };
    let cursor = (x as i16, y as i16);
    zoom_allowed(game)
        && game_screen::inside(game_screen::PLAYFIELD, cursor)
        && game_screen::hover(&game.front.session, cursor) == game_screen::Hover::Nothing
}

fn carrying(game: &Game) -> bool {
    let editor = &game.front.session.editor;
    game.mode == Mode::Puzzle
        && game.front.session.phase == Phase::Editing
        && (editor.held.is_some() || editor.resizing.is_some())
}

/// Unzoomed source pixels under a logical window point.
fn unzoomed(window: &Window, game: &Game, point: Vec2) -> (f32, f32) {
    let base = base_viewport(window, game);
    (
        (point.x - base.origin.0) / base.scale,
        (point.y - base.origin.1) / base.scale_y,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn emulate_mouse(
    touches: Res<Touches>,
    time: Res<Time>,
    windows: Query<&Window>,
    game: Res<Game>,
    mut moved: MessageReader<CursorMoved>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut touch: ResMut<TouchPointer>,
    mut zoom: ResMut<ZoomState>,
) {
    let now = time.elapsed_secs();
    let active: Vec<_> = touches.iter().collect();
    let ended: Vec<_> = touches
        .iter_just_released()
        .chain(touches.iter_just_canceled())
        .collect();
    if !active.is_empty() || !ended.is_empty() {
        touch.last_touch = now;
    }
    // Browsers may follow a touch with emulated pointer moves; only a later
    // mouse movement switches back to the mouse.
    if moved.read().count() > 0 && now - touch.last_touch > 0.5 {
        touch.position = None;
    }
    if touch.pinching {
        if let ([a, b, ..], Ok(window)) = (active.as_slice(), windows.single()) {
            let distance = a.position().distance(b.position());
            let middle = a.position().midpoint(b.position());
            if let Some((last_distance, last_middle)) = touch.pinch.filter(|p| p.0 > 0.0)
                && zoom_allowed(&game)
            {
                let point = unzoomed(window, &game, middle);
                let factor = zoom.0.factor * distance / last_distance;
                zoom.0.zoom_at(point, factor);
                let last = unzoomed(window, &game, last_middle);
                zoom.0.pan((point.0 - last.0, point.1 - last.1));
            }
            touch.pinch = Some((distance, middle));
        } else {
            touch.pinch = None;
        }
        if active.is_empty() {
            touch.pinching = false;
        }
    } else if active.len() >= 2 {
        // A second finger turns the touch into a gesture.
        touch.pinching = true;
        touch.pinch = None;
        if let Some(finger) = touch.primary.take()
            && finger.kind == Gesture::Press
        {
            touch.queue.push_back(Step::Release(MouseButton::Left));
        }
    } else {
        let window = windows.single().ok();
        for t in touches.iter_just_pressed() {
            if touch.primary.is_none() {
                let kind = if carrying(&game) {
                    Gesture::Carry
                } else if window.is_some_and(|w| on_background(w, &game, &zoom, t.position())) {
                    Gesture::Pan
                } else {
                    Gesture::Press
                };
                touch.primary = Some(Finger {
                    id: t.id(),
                    start: t.position(),
                    last: t.position(),
                    kind,
                    moved: false,
                });
                touch.position = Some(t.position());
                if kind == Gesture::Press {
                    touch.queue.push_back(Step::Press(MouseButton::Left));
                }
            }
        }
        if let Some(position) = touch
            .primary
            .as_ref()
            .and_then(|f| touches.get_pressed(f.id))
            .map(|t| t.position())
            && let Some(mut finger) = touch.primary.take()
        {
            if finger.start.distance(position) > DRAG_DISTANCE {
                finger.moved = true;
            }
            if finger.kind == Gesture::Pan && zoom.0.active() {
                if let Some(window) = window
                    && finger.moved
                {
                    let now = unzoomed(window, &game, position);
                    let last = unzoomed(window, &game, finger.last);
                    zoom.0.pan((now.0 - last.0, now.1 - last.1));
                }
            } else {
                touch.position = Some(position);
            }
            finger.last = position;
            touch.primary = Some(finger);
        }
        for t in ended {
            if touch.primary.as_ref().is_some_and(|f| f.id == t.id()) {
                let finger = touch.primary.take().unwrap();
                let left = MouseButton::Left;
                let click = [Step::Press(left), Step::Release(left)];
                match finger.kind {
                    Gesture::Pan if finger.moved => continue,
                    Gesture::Pan => touch.queue.extend(click),
                    Gesture::Carry => {
                        touch.queue.extend(click);
                        // A tap that does not connect puts a belt or rope down.
                        if holding_tool(&game) {
                            touch
                                .queue
                                .push_back(Step::DropUnusedTool(held_carry(&game)));
                        }
                    }
                    Gesture::Press => {
                        touch.queue.push_back(Step::Release(left));
                        // Dragged straight out of the bin or off a handle: drop it.
                        if finger.moved && carrying(&game) {
                            touch.queue.extend(click);
                        }
                    }
                }
                touch.position = Some(t.position());
            }
        }
    }
    // The loupe hides while a drag is moving the zoomed view.
    touch.down = touch
        .primary
        .as_ref()
        .is_some_and(|f| f.kind != Gesture::Pan || !zoom.0.active());
    match touch.queue.pop_front() {
        Some(Step::Press(button)) => mouse.press(button),
        Some(Step::Release(button)) => mouse.release(button),
        Some(Step::DropUnusedTool(before)) => {
            if before.is_some() && held_carry(&game) == before {
                // A right click puts the carried tool down.
                mouse.press(MouseButton::Right);
                touch.queue.push_front(Step::Release(MouseButton::Right));
            }
        }
        None => {}
    }
}

/// Ctrl + wheel (and trackpad pinch, which browsers report the same way)
/// zooms the playfield around the pointer.
pub(super) fn wheel_zoom(
    keys: Res<ButtonInput<KeyCode>>,
    mut wheel: MessageReader<MouseWheel>,
    windows: Query<&Window>,
    game: Res<Game>,
    mut zoom: ResMut<ZoomState>,
) {
    let delta: f32 = wheel
        .read()
        .map(|event| match event.unit {
            MouseScrollUnit::Line => event.y,
            MouseScrollUnit::Pixel => event.y / 40.0,
        })
        .sum();
    if delta == 0.0 || !ctrl_held(&keys) || !zoom_allowed(&game) {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(pointer) = window.cursor_position() else {
        return;
    };
    let point = unzoomed(window, &game, pointer);
    if zoom.0.contains(point) {
        let factor = zoom.0.factor * 1.15f32.powf(delta);
        zoom.0.zoom_at(point, factor);
    }
}

pub(super) fn ctrl_held(keys: &ButtonInput<KeyCode>) -> bool {
    keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight])
}

/// Glide back to the whole playfield when the machine starts or a menu opens.
pub(super) fn ease_zoom(time: Res<Time>, game: Res<Game>, mut zoom: ResMut<ZoomState>) {
    if zoom.0.active() && !zoom_allowed(&game) {
        zoom.0.ease_out((-time.delta_secs() * 6.0).exp());
    }
}

/// Set by the browser page's back button; read once per frame as Escape.
pub(super) static BACK_REQUESTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Press Escape for one frame when the page asks to go back, except where
/// Escape would quit the game.
pub(super) fn page_back(
    game: Res<Game>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut held: Local<bool>,
) {
    if std::mem::take(&mut *held) {
        keys.release(KeyCode::Escape);
    }
    if !BACK_REQUESTED.swap(false, std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    let front = &game.front;
    let backs_out = match game.mode {
        Mode::Puzzle => true,
        Mode::Panel => front.dialog.is_some() || front.qol_open || front.solutions.open,
        Mode::Intro | Mode::Attract => false,
    };
    if backs_out {
        keys.press(KeyCode::Escape);
        *held = true;
    }
}
