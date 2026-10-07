//! LWJGL 2 key codes: what `KeyBinding.keyCode`, `options.txt`'s `key_*`
//! lines and every `Keyboard.getEventKey() == N` test in the vanilla client
//! compare against, with `Keyboard.getKeyName` and the Bevy [`KeyCode`] of
//! the same physical key.
//!
//! The codes are LWJGL's `Keyboard.KEY_*` constants (the engine library, not
//! the vanilla tree); the names are the constants without `KEY_`, which is
//! what `Keyboard.getKeyName` returns and what the options screen shows
//! (GameSettings.java:91-93). The scripted-input mode
//! ([`crate::script`]) names keys the same way.

use bevy::input::keyboard::KeyCode;

/// LWJGL `Keyboard.KEY_ESCAPE`.
pub const KEY_ESCAPE: i32 = 1;
/// `Keyboard.KEY_1`; `KEY_1 + n` selects hotbar slot `n` (Minecraft.java:1073-1077).
pub const KEY_1: i32 = 2;
/// `Keyboard.KEY_S` (F3+S reloads the sounds, Minecraft.java:1040-1042).
pub const KEY_S: i32 = 31;
/// `Keyboard.KEY_LSHIFT`.
pub const KEY_LSHIFT: i32 = 42;
/// `Keyboard.KEY_RSHIFT`.
pub const KEY_RSHIFT: i32 = 54;
/// `Keyboard.KEY_F1`: hide the GUI (Minecraft.java:1044).
pub const KEY_F1: i32 = 59;
/// `Keyboard.KEY_F2`: screenshot (Minecraft.java:660).
pub const KEY_F2: i32 = 60;
/// `Keyboard.KEY_F3`: debug screen (Minecraft.java:1048).
pub const KEY_F3: i32 = 61;
/// `Keyboard.KEY_F5`: third person (Minecraft.java:1052).
pub const KEY_F5: i32 = 63;
/// `Keyboard.KEY_F7` (Minecraft.java:556, :588: the `Display.update` order).
pub const KEY_F7: i32 = 65;
/// `Keyboard.KEY_F8`: smooth camera (Minecraft.java:1056).
pub const KEY_F8: i32 = 66;
/// `Keyboard.KEY_F11`: fullscreen (Minecraft.java:1030).
pub const KEY_F11: i32 = 87;

/// `(code, Keyboard.getKeyName(code), Bevy key)` for every key LWJGL 2
/// names. Keys with no Bevy counterpart map to `None`.
pub const KEYS: &[(i32, &str, Option<KeyCode>)] = &[
    (1, "ESCAPE", Some(KeyCode::Escape)),
    (2, "1", Some(KeyCode::Digit1)),
    (3, "2", Some(KeyCode::Digit2)),
    (4, "3", Some(KeyCode::Digit3)),
    (5, "4", Some(KeyCode::Digit4)),
    (6, "5", Some(KeyCode::Digit5)),
    (7, "6", Some(KeyCode::Digit6)),
    (8, "7", Some(KeyCode::Digit7)),
    (9, "8", Some(KeyCode::Digit8)),
    (10, "9", Some(KeyCode::Digit9)),
    (11, "0", Some(KeyCode::Digit0)),
    (12, "MINUS", Some(KeyCode::Minus)),
    (13, "EQUALS", Some(KeyCode::Equal)),
    (14, "BACK", Some(KeyCode::Backspace)),
    (15, "TAB", Some(KeyCode::Tab)),
    (16, "Q", Some(KeyCode::KeyQ)),
    (17, "W", Some(KeyCode::KeyW)),
    (18, "E", Some(KeyCode::KeyE)),
    (19, "R", Some(KeyCode::KeyR)),
    (20, "T", Some(KeyCode::KeyT)),
    (21, "Y", Some(KeyCode::KeyY)),
    (22, "U", Some(KeyCode::KeyU)),
    (23, "I", Some(KeyCode::KeyI)),
    (24, "O", Some(KeyCode::KeyO)),
    (25, "P", Some(KeyCode::KeyP)),
    (26, "LBRACKET", Some(KeyCode::BracketLeft)),
    (27, "RBRACKET", Some(KeyCode::BracketRight)),
    (28, "RETURN", Some(KeyCode::Enter)),
    (29, "LCONTROL", Some(KeyCode::ControlLeft)),
    (30, "A", Some(KeyCode::KeyA)),
    (31, "S", Some(KeyCode::KeyS)),
    (32, "D", Some(KeyCode::KeyD)),
    (33, "F", Some(KeyCode::KeyF)),
    (34, "G", Some(KeyCode::KeyG)),
    (35, "H", Some(KeyCode::KeyH)),
    (36, "J", Some(KeyCode::KeyJ)),
    (37, "K", Some(KeyCode::KeyK)),
    (38, "L", Some(KeyCode::KeyL)),
    (39, "SEMICOLON", Some(KeyCode::Semicolon)),
    (40, "APOSTROPHE", Some(KeyCode::Quote)),
    (41, "GRAVE", Some(KeyCode::Backquote)),
    (42, "LSHIFT", Some(KeyCode::ShiftLeft)),
    (43, "BACKSLASH", Some(KeyCode::Backslash)),
    (44, "Z", Some(KeyCode::KeyZ)),
    (45, "X", Some(KeyCode::KeyX)),
    (46, "C", Some(KeyCode::KeyC)),
    (47, "V", Some(KeyCode::KeyV)),
    (48, "B", Some(KeyCode::KeyB)),
    (49, "N", Some(KeyCode::KeyN)),
    (50, "M", Some(KeyCode::KeyM)),
    (51, "COMMA", Some(KeyCode::Comma)),
    (52, "PERIOD", Some(KeyCode::Period)),
    (53, "SLASH", Some(KeyCode::Slash)),
    (54, "RSHIFT", Some(KeyCode::ShiftRight)),
    (55, "MULTIPLY", Some(KeyCode::NumpadMultiply)),
    (56, "LMENU", Some(KeyCode::AltLeft)),
    (57, "SPACE", Some(KeyCode::Space)),
    (58, "CAPITAL", Some(KeyCode::CapsLock)),
    (59, "F1", Some(KeyCode::F1)),
    (60, "F2", Some(KeyCode::F2)),
    (61, "F3", Some(KeyCode::F3)),
    (62, "F4", Some(KeyCode::F4)),
    (63, "F5", Some(KeyCode::F5)),
    (64, "F6", Some(KeyCode::F6)),
    (65, "F7", Some(KeyCode::F7)),
    (66, "F8", Some(KeyCode::F8)),
    (67, "F9", Some(KeyCode::F9)),
    (68, "F10", Some(KeyCode::F10)),
    (69, "NUMLOCK", Some(KeyCode::NumLock)),
    (70, "SCROLL", Some(KeyCode::ScrollLock)),
    (71, "NUMPAD7", Some(KeyCode::Numpad7)),
    (72, "NUMPAD8", Some(KeyCode::Numpad8)),
    (73, "NUMPAD9", Some(KeyCode::Numpad9)),
    (74, "SUBTRACT", Some(KeyCode::NumpadSubtract)),
    (75, "NUMPAD4", Some(KeyCode::Numpad4)),
    (76, "NUMPAD5", Some(KeyCode::Numpad5)),
    (77, "NUMPAD6", Some(KeyCode::Numpad6)),
    (78, "ADD", Some(KeyCode::NumpadAdd)),
    (79, "NUMPAD1", Some(KeyCode::Numpad1)),
    (80, "NUMPAD2", Some(KeyCode::Numpad2)),
    (81, "NUMPAD3", Some(KeyCode::Numpad3)),
    (82, "NUMPAD0", Some(KeyCode::Numpad0)),
    (83, "DECIMAL", Some(KeyCode::NumpadDecimal)),
    (87, "F11", Some(KeyCode::F11)),
    (88, "F12", Some(KeyCode::F12)),
    (100, "F13", Some(KeyCode::F13)),
    (101, "F14", Some(KeyCode::F14)),
    (102, "F15", Some(KeyCode::F15)),
    (112, "KANA", Some(KeyCode::KanaMode)),
    (121, "CONVERT", Some(KeyCode::Convert)),
    (123, "NOCONVERT", Some(KeyCode::NonConvert)),
    (125, "YEN", Some(KeyCode::IntlYen)),
    (141, "NUMPADEQUALS", Some(KeyCode::NumpadEqual)),
    (144, "CIRCUMFLEX", None),
    (145, "AT", None),
    (146, "COLON", None),
    (147, "UNDERLINE", None),
    (148, "KANJI", None),
    (149, "STOP", None),
    (150, "AX", None),
    (151, "UNLABELED", None),
    (156, "NUMPADENTER", Some(KeyCode::NumpadEnter)),
    (157, "RCONTROL", Some(KeyCode::ControlRight)),
    (179, "NUMPADCOMMA", Some(KeyCode::NumpadComma)),
    (181, "DIVIDE", Some(KeyCode::NumpadDivide)),
    (183, "SYSRQ", Some(KeyCode::PrintScreen)),
    (184, "RMENU", Some(KeyCode::AltRight)),
    (197, "PAUSE", Some(KeyCode::Pause)),
    (199, "HOME", Some(KeyCode::Home)),
    (200, "UP", Some(KeyCode::ArrowUp)),
    (201, "PRIOR", Some(KeyCode::PageUp)),
    (203, "LEFT", Some(KeyCode::ArrowLeft)),
    (205, "RIGHT", Some(KeyCode::ArrowRight)),
    (207, "END", Some(KeyCode::End)),
    (208, "DOWN", Some(KeyCode::ArrowDown)),
    (209, "NEXT", Some(KeyCode::PageDown)),
    (210, "INSERT", Some(KeyCode::Insert)),
    (211, "DELETE", Some(KeyCode::Delete)),
    (219, "LMETA", Some(KeyCode::SuperLeft)),
    (220, "RMETA", Some(KeyCode::SuperRight)),
    (221, "APPS", Some(KeyCode::ContextMenu)),
    (222, "POWER", Some(KeyCode::Power)),
    (223, "SLEEP", Some(KeyCode::Sleep)),
];

/// `Keyboard.getKeyName(code)`: the constant's name, `None` for an unnamed
/// code (LWJGL returns null).
pub fn key_name(code: i32) -> Option<&'static str> {
    KEYS.iter().find(|k| k.0 == code).map(|k| k.1)
}

/// `Keyboard.getKeyIndex(name)`: the code of a key name, `None` for an
/// unknown name (LWJGL returns `KEY_NONE`, 0).
pub fn key_index(name: &str) -> Option<i32> {
    KEYS.iter().find(|k| k.1 == name).map(|k| k.0)
}

/// The LWJGL code of a Bevy key, `None` when LWJGL 2 has no such key.
pub fn from_bevy(key: KeyCode) -> Option<i32> {
    KEYS.iter().find(|k| k.2 == Some(key)).map(|k| k.0)
}

/// The Bevy key of an LWJGL code.
pub fn to_bevy(code: i32) -> Option<KeyCode> {
    KEYS.iter().find(|k| k.0 == code).and_then(|k| k.2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_constants_match_the_table() {
        assert_eq!(key_name(KEY_ESCAPE), Some("ESCAPE"));
        assert_eq!(key_name(KEY_1), Some("1"));
        assert_eq!(key_name(KEY_S), Some("S"));
        assert_eq!(key_name(KEY_LSHIFT), Some("LSHIFT"));
        assert_eq!(key_name(KEY_RSHIFT), Some("RSHIFT"));
        assert_eq!(key_name(KEY_F1), Some("F1"));
        assert_eq!(key_name(KEY_F2), Some("F2"));
        assert_eq!(key_name(KEY_F3), Some("F3"));
        assert_eq!(key_name(KEY_F5), Some("F5"));
        assert_eq!(key_name(KEY_F7), Some("F7"));
        assert_eq!(key_name(KEY_F8), Some("F8"));
        assert_eq!(key_name(KEY_F11), Some("F11"));
    }

    #[test]
    fn the_vanilla_default_bindings_resolve() {
        // GameSettings.java:28-37
        for (code, name, bevy) in [
            (17, "W", KeyCode::KeyW),
            (30, "A", KeyCode::KeyA),
            (31, "S", KeyCode::KeyS),
            (32, "D", KeyCode::KeyD),
            (57, "SPACE", KeyCode::Space),
            (18, "E", KeyCode::KeyE),
            (16, "Q", KeyCode::KeyQ),
            (20, "T", KeyCode::KeyT),
            (33, "F", KeyCode::KeyF),
            (42, "LSHIFT", KeyCode::ShiftLeft),
        ] {
            assert_eq!(key_name(code), Some(name));
            assert_eq!(key_index(name), Some(code));
            assert_eq!(from_bevy(bevy), Some(code));
            assert_eq!(to_bevy(code), Some(bevy));
        }
    }

    #[test]
    fn codes_and_names_are_unique() {
        for (i, a) in KEYS.iter().enumerate() {
            for b in &KEYS[i + 1..] {
                assert_ne!(a.0, b.0);
                assert_ne!(a.1, b.1);
                if a.2.is_some() {
                    assert_ne!(a.2, b.2);
                }
            }
        }
    }
}
