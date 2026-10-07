//! `GuiScreen` (GuiScreen.java): the [`Screen`] trait every screen
//! implements, its fields ([`ScreenBase`]), the default behaviour Java
//! subclasses reach with `super.` ([`default_draw_screen`],
//! [`default_key_typed`], [`default_mouse_clicked`],
//! [`default_mouse_moved_or_up`]), the backgrounds, and [`ScreenCtx`]:
//! what a screen reaches through `this.mc` (the game, the font, the
//! translations, `displayGuiScreen`, the sound manager).
//!
//! # Screen objects and the shell's slot
//!
//! The shell keeps `Minecraft.currentScreen` as a description
//! ([`crate::app::GuiScreen`]: kind, size, the two flags it reads). The GUI
//! binds one screen object to it ([`Binder`]): an object a screen handed to
//! [`ScreenCtx::display_gui_screen`], or, for a screen the shell opened by
//! itself (Escape's `GuiIngameMenu`, `GuiGameOver`, the inventory key's
//! `GuiInventory`, …), one made by the [`ScreenRegistry`] from its kind.
//! A new object is told `setWorldAndResolution` (its size, an empty
//! control list, `initGui`); the replaced one `onGuiClosed`. The binder
//! catches up with the slot ([`ScreenCtx::sync`]) right after every call a
//! screen makes into `displayGuiScreen`, and at the start of the screen
//! hook and of the frame's drawing (for screens the shell opened or
//! closed).
//!
//! A screen method runs on the object taken out of the binder. If it
//! replaces itself, its `onGuiClosed` runs when the method returns (Java
//! runs it inside `displayGuiScreen`, before the new screen's `initGui`).
//! `mc.displayGuiScreen(this)` and `new GuiYesNo(this, …)` hand the
//! running object itself to the slot: a screen asks with
//! [`ScreenCtx::display_this`] / [`ScreenCtx::display_wrapping_this`] and
//! whoever owns the running object resolves it when the method returns
//! (the dispatcher for the current screen, [`GuiYesNo`] for its parent).
//!
//! [`GuiYesNo`]: super::widgets::yesno::GuiYesNo

use std::collections::BTreeMap;

use bevy::prelude::*;

use super::batch::{GuiGl, gl};
use super::draw;
use super::font::FontRenderer;
use super::widgets::button::GuiButton;
use crate::app::{GuiScreen as ScreenSlot, ScreenKind};
use crate::events::ClientSound;
use crate::game::Game;
use crate::keys;
use crate::lang::StringTranslate;

/// `/gui/background.png`: the dirt tile of menus without a world.
// src: GuiScreen.java:151
pub const BACKGROUND_PNG: &str = "/gui/background.png";

/// `GuiScreen`'s fields.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScreenBase {
    /// `width` (scaled pixels, set by `setWorldAndResolution`).
    // src: GuiScreen.java:15
    pub width: i32,
    /// `height`.
    // src: GuiScreen.java:16
    pub height: i32,
    /// `controlList`: the buttons.
    // src: GuiScreen.java:17
    pub control_list: Vec<GuiButton>,
    /// `allowUserInput`.
    // src: GuiScreen.java:18
    pub allow_user_input: bool,
    /// `selectedButton` (an index into `control_list`; module docs of
    /// [`default_mouse_clicked`]).
    // src: GuiScreen.java:21
    pub selected_button: Option<usize>,
    /// `zLevel` (Gui.java:6).
    pub z_level: f32,
}

/// A screen (`GuiScreen` and its subclasses). The methods with defaults are
/// `GuiScreen`'s; an override that calls `super.x(…)` calls the matching
/// `default_*` function.
pub trait Screen: Send + Sync + 'static {
    /// The fields.
    fn base(&self) -> &ScreenBase;
    /// The fields.
    fn base_mut(&mut self) -> &mut ScreenBase;
    /// The class, as the shell's slot names it.
    fn kind(&self) -> ScreenKind;

    /// `doesGuiPauseGame()`.
    // src: GuiScreen.java:163-165
    fn does_gui_pause_game(&self) -> bool {
        true
    }

    /// `initGui()`.
    // src: GuiScreen.java:87-88
    fn init_gui(&mut self, _ctx: &mut ScreenCtx) {}

    /// `drawScreen(mouseX, mouseY, partialTicks)`.
    // src: GuiScreen.java:23-29
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, partial: f32) {
        let _ = partial;
        default_draw_screen(self.base_mut(), ctx, g, mx, my);
    }

    /// `keyTyped(char, key)`.
    // src: GuiScreen.java:31-37
    fn key_typed(&mut self, ctx: &mut ScreenCtx, ch: char, key: i32) {
        let _ = ch;
        default_key_typed(ctx, key);
    }

    /// `mouseClicked(x, y, button)`.
    // src: GuiScreen.java:52-64
    fn mouse_clicked(&mut self, ctx: &mut ScreenCtx, x: i32, y: i32, button: i32) {
        default_mouse_clicked(self, ctx, x, y, button);
    }

    /// `mouseMovedOrUp(x, y, button)` (button -1 for motion and the wheel).
    // src: GuiScreen.java:66-72
    fn mouse_moved_or_up(&mut self, ctx: &mut ScreenCtx, x: i32, y: i32, button: i32) {
        default_mouse_moved_or_up(self.base_mut(), ctx, x, y, button);
    }

    /// `actionPerformed(button)`, with the button's index in
    /// `control_list`.
    // src: GuiScreen.java:74-75
    fn action_performed(&mut self, _ctx: &mut ScreenCtx, _button: usize) {}

    /// `updateScreen()`, every tick while open.
    // src: GuiScreen.java:128-129
    fn update_screen(&mut self, _ctx: &mut ScreenCtx) {}

    /// `onGuiClosed()`.
    // src: GuiScreen.java:131-132
    fn on_gui_closed(&mut self, _ctx: &mut ScreenCtx) {}

    /// `deleteWorld(result, id)`: the answer of a [`GuiYesNo`] this screen
    /// opened (`true` for its first button).
    ///
    /// [`GuiYesNo`]: super::widgets::yesno::GuiYesNo
    // src: GuiScreen.java:167-168
    fn delete_world(&mut self, _ctx: &mut ScreenCtx, _result: bool, _id: i32) {}

    /// `selectNextField()` (a text field's Tab).
    // src: GuiScreen.java:170-171
    fn select_next_field(&mut self, _ctx: &mut ScreenCtx) {}

    /// The shell's description of this screen.
    fn slot(&self) -> ScreenSlot {
        ScreenSlot {
            kind: self.kind(),
            width: 0,
            height: 0,
            allow_user_input: self.base().allow_user_input,
            pauses_game: self.does_gui_pause_game(),
        }
    }
}

/// `GuiScreen.drawScreen`: every button of the control list.
// src: GuiScreen.java:23-29
pub fn default_draw_screen(
    b: &mut ScreenBase,
    ctx: &mut ScreenCtx,
    g: &mut GuiGl,
    mx: i32,
    my: i32,
) {
    let mut i = 0;
    while i < b.control_list.len() {
        b.control_list[i].draw_button(ctx, g, mx, my);
        i += 1;
    }
}

/// `GuiScreen.keyTyped`: Escape closes the screen and gives the game the
/// focus.
// src: GuiScreen.java:31-37
pub fn default_key_typed(ctx: &mut ScreenCtx, key: i32) {
    if key == keys::KEY_ESCAPE {
        ctx.display_gui_screen(None);
        ctx.set_ingame_focus();
    }
}

/// `GuiScreen.mouseClicked`: the left button presses every button under the
/// pointer, in list order, each with a click and `actionPerformed`. The
/// list is re-read every step (an action may rebuild it). `selectedButton`
/// is kept as an index, so after a rebuild it names the new list's button
/// at that place where Java keeps the old object (only `GuiSlider`'s
/// `mouseReleased` acts on it: it stops dragging).
// src: GuiScreen.java:52-64
pub fn default_mouse_clicked<S: Screen + ?Sized>(
    s: &mut S,
    ctx: &mut ScreenCtx,
    x: i32,
    y: i32,
    button: i32,
) {
    if button == 0 {
        let mut i = 0;
        while i < s.base().control_list.len() {
            if s.base_mut().control_list[i].mouse_pressed(ctx, x, y) {
                s.base_mut().selected_button = Some(i);
                ctx.play_sound_fx("random.click", 1.0, 1.0);
                s.action_performed(ctx, i);
            }
            i += 1;
        }
    }
}

/// `GuiScreen.mouseMovedOrUp`: releasing the left button releases the
/// button it pressed.
// src: GuiScreen.java:66-72
pub fn default_mouse_moved_or_up(
    b: &mut ScreenBase,
    _ctx: &mut ScreenCtx,
    x: i32,
    y: i32,
    button: i32,
) {
    if let Some(i) = b.selected_button
        && button == 0
    {
        if let Some(btn) = b.control_list.get_mut(i) {
            btn.mouse_released(x, y);
        }
        b.selected_button = None;
    }
}

/// `drawDefaultBackground()`: `drawWorldBackground(0)`.
// src: GuiScreen.java:134-136
pub fn draw_default_background(b: &ScreenBase, ctx: &ScreenCtx, g: &mut GuiGl) {
    draw_world_background(b, ctx, g, 0);
}

/// `drawWorldBackground(v)`: over a world a dark translucent gradient
/// (`0xC0101010` to `0xD0101010`), else the dirt.
// src: GuiScreen.java:138-145
pub fn draw_world_background(b: &ScreenBase, ctx: &ScreenCtx, g: &mut GuiGl, var1: i32) {
    if ctx.game.the_world.is_some() {
        draw::draw_gradient_rect(g, 0, 0, b.width, b.height, -1072689136, -804253680);
    } else {
        draw_background(b, g, var1);
    }
}

/// `drawBackground(v)`: `/gui/background.png` tiled every 32 pixels,
/// tinted `0x404040`, scrolled down `v` tiles.
// src: GuiScreen.java:147-161
#[allow(clippy::identity_op)]
pub fn draw_background(b: &ScreenBase, g: &mut GuiGl, var1: i32) {
    g.disable(gl::LIGHTING);
    g.disable(gl::FOG);
    g.bind_texture(BACKGROUND_PNG);
    g.color4f(1.0, 1.0, 1.0, 1.0);
    let var3 = 32.0f32;
    let (w, h) = (b.width, b.height);
    g.start_drawing_quads();
    g.set_color_opaque_i(4210752);
    g.add_vertex_with_uv(
        0.0,
        h as f64,
        0.0,
        0.0,
        (h as f32 / var3 + var1 as f32) as f64,
    );
    g.add_vertex_with_uv(
        w as f64,
        h as f64,
        0.0,
        (w as f32 / var3) as f64,
        (h as f32 / var3 + var1 as f32) as f64,
    );
    g.add_vertex_with_uv(
        w as f64,
        0.0,
        0.0,
        (w as f32 / var3) as f64,
        (0 + var1) as f64,
    );
    g.add_vertex_with_uv(0.0, 0.0, 0.0, 0.0, (0 + var1) as f64);
    g.draw();
}

/// The class name the registry and the scripts use for a kind.
pub fn screen_name(kind: &ScreenKind) -> String {
    match kind {
        ScreenKind::MainMenu => "GuiMainMenu",
        ScreenKind::IngameMenu => "GuiIngameMenu",
        ScreenKind::GameOver => "GuiGameOver",
        ScreenKind::Inventory => "GuiInventory",
        ScreenKind::Chat => "GuiChat",
        ScreenKind::SleepMp => "GuiSleepMP",
        ScreenKind::ConflictWarning => "GuiConflictWarning",
        ScreenKind::ErrorScreen => "GuiErrorScreen",
        ScreenKind::Unused => "GuiUnused",
        ScreenKind::Other(s) => s,
    }
    .to_string()
}

/// The kind for a class name ([`screen_name`]'s inverse).
pub fn screen_kind(name: &str) -> ScreenKind {
    match name {
        "GuiMainMenu" => ScreenKind::MainMenu,
        "GuiIngameMenu" => ScreenKind::IngameMenu,
        "GuiGameOver" => ScreenKind::GameOver,
        "GuiInventory" => ScreenKind::Inventory,
        "GuiChat" => ScreenKind::Chat,
        "GuiSleepMP" => ScreenKind::SleepMp,
        "GuiConflictWarning" => ScreenKind::ConflictWarning,
        "GuiErrorScreen" => ScreenKind::ErrorScreen,
        "GuiUnused" => ScreenKind::Unused,
        other => ScreenKind::Other(other.to_string()),
    }
}

/// Makes a screen object.
pub type ScreenFactory = fn() -> Box<dyn Screen>;

/// The screens the binder can make from a kind alone (module docs): the
/// ones the shell opens by itself, and any a script names. WP-3.6b/c add
/// theirs with [`ScreenRegistry::register`]; a kind without a factory gets
/// a [`ShellScreen`].
#[derive(Resource, Default)]
pub struct ScreenRegistry {
    by_name: BTreeMap<String, ScreenFactory>,
}

impl ScreenRegistry {
    /// Registers `f` for the class `name` (`"GuiIngameMenu"`, …).
    pub fn register(&mut self, name: &str, f: ScreenFactory) {
        self.by_name.insert(name.to_string(), f);
    }

    /// Whether `name` has a factory.
    pub fn has(&self, name: &str) -> bool {
        self.by_name.contains_key(name)
    }

    /// A new object for `kind`.
    pub fn create(&self, kind: &ScreenKind) -> Box<dyn Screen> {
        match self.by_name.get(&screen_name(kind)) {
            Some(f) => f(),
            None => Box::new(ShellScreen::new(kind.clone())),
        }
    }

    /// The registered names, sorted.
    pub fn names(&self) -> Vec<String> {
        self.by_name.keys().cloned().collect()
    }
}

/// A screen the shell opened for which no WP has a class yet (the main
/// menu, the pause menu, the game-over screen, the inventory, until
/// WP-3.6b/c register theirs): `drawDefaultBackground` and the keys the
/// shell gave it before WP-3.6 (`GuiScreen.keyTyped`'s Escape;
/// `GuiContainer.keyTyped`'s Escape or inventory key for the inventory;
/// none for the screens whose `keyTyped` is empty).
#[derive(Clone, Debug)]
pub struct ShellScreen {
    base: ScreenBase,
    kind: ScreenKind,
}

impl ShellScreen {
    /// A placeholder for `kind`.
    pub fn new(kind: ScreenKind) -> Self {
        let slot = ScreenSlot::new(kind.clone());
        ShellScreen {
            base: ScreenBase {
                allow_user_input: slot.allow_user_input,
                ..ScreenBase::default()
            },
            kind,
        }
    }
}

impl Screen for ShellScreen {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        self.kind.clone()
    }
    fn does_gui_pause_game(&self) -> bool {
        ScreenSlot::new(self.kind.clone()).pauses_game
    }
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, _p: f32) {
        draw_default_background(&self.base, ctx, g);
        default_draw_screen(&mut self.base, ctx, g, mx, my);
    }
    // src: GuiMainMenu.java:41-42, GuiGameOver.java:16-17,
    // GuiContainer.java:155-160
    fn key_typed(&mut self, ctx: &mut ScreenCtx, _ch: char, key: i32) {
        match self.kind {
            ScreenKind::MainMenu | ScreenKind::GameOver => {}
            ScreenKind::Inventory => {
                if key == keys::KEY_ESCAPE
                    || key == ctx.game.game_settings.key(crate::settings::KEY_INVENTORY)
                {
                    // thePlayer.closeScreen(): displayGuiScreen(null)
                    // (the crafting inventory's half waits for WP-3.6b).
                    ctx.display_gui_screen(None);
                }
            }
            _ => default_key_typed(ctx, key),
        }
    }
}

/// The screen object bound to the slot, and the bookkeeping of the one
/// running a method (module docs).
#[derive(Resource, Default)]
pub struct Binder {
    /// The object bound to the slot (out while it runs a method).
    screen: Option<Box<dyn Screen>>,
    /// `(screen_serial, screen_resize_serial, width, height)` of the bound
    /// object (the [`Game`] counters it was last initialised at).
    bound: Option<(u64, u64, i32, i32)>,
    /// An object handed to `displayGuiScreen`, for the serial it gets.
    pending: Option<(u64, Box<dyn Screen>)>,
    /// The bound object that is running a method (taken out).
    running: Option<RunState>,
}

/// What happened to a running bound object meanwhile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RunState {
    serial: u64,
    /// It was replaced: `onGuiClosed` when it returns.
    closed: bool,
    /// The window was resized: `setWorldAndResolution` when it returns.
    reinit: bool,
}

impl Binder {
    /// The bound object's serial ([`Game::screen_serial`]).
    pub fn bound_serial(&self) -> Option<u64> {
        self.bound.map(|b| b.0)
    }

    /// The bound object (not while it runs a method).
    pub fn screen(&self) -> Option<&dyn Screen> {
        self.screen.as_deref()
    }
}

/// Makes the screen that takes the running object (`new X(this, …)`).
pub type WrapThis = Box<dyn FnOnce(Box<dyn Screen>) -> Box<dyn Screen> + Send + Sync>;

/// What a running screen asked to do with itself (module docs).
pub enum ThisRequest {
    /// `mc.displayGuiScreen(this)`.
    Display,
    /// `mc.displayGuiScreen(new X(this, …))`.
    Wrap(WrapThis),
}

/// After a method on a screen object: where the object is now.
pub enum After {
    /// Bound to the slot, with this serial (it may have been displayed
    /// again).
    Bound(u64),
    /// No longer the current screen (closed); Java's object lives on in
    /// the caller.
    Detached(Box<dyn Screen>),
    /// Handed to another screen (`new GuiYesNo(this, …)`).
    Gone,
}

/// What a screen reaches through `this.mc` (module docs).
pub struct ScreenCtx<'a> {
    /// `Minecraft`.
    pub game: &'a mut Game,
    /// `mc.fontRenderer`.
    pub font: &'a FontRenderer,
    /// `StringTranslate.getInstance()`.
    pub lang: &'a StringTranslate,
    binder: &'a mut Binder,
    registry: &'a ScreenRegistry,
    clipboard: Option<&'a mut dyn FnMut() -> Option<String>>,
    this_request: Option<ThisRequest>,
}

impl<'a> ScreenCtx<'a> {
    /// A context over the game and the GUI's resources.
    pub fn new(
        game: &'a mut Game,
        font: &'a FontRenderer,
        lang: &'a StringTranslate,
        binder: &'a mut Binder,
        registry: &'a ScreenRegistry,
        clipboard: Option<&'a mut dyn FnMut() -> Option<String>>,
    ) -> Self {
        ScreenCtx {
            game,
            font,
            lang,
            binder,
            registry,
            clipboard,
            this_request: None,
        }
    }

    /// `mc.sndManager.playSoundFX(name, volume, pitch)` (through the
    /// shell's sound messages, WP-3.8).
    pub fn play_sound_fx(&mut self, name: &str, volume: f32, pitch: f32) {
        self.game.outbox.sounds.push(ClientSound::PlaySoundFx {
            name: name.to_string(),
            volume,
            pitch,
        });
    }

    /// `GuiScreen.getClipboardString()` (`None` when there is none).
    // src: GuiScreen.java:39-50
    pub fn get_clipboard_string(&mut self) -> Option<String> {
        self.clipboard.as_mut().and_then(|f| f())
    }

    /// `Keyboard.enableRepeatEvents(on)`: held keys repeat into `keyTyped`
    /// while on (GuiChat, GuiCreateWorld, GuiRenameWorld and GuiEditSign
    /// turn it on in `initGui` and off in `onGuiClosed`; LWJGL's default
    /// is off). Sets [`crate::input::InputQueue::repeat_events`].
    // src: GuiChat.java:11, :15
    pub fn enable_repeat_events(&mut self, on: bool) {
        self.game.input.repeat_events = on;
    }

    /// A new object of the class `kind` from the registry (`new X()`).
    pub fn create_screen(&self, kind: &ScreenKind) -> Box<dyn Screen> {
        self.registry.create(kind)
    }

    /// `mc.displayGuiScreen(screen)`, at once (module docs).
    // src: Minecraft.java:410-444
    pub fn display_gui_screen(&mut self, screen: Option<Box<dyn Screen>>) {
        match screen {
            Some(obj) => {
                let slot = obj.slot();
                self.binder.pending = Some((self.game.screen_serial.wrapping_add(1), obj));
                self.game.display_gui_screen(Some(slot));
            }
            None => self.game.display_gui_screen(None),
        }
        self.sync();
    }

    /// `mc.displayGuiScreen(this)`: resolved by the running object's owner.
    pub fn display_this(&mut self) {
        self.this_request = Some(ThisRequest::Display);
    }

    /// `mc.displayGuiScreen(new X(this, …))`: `f` gets the running object.
    pub fn display_wrapping_this(
        &mut self,
        f: impl FnOnce(Box<dyn Screen>) -> Box<dyn Screen> + Send + Sync + 'static,
    ) {
        self.this_request = Some(ThisRequest::Wrap(Box::new(f)));
    }

    /// Takes a pending [`ThisRequest`] (for an owner of a sub-screen).
    pub fn take_this_request(&mut self) -> Option<ThisRequest> {
        self.this_request.take()
    }

    /// Carries out a [`ThisRequest`] on the object it was made by.
    pub fn resolve_this(&mut self, req: ThisRequest, obj: Box<dyn Screen>) {
        match req {
            ThisRequest::Display => self.display_gui_screen(Some(obj)),
            ThisRequest::Wrap(f) => self.display_gui_screen(Some(f(obj))),
        }
    }

    /// `mc.setIngameFocus()`.
    // src: Minecraft.java:750-760
    pub fn set_ingame_focus(&mut self) {
        self.game.set_ingame_focus();
        self.sync();
    }

    /// `mc.toggleFullscreen()`.
    pub fn toggle_fullscreen(&mut self) {
        self.game.toggle_fullscreen();
        self.sync();
    }

    /// Brings the bound object in line with the shell's slot (module docs).
    pub fn sync(&mut self) {
        let slot = self.game.current_screen.clone();
        let serial = self.game.screen_serial;
        let resized = self.game.screen_resize_serial;
        match slot {
            None => {
                if self.binder.bound.is_some() {
                    self.close_bound();
                    self.binder.bound = None;
                }
            }
            Some(s) => {
                if self.binder.bound_serial() != Some(serial) {
                    if self.binder.bound.is_some() {
                        self.close_bound();
                    }
                    let obj = match self.binder.pending.take() {
                        Some((p, obj)) if p == serial => obj,
                        _ => self.registry.create(&s.kind),
                    };
                    self.binder.bound = Some((serial, resized, s.width, s.height));
                    let after = self.run_bound(obj, serial, |o, ctx| {
                        set_world_and_resolution(o, ctx, s.width, s.height);
                    });
                    drop_detached(after);
                } else if let Some((_, r, w, h)) = self.binder.bound
                    && (r != resized || (w, h) != (s.width, s.height))
                {
                    // Every `resize` with a screen open is a
                    // setWorldAndResolution, whether or not the scaled size
                    // changed (Game::screen_resize_serial).
                    // src: Minecraft.java:900-918
                    self.binder.bound = Some((serial, resized, s.width, s.height));
                    match self.binder.running.as_mut() {
                        Some(r) if r.serial == serial => r.reinit = true,
                        _ => {
                            if let Some(obj) = self.binder.screen.take() {
                                let after = self.run_bound(obj, serial, |o, ctx| {
                                    set_world_and_resolution(o, ctx, s.width, s.height);
                                });
                                drop_detached(after);
                            }
                        }
                    }
                }
            }
        }
        self.binder.pending = None;
    }

    /// `currentScreen.onGuiClosed()` for the bound object being replaced.
    fn close_bound(&mut self) {
        let serial = self.binder.bound_serial();
        match self.binder.running.as_mut() {
            Some(r) if Some(r.serial) == serial => r.closed = true,
            _ => {
                if let Some(mut obj) = self.binder.screen.take() {
                    obj.on_gui_closed(self);
                }
            }
        }
    }

    /// Runs `f` on the bound object `obj` (serial `serial`, taken out of the
    /// binder) and settles what it did (module docs): returns where the
    /// object is now.
    pub fn run_bound(
        &mut self,
        obj: Box<dyn Screen>,
        serial: u64,
        f: impl FnOnce(&mut dyn Screen, &mut ScreenCtx),
    ) -> After {
        let mut f = Some(f);
        self.run_bound_dyn(obj, serial, &mut |o, c| {
            if let Some(f) = f.take() {
                f(o, c);
            }
        })
    }

    fn run_bound_dyn(
        &mut self,
        mut obj: Box<dyn Screen>,
        serial: u64,
        f: &mut dyn FnMut(&mut dyn Screen, &mut ScreenCtx),
    ) -> After {
        let prev = self.binder.running.replace(RunState {
            serial,
            closed: false,
            reinit: false,
        });
        let outer = self.this_request.take();
        f(obj.as_mut(), self);
        let mine = std::mem::replace(&mut self.this_request, outer);
        let st = std::mem::replace(&mut self.binder.running, prev).expect("running");
        let still_bound = !st.closed && self.binder.bound_serial() == Some(serial);
        if st.closed {
            obj.on_gui_closed(self);
        } else if still_bound && st.reinit {
            let (_, _, w, h) = self.binder.bound.expect("bound");
            let after = self.run_bound_dyn(obj, serial, &mut |o, ctx| {
                set_world_and_resolution(o, ctx, w, h);
            });
            return self.settle_this(mine, after);
        }
        let after = if still_bound {
            self.binder.screen = Some(obj);
            After::Bound(serial)
        } else {
            After::Detached(obj)
        };
        self.settle_this(mine, after)
    }

    /// Runs `f` on an object no longer bound (Java's old `this` receiving
    /// the rest of its `handleInput`).
    pub fn run_detached(
        &mut self,
        mut obj: Box<dyn Screen>,
        f: impl FnOnce(&mut dyn Screen, &mut ScreenCtx),
    ) -> After {
        let outer = self.this_request.take();
        f(obj.as_mut(), self);
        let mine = std::mem::replace(&mut self.this_request, outer);
        self.settle_this(mine, After::Detached(obj))
    }

    /// A [`ThisRequest`] left by an object's method: the object goes to the
    /// slot (closing it first if it is the current screen), itself or
    /// inside the screen the request makes (then it is [`After::Gone`]).
    fn settle_this(&mut self, req: Option<ThisRequest>, after: After) -> After {
        let Some(req) = req else {
            return after;
        };
        let wrapped = matches!(req, ThisRequest::Wrap(_));
        match after {
            After::Bound(serial) => {
                let obj = if self.binder.bound_serial() == Some(serial) {
                    self.binder.screen.take()
                } else {
                    None
                };
                let Some(mut obj) = obj else {
                    return After::Bound(serial);
                };
                // displayGuiScreen: currentScreen (this object) .onGuiClosed()
                // first.
                obj.on_gui_closed(self);
                self.binder.bound = None;
                self.resolve_this(req, obj);
            }
            After::Detached(obj) => self.resolve_this(req, obj),
            After::Gone => return After::Gone,
        }
        match self.binder.bound_serial() {
            Some(s) if !wrapped => After::Bound(s),
            _ => After::Gone,
        }
    }

    /// The bound object's serial.
    pub fn bound_serial(&self) -> Option<u64> {
        self.binder.bound_serial()
    }

    /// Puts back an object [`Self::take_bound`] took, untouched.
    pub fn restore_bound(&mut self, obj: Box<dyn Screen>) {
        self.binder.screen = Some(obj);
    }

    /// Takes the bound object out to run a method on it ([`Self::run_bound`]).
    pub fn take_bound(&mut self) -> Option<(Box<dyn Screen>, u64)> {
        let serial = self.binder.bound_serial()?;
        self.binder.screen.take().map(|o| (o, serial))
    }
}

/// An object that left the slot during its own setup is dropped.
fn drop_detached(_after: After) {}

/// `setWorldAndResolution(mc, width, height)`: the size, an empty control
/// list, `initGui()`. (`guiParticles`, `mc` and `fontRenderer` are the
/// context.)
// src: GuiScreen.java:77-85
pub fn set_world_and_resolution(s: &mut dyn Screen, ctx: &mut ScreenCtx, w: i32, h: i32) {
    let b = s.base_mut();
    b.width = w;
    b.height = h;
    b.control_list.clear();
    s.init_gui(ctx);
}
