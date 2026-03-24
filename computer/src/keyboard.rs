use minifb::{Key, Window};

/// Translate the currently held key to a Hack keycode (0 = no key).
pub fn hack_keycode(window: &Window) -> u64 {
    let shift = window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);

    let specials: &[(Key, u64)] = &[
        (Key::Enter, 128),
        (Key::Backspace, 129),
        (Key::Left, 130),
        (Key::Up, 131),
        (Key::Right, 132),
        (Key::Down, 133),
        (Key::Home, 134),
        (Key::End, 135),
        (Key::PageUp, 136),
        (Key::PageDown, 137),
        (Key::Insert, 138),
        (Key::Delete, 139),
        (Key::Escape, 140),
        (Key::F1, 141),
        (Key::F2, 142),
        (Key::F3, 143),
        (Key::F4, 144),
        (Key::F5, 145),
        (Key::F6, 146),
        (Key::F7, 147),
        (Key::F8, 148),
        (Key::F9, 149),
        (Key::F10, 150),
        (Key::F11, 151),
        (Key::F12, 152),
    ];
    for &(key, code) in specials {
        if window.is_key_down(key) {
            return code;
        }
    }

    if window.is_key_down(Key::Space) {
        return b' ' as u64;
    }

    let letters = [
        Key::A,
        Key::B,
        Key::C,
        Key::D,
        Key::E,
        Key::F,
        Key::G,
        Key::H,
        Key::I,
        Key::J,
        Key::K,
        Key::L,
        Key::M,
        Key::N,
        Key::O,
        Key::P,
        Key::Q,
        Key::R,
        Key::S,
        Key::T,
        Key::U,
        Key::V,
        Key::W,
        Key::X,
        Key::Y,
        Key::Z,
    ];
    for (i, &key) in letters.iter().enumerate() {
        if window.is_key_down(key) {
            return (if shift { b'A' } else { b'a' } as usize + i) as u64;
        }
    }

    let digits = [
        Key::Key0,
        Key::Key1,
        Key::Key2,
        Key::Key3,
        Key::Key4,
        Key::Key5,
        Key::Key6,
        Key::Key7,
        Key::Key8,
        Key::Key9,
    ];
    for (i, &key) in digits.iter().enumerate() {
        if window.is_key_down(key) {
            return (b'0' as usize + i) as u64;
        }
    }

    0
}
