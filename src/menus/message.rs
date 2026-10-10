use super::*;

/// A show_two_button_dialog box: a title bar, a message in an inset, and
/// one or two buttons along y 0xd4.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub title: String,
    pub text: String,
    pub first: &'static str,
    pub second: Option<&'static str>,
}

impl Message {
    /// show_continue_message.
    pub fn notice(title: &str, text: &str) -> Self {
        Self {
            title: title.into(),
            text: text.into(),
            first: "CONTINUE",
            second: None,
        }
    }
    /// show_yes_no_dialog.
    pub fn question(title: &str, text: &str) -> Self {
        Self {
            title: title.into(),
            text: text.into(),
            first: "YES",
            second: Some("NO"),
        }
    }

    pub(super) fn buttons(&self, font: &Font) -> (Clip, Option<Clip>) {
        let second = self.second.map(|label| {
            let x = 0x168 - ((font.width(label) + 7) & !7);
            button_rect(font, label, x)
        });
        (button_rect(font, self.first, 200), second)
    }

    /// Some(true) for the first button, Some(false) for the second.
    pub fn choice(&self, font: &Font, cursor: (i16, i16)) -> Option<bool> {
        let (first, second) = self.buttons(font);
        if inside(first, cursor) {
            Some(true)
        } else if second.is_some_and(|clip| inside(clip, cursor)) {
            Some(false)
        } else {
            None
        }
    }

    pub fn render(&self, screen: &mut Screen, assets: &Assets, font: &Font, hovered: Option<bool>) {
        bordered_panel(screen, assets, (0xb0, 0x70, 0x190, 0xf8));
        label_bar(screen, assets, font, &self.title, 0xb8, 0x74, 0xd0);
        inset(screen, assets, (0xb8, 0x90, 0xd0, 0x5a));
        font.wrapped(screen, &self.text, 0xbc, 0x94, 200, assets.shadow);
        let (_, second) = self.buttons(font);
        button(
            screen,
            assets,
            font,
            self.first,
            200,
            0xd4,
            hovered == Some(true),
        );
        if let (Some(label), Some(clip)) = (self.second, second) {
            button(
                screen,
                assets,
                font,
                label,
                clip.0,
                0xd4,
                hovered == Some(false),
            );
        }
    }
}
