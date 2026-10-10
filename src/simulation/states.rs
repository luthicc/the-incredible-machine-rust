// Object.state remains the native u16 in saves and traces. These enums name
// only the small, well-defined state machines used by simulation callbacks.
macro_rules! native_part_state {
    ($name:ident { $($variant:ident = $value:literal),+ $(,)? }) => {
        #[repr(u16)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub(super) enum $name {
            $($variant = $value),+
        }

        impl $name {
            pub(super) const fn from_raw(raw: u16) -> Option<Self> {
                match raw {
                    $($value => Some(Self::$variant),)+
                    _ => None,
                }
            }

            pub(super) const fn raw(self) -> u16 {
                self as u16
            }
        }
    };
}

pub(super) use native_part_state;
