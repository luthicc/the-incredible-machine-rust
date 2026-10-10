use bevy::prelude::*;

pub struct BrandingPlugin;

impl Plugin for BrandingPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(Update, set_window_icon);
        #[cfg(target_arch = "wasm32")]
        let _ = app;
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn set_window_icon(
    _main_thread: bevy::ecs::system::NonSendMarker,
    windows: Query<Entity, With<bevy::window::PrimaryWindow>>,
    mut applied: Local<bool>,
) {
    if *applied {
        return;
    }
    let Ok(entity) = windows.single() else { return };
    bevy::winit::WINIT_WINDOWS.with(|windows| {
        let windows = windows.borrow();
        let Some(window) = windows.get_window(entity) else {
            return;
        };
        let image = tim_bevy::branding::icon(32);
        if let Ok(icon) = winit::window::Icon::from_rgba(image.rgba, image.width, image.height) {
            window.set_window_icon(Some(icon));
            *applied = true;
        }
    });
}
