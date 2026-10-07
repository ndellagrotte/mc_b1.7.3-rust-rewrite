//! Input in vanilla's shape: LWJGL's buffered `Keyboard`/`Mouse` events,
//! which `Minecraft.runTick` drains once per tick (Minecraft.java:995-1131),
//! and their live state (`Keyboard.isKeyDown`, `Mouse.isButtonDown`);
//! `MouseHelper` (MouseHelper.java), `MovementInputFromOptions`
//! (MovementInputFromOptions.java) and `MouseFilter` (MouseFilter.java).
//!
//! Real devices feed [`InputQueue`] through [`collect_device_input`]
//! (Bevy keyboard, mouse-button, wheel and motion messages); the scripted
//! mode ([`crate::script`]) feeds it instead and real devices are ignored.
//! Keys are LWJGL codes ([`crate::keys`]); mouse buttons are LWJGL's (0
//! left, 1 right, 2 middle); mouse coordinates are LWJGL's (origin at the
//! bottom left of the window, in physical pixels); vertical motion is
//! positive upwards, as `Mouse.getDY()`.
//!
//! LWJGL 2 delivers no key-repeat events unless
//! `Keyboard.enableRepeatEvents(true)` (the chat and sign screens turn it
//! on, WP-3.6): [`InputQueue::repeat_events`].

use std::collections::{BTreeSet, VecDeque};

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::{AccumulatedMouseMotion, MouseButtonInput, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use crate::keys;
use crate::settings::{
    GameSettings, KEY_BACK, KEY_FORWARD, KEY_JUMP, KEY_LEFT, KEY_RIGHT, KEY_SNEAK,
};

/// One buffered LWJGL event.
#[derive(Clone, Debug, PartialEq)]
pub enum InputEvent {
    /// `Keyboard.next()`: `getEventKey()`, `getEventKeyState()`,
    /// `getEventCharacter()` (`'\0'` when none).
    Key {
        /// LWJGL key code.
        key: i32,
        /// Pressed (true) or released.
        down: bool,
        /// The typed character.
        ch: char,
    },
    /// `Mouse.next()` with a button: `getEventButton()`,
    /// `getEventButtonState()`, `getEventX()`, `getEventY()`.
    MouseButton {
        /// 0 left, 1 right, 2 middle.
        button: i32,
        /// Pressed or released.
        down: bool,
        /// LWJGL x (from the left).
        x: i32,
        /// LWJGL y (from the bottom).
        y: i32,
    },
    /// `Mouse.next()` of a wheel turn: `getEventButton()` is -1 and
    /// `getEventDWheel()` is non-zero (±120 per notch).
    MouseWheel {
        /// `getEventDWheel()`.
        dwheel: i32,
        /// LWJGL x.
        x: i32,
        /// LWJGL y.
        y: i32,
    },
}

/// The LWJGL event buffers and live state (module docs).
#[derive(Clone, Debug, Default)]
pub struct InputQueue {
    /// `Keyboard`'s buffer.
    pub keyboard: VecDeque<InputEvent>,
    /// `Mouse`'s buffer.
    pub mouse: VecDeque<InputEvent>,
    /// `Keyboard.isKeyDown`.
    pub keys_down: BTreeSet<i32>,
    /// `Mouse.isButtonDown(0..3)`.
    pub buttons_down: [bool; 3],
    /// `Mouse.getX()` / `getY()` (LWJGL coordinates).
    pub mouse_x: i32,
    /// See [`InputQueue::mouse_x`].
    pub mouse_y: i32,
    /// Mouse motion since the last `MouseHelper.mouseXYChange`
    /// (`Mouse.getDX()`, `getDY()`, y up).
    pub pending_dx: f32,
    /// See [`InputQueue::pending_dx`].
    pub pending_dy: f32,
    /// `Keyboard.enableRepeatEvents` (module docs).
    pub repeat_events: bool,
}

impl InputQueue {
    /// Records a key event and its live state.
    pub fn push_key(&mut self, key: i32, down: bool, ch: char) {
        if down {
            self.keys_down.insert(key);
        } else {
            self.keys_down.remove(&key);
        }
        self.keyboard.push_back(InputEvent::Key { key, down, ch });
    }

    /// Records a mouse-button event and its live state.
    pub fn push_button(&mut self, button: i32, down: bool) {
        if (0..3).contains(&button) {
            self.buttons_down[button as usize] = down;
        }
        self.mouse.push_back(InputEvent::MouseButton {
            button,
            down,
            x: self.mouse_x,
            y: self.mouse_y,
        });
    }

    /// Records a wheel event.
    pub fn push_wheel(&mut self, dwheel: i32) {
        self.mouse.push_back(InputEvent::MouseWheel {
            dwheel,
            x: self.mouse_x,
            y: self.mouse_y,
        });
    }

    /// `Keyboard.isKeyDown(key)`.
    pub fn is_key_down(&self, key: i32) -> bool {
        self.keys_down.contains(&key)
    }

    /// `Mouse.isButtonDown(button)`.
    pub fn is_button_down(&self, button: i32) -> bool {
        (0..3).contains(&button) && self.buttons_down[button as usize]
    }

    /// Forgets every buffered event (a screen's `handleInput` drained them).
    pub fn clear_events(&mut self) {
        self.keyboard.clear();
        self.mouse.clear();
    }
}

/// `MouseHelper`: the per-frame mouse delta and the cursor grab.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MouseHelper {
    /// `deltaX`.
    // src: MouseHelper.java:12
    pub delta_x: i32,
    /// `deltaY` (up positive).
    // src: MouseHelper.java:13
    pub delta_y: i32,
    /// `Mouse.setGrabbed`'s state, which [`apply_cursor_grab`] mirrors onto
    /// the window. `field_1115_e` (10, never read) and the blank `Cursor`
    /// are not ported.
    pub grabbed: bool,
    /// Set by `ungrabMouseCursor`: centre the pointer
    /// (`Mouse.setCursorPosition(w / 2, h / 2)`).
    pub recenter: bool,
}

impl MouseHelper {
    /// `grabMouseCursor()`.
    // src: MouseHelper.java:31-35
    pub fn grab_mouse_cursor(&mut self) {
        self.grabbed = true;
        self.delta_x = 0;
        self.delta_y = 0;
    }

    /// `ungrabMouseCursor()`.
    // src: MouseHelper.java:37-40
    pub fn ungrab_mouse_cursor(&mut self) {
        self.recenter = true;
        self.grabbed = false;
    }

    /// `mouseXYChange()`: takes the motion since the last call. LWJGL's
    /// `getDX`/`getDY` are whole pixels; the fraction is carried.
    // src: MouseHelper.java:42-45
    pub fn mouse_xy_change(&mut self, q: &mut InputQueue) {
        self.delta_x = q.pending_dx.trunc() as i32;
        self.delta_y = q.pending_dy.trunc() as i32;
        q.pending_dx -= self.delta_x as f32;
        q.pending_dy -= self.delta_y as f32;
    }
}

/// `MovementInputFromOptions`: the movement keys' states, set by key events
/// (`checkKeyForMovementInput`), read into the player's
/// [`MovementInput`](mc_sim::entity::player::MovementInput) every tick.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MovementInputFromOptions {
    /// `movementKeyStates` (forward, back, left, right, jump, sneak; ten
    /// slots, six used).
    // src: MovementInputFromOptions.java:4
    pub movement_key_states: [bool; 10],
}

impl MovementInputFromOptions {
    /// `checkKeyForMovementInput(key, state)`: a later matching binding wins
    /// (the tests are not exclusive).
    // src: MovementInputFromOptions.java:11-41
    pub fn check_key_for_movement_input(&mut self, s: &GameSettings, var1: i32, var2: bool) {
        let mut var3: i8 = -1;
        if var1 == s.key(KEY_FORWARD) {
            var3 = 0;
        }
        if var1 == s.key(KEY_BACK) {
            var3 = 1;
        }
        if var1 == s.key(KEY_LEFT) {
            var3 = 2;
        }
        if var1 == s.key(KEY_RIGHT) {
            var3 = 3;
        }
        if var1 == s.key(KEY_JUMP) {
            var3 = 4;
        }
        if var1 == s.key(KEY_SNEAK) {
            var3 = 5;
        }
        if var3 >= 0 {
            self.movement_key_states[var3 as usize] = var2;
        }
    }

    /// `resetKeyState()`.
    // src: MovementInputFromOptions.java:43-48
    pub fn reset_key_state(&mut self) {
        self.movement_key_states = [false; 10];
    }

    /// `updatePlayerMoveState(player)`: the `MovementInput` fields from the
    /// key states (the sneak scaling is in
    /// [`MovementInput::from_keys`](mc_sim::entity::player::MovementInput::from_keys)).
    // src: MovementInputFromOptions.java:50-76
    pub fn update_player_move_state(&self) -> mc_sim::entity::player::MovementInput {
        let mut forward = 0.0f32;
        let mut strafe = 0.0f32;
        if self.movement_key_states[0] {
            forward += 1.0;
        }
        if self.movement_key_states[1] {
            forward -= 1.0;
        }
        if self.movement_key_states[2] {
            strafe += 1.0;
        }
        if self.movement_key_states[3] {
            strafe -= 1.0;
        }
        mc_sim::entity::player::MovementInput::from_keys(
            forward,
            strafe,
            self.movement_key_states[4],
            self.movement_key_states[5],
        )
    }
}

/// `MouseFilter`: the smooth camera's easing (F8).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MouseFilter {
    /// `targetValue`.
    target_value: f32,
    /// `remainingValue`.
    remaining_value: f32,
    /// `lastAmount`.
    last_amount: f32,
}

impl MouseFilter {
    /// `smooth(delta, factor)`.
    // src: MouseFilter.java:8-18
    pub fn smooth(&mut self, mut var1: f32, var2: f32) -> f32 {
        self.target_value += var1;
        var1 = (self.target_value - self.remaining_value) * var2;
        self.last_amount += (var1 - self.last_amount) * 0.5;
        if var1 > 0.0 && var1 > self.last_amount || var1 < 0.0 && var1 < self.last_amount {
            var1 = self.last_amount;
        }
        self.remaining_value += var1;
        var1
    }
}

/// Whether real devices are read (false in the scripted mode).
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeviceInput(pub bool);

impl Default for DeviceInput {
    fn default() -> Self {
        DeviceInput(true)
    }
}

/// Bevy's keyboard, mouse-button, wheel and motion input into the
/// [`InputQueue`] of the game (module docs).
#[allow(clippy::too_many_arguments)]
pub fn collect_device_input(
    device: Res<DeviceInput>,
    mut game: NonSendMut<crate::game::Game>,
    mut keys_in: MessageReader<KeyboardInput>,
    mut buttons_in: MessageReader<MouseButtonInput>,
    mut wheel_in: MessageReader<MouseWheel>,
    motion: Res<AccumulatedMouseMotion>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    if !device.0 {
        keys_in.clear();
        buttons_in.clear();
        wheel_in.clear();
        return;
    }
    let q = &mut game.input;
    if let Ok(w) = windows.single()
        && let Some(p) = w.physical_cursor_position()
    {
        q.mouse_x = p.x as i32;
        q.mouse_y = w.physical_height() as i32 - p.y as i32 - 1;
    }
    for e in keys_in.read() {
        if e.repeat && !q.repeat_events {
            continue;
        }
        let Some(code) = keys::from_bevy(e.key_code) else {
            continue;
        };
        let ch = match &e.logical_key {
            Key::Character(s) => s.chars().next().unwrap_or('\0'),
            Key::Space => ' ',
            _ => '\0',
        };
        q.push_key(code, e.state == ButtonState::Pressed, ch);
    }
    for e in buttons_in.read() {
        let b = match e.button {
            MouseButton::Left => 0,
            MouseButton::Right => 1,
            MouseButton::Middle => 2,
            _ => continue,
        };
        q.push_button(b, e.state == ButtonState::Pressed);
    }
    for e in wheel_in.read() {
        // One line (notch) is 120, LWJGL's Windows unit; pixels are rare.
        let d = match e.unit {
            MouseScrollUnit::Line => e.y * 120.0,
            MouseScrollUnit::Pixel => e.y,
        };
        if d != 0.0 {
            q.push_wheel(d as i32);
        }
    }
    q.pending_dx += motion.delta.x;
    // Bevy's y grows downwards; LWJGL's getDY upwards.
    q.pending_dy -= motion.delta.y;
}

/// Mirrors `Mouse.setGrabbed` onto the primary window: locked (confined
/// where locking is unsupported) and hidden while grabbed. The scripted mode
/// never grabs the desktop pointer.
pub fn apply_cursor_grab(
    device: Res<DeviceInput>,
    mut game: NonSendMut<crate::game::Game>,
    mut windows: Query<(&mut Window, &mut CursorOptions), With<PrimaryWindow>>,
) {
    let Ok((mut window, mut cursor)) = windows.single_mut() else {
        return;
    };
    let grab = game.mouse_helper.grabbed && device.0;
    let mode = if grab {
        CursorGrabMode::Locked
    } else {
        CursorGrabMode::None
    };
    if cursor.grab_mode != mode {
        cursor.grab_mode = mode;
        cursor.visible = !grab;
    }
    if game.mouse_helper.recenter {
        game.mouse_helper.recenter = false;
        if device.0 {
            let (w, h) = (window.width(), window.height());
            window.set_cursor_position(Some(Vec2::new(w / 2.0, h / 2.0)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movement_keys_follow_the_bindings() {
        let s = GameSettings::default();
        let mut m = MovementInputFromOptions::default();
        m.check_key_for_movement_input(&s, 17, true); // W
        m.check_key_for_movement_input(&s, 30, true); // A
        let mi = m.update_player_move_state();
        assert_eq!((mi.move_forward, mi.move_strafe), (1.0, 1.0));
        m.check_key_for_movement_input(&s, 42, true); // sneak
        let mi = m.update_player_move_state();
        assert!(mi.sneak);
        assert_eq!(mi.move_forward, (1.0f64 * 0.3) as f32);
        m.check_key_for_movement_input(&s, 31, true); // S cancels W
        m.check_key_for_movement_input(&s, 32, true); // D cancels A
        let mi = m.update_player_move_state();
        assert_eq!((mi.move_forward, mi.move_strafe), (0.0, 0.0));
        m.reset_key_state();
        assert_eq!(m.update_player_move_state(), Default::default());
    }

    #[test]
    fn mouse_filter_eases() {
        let mut f = MouseFilter::default();
        // target 10, factor 0.5 → raw 5, lastAmount 2.5 → capped to 2.5
        assert_eq!(f.smooth(10.0, 0.5), 2.5);
        // target 10, remaining 2.5 → raw 3.75, lastAmount 3.125 → 3.125
        assert_eq!(f.smooth(0.0, 0.5), 3.125);
    }

    #[test]
    fn mouse_helper_takes_whole_pixels() {
        let mut q = InputQueue {
            pending_dx: 3.75,
            pending_dy: -2.5,
            ..Default::default()
        };
        let mut h = MouseHelper::default();
        h.mouse_xy_change(&mut q);
        assert_eq!((h.delta_x, h.delta_y), (3, -2));
        assert_eq!((q.pending_dx, q.pending_dy), (0.75, -0.5));
        h.grab_mouse_cursor();
        assert!(h.grabbed);
        assert_eq!((h.delta_x, h.delta_y), (0, 0));
    }

    #[test]
    fn queue_tracks_live_state() {
        let mut q = InputQueue::default();
        q.push_key(17, true, 'w');
        q.push_button(0, true);
        assert!(q.is_key_down(17));
        assert!(q.is_button_down(0));
        q.push_key(17, false, 'w');
        assert!(!q.is_key_down(17));
        assert_eq!(q.keyboard.len(), 2);
        q.clear_events();
        assert!(q.keyboard.is_empty() && q.mouse.is_empty());
        assert!(q.is_button_down(0));
    }
}
