use super::*;
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, Projection, Viewport as CameraViewport};
use bevy::image::ImageSampler;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Layer for the pointer and loupe, drawn above the zoomed playfield.
pub(super) const OVERLAY_LAYER: usize = 1;
/// Loupe diameter in logical pixels and its enlargement of what is on screen.
const LOUPE_SIZE: u32 = 200;
const LOUPE_MAGNIFY: f32 = 2.0;
/// Gap between the fingertip and the loupe's edge.
const LOUPE_GAP: f32 = 40.0;

/// Draws the screen again, enlarged, clipped to the playfield.
#[derive(Component)]
pub(super) struct PlayfieldCamera;

#[derive(Component)]
pub(super) struct LoupeSprite;

/// The latest composed 640x480 frame, for the loupe.
#[derive(Resource, Default)]
pub(super) struct LastFrame(pub Vec<u8>);

pub(super) fn spawn_magnifiers(commands: &mut Commands, images: &mut Assets<Image>) {
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            is_active: false,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        PlayfieldCamera,
    ));
    commands.spawn((
        Camera2d,
        Camera {
            order: 2,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        RenderLayers::layer(OVERLAY_LAYER),
    ));
    let mut loupe = Image::new_fill(
        Extent3d {
            width: LOUPE_SIZE,
            height: LOUPE_SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    loupe.sampler = ImageSampler::nearest();
    commands.spawn((
        Sprite::from_image(images.add(loupe)),
        Transform::from_xyz(0.0, 0.0, 5.0),
        Visibility::Hidden,
        RenderLayers::layer(OVERLAY_LAYER),
        LoupeSprite,
    ));
}

/// Aim the playfield camera at the zoomed region of the screen sprite.
pub(super) fn place_playfield_camera(
    windows: Query<&Window>,
    game: Res<Game>,
    zoom: Res<ZoomState>,
    mut cameras: Query<(&mut Camera, &mut Projection, &mut Transform), With<PlayfieldCamera>>,
) {
    let (Ok(window), Ok((mut camera, mut projection, mut transform))) =
        (windows.single(), cameras.single_mut())
    else {
        return;
    };
    let zoom = zoom.0;
    let active = zoom.active() && game.mode == Mode::Puzzle;
    if camera.is_active != active {
        camera.is_active = active;
    }
    if !active {
        return;
    }
    let base = base_viewport(window, &game);
    let dpi = window.scale_factor();
    let (l, t, r, b) = zoom.region;
    let to_physical = |x: f32, y: f32| {
        (
            ((base.origin.0 + x * base.scale) * dpi).round().max(0.0) as u32,
            ((base.origin.1 + y * base.scale_y) * dpi).round().max(0.0) as u32,
        )
    };
    let (x0, y0) = to_physical(l, t);
    let (x1, y1) = to_physical(r, b);
    let (x1, y1) = (
        x1.min(window.physical_width()),
        y1.min(window.physical_height()),
    );
    if x1 <= x0 || y1 <= y0 {
        camera.is_active = false;
        return;
    }
    camera.viewport = Some(CameraViewport {
        physical_position: UVec2::new(x0, y0),
        physical_size: UVec2::new(x1 - x0, y1 - y0),
        ..default()
    });
    if let Projection::Orthographic(ortho) = &mut *projection {
        ortho.scale = 1.0 / zoom.factor;
    }
    // World coordinates of the zoom centre, as the screen sprite is laid out.
    let (cx, cy) = zoom.center;
    transform.translation.x = base.origin.0 + cx * base.scale - window.width() / 2.0;
    transform.translation.y = window.height() / 2.0 - base.origin.1 - cy * base.scale_y;
}

/// A round magnifier above the finger, showing exactly what it is on.
pub(super) fn place_loupe(
    windows: Query<&Window>,
    game: Res<Game>,
    touch: Res<TouchPointer>,
    zoom: Res<ZoomState>,
    frame: Res<LastFrame>,
    mut images: ResMut<Assets<Image>>,
    mut loupes: Query<(&Sprite, &mut Transform, &mut Visibility), With<LoupeSprite>>,
) {
    let Ok((sprite, mut transform, mut visibility)) = loupes.single_mut() else {
        return;
    };
    let finger = touch.position.filter(|_| touch.down && !touch.pinching);
    let (Ok(window), Some(finger)) = (windows.single(), finger) else {
        visibility.set_if_neq(Visibility::Hidden);
        return;
    };
    if game.mode != Mode::Puzzle || frame.0.len() != 640 * 480 * 4 {
        visibility.set_if_neq(Visibility::Hidden);
        return;
    }
    let Some(image) = images.get_mut(&sprite.image) else {
        return;
    };
    let size = LOUPE_SIZE as usize;
    let radius = LOUPE_SIZE as f32 / 2.0;
    let mut rgba = vec![0u8; size * size * 4];
    for j in 0..size {
        for i in 0..size {
            let (dx, dy) = (i as f32 + 0.5 - radius, j as f32 + 0.5 - radius);
            let distance = (dx * dx + dy * dy).sqrt();
            let pixel = &mut rgba[(j * size + i) * 4..][..4];
            if distance > radius {
                continue;
            }
            if distance > radius - 3.0 {
                pixel.copy_from_slice(&[0xf4, 0xd1, 0x55, 0xff]);
                continue;
            }
            // A small cross marks the exact point under the finger.
            let (ax, ay) = (dx.abs(), dy.abs());
            if (ax < 0.6 && (1.5..6.0).contains(&ay)) || (ay < 0.6 && (1.5..6.0).contains(&ax)) {
                pixel.copy_from_slice(&[0, 0, 0, 0xff]);
                continue;
            }
            let point = finger + Vec2::new(dx, dy) / LOUPE_MAGNIFY;
            let colour = window_to_source(window, &game, &zoom, point)
                .map(|(x, y)| {
                    let at = ((y as usize).min(479) * 640 + (x as usize).min(639)) * 4;
                    [frame.0[at], frame.0[at + 1], frame.0[at + 2], 0xff]
                })
                .unwrap_or([0, 0, 0, 0xff]);
            pixel.copy_from_slice(&colour);
        }
    }
    image.data = Some(rgba);
    // Above the finger; near the top edge it stays at the top and moves to
    // the finger's left (or right, without room) so the finger never covers it.
    let mut centre = finger - Vec2::new(0.0, radius + LOUPE_GAP);
    if centre.y < radius {
        centre.y = radius;
        centre.x = finger.x - radius - LOUPE_GAP;
        if centre.x < radius {
            centre.x = finger.x + radius + LOUPE_GAP;
        }
    }
    centre.x = centre
        .x
        .clamp(radius, (window.width() - radius).max(radius));
    transform.translation.x = centre.x - window.width() / 2.0;
    transform.translation.y = window.height() / 2.0 - centre.y;
    visibility.set_if_neq(Visibility::Visible);
}
