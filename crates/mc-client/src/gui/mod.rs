//! The GUI (WP-3.6a): `Gui`, `GuiScreen` and the widgets, `FontRenderer`,
//! `ScaledResolution`, and the plumbing that binds screen objects to the
//! shell's `currentScreen` slot, feeds them `handleInput`/`updateScreen`
//! inside `runTick` and draws them every frame.
//!
//! | Module | Vanilla |
//! |---|---|
//! | [`batch`] | the GL state and `Tessellator` the GUI draws with (as data) |
//! | [`draw`] | `Gui` |
//! | [`font`] | `FontRenderer`, `ChatAllowedCharacters` |
//! | [`scaled`] | `ScaledResolution` (the type is [`crate::app`]'s), the mouse scaling |
//! | [`screen`] | `GuiScreen`, `Minecraft.displayGuiScreen`'s object side |
//! | [`widgets`] | `GuiButton`, `GuiSmallButton`, `GuiSlider`, `GuiTextField`, `GuiSlot`, `GuiYesNo` |
//! | [`error_screen`], [`conflict_warning`], [`unused`] | `GuiErrorScreen`, `GuiConflictWarning`, `GuiUnused` |
//! | [`render`] | the Bevy side: two orthographic cameras, [`render::GuiMaterial`] |
//! | [`debug_screens`] | dev screens for the `debug_gui` script action |
//! | [`ingame`] (WP-3.6b) | `GuiIngame` (the HUD, the chat lines), `Minecraft.displayDebugInfo`'s graph |
//! | [`chat`] (WP-3.6b) | `ChatLine`, `GuiChat` |
//! | [`achievement`] (WP-3.6b) | `GuiAchievement` |
//! | [`render_item`] (WP-3.6b) | `RenderItem`'s GUI half, the GUI's item lighting |
//! | [`container`] (WP-3.6b) | `GuiContainer`, `GuiInventory`, `GuiChest`, `GuiFurnace`, `GuiCrafting`, `GuiDispenser`, `GuiEditSign`, `GuiSleepMP`, `GuiGameOver`; `EntityPlayerSP.displayGUI*` |
//!
//! # Frame and tick
//!
//! - In `runTick`, [`RunTickHook::ScreenInput`] runs [`screen_input`]:
//!   `currentScreen.handleInput()` (every buffered mouse event, then every
//!   key event, each to the screen object that was current when it began,
//!   as Java's `this` — GuiScreen.java:90-99), `guiParticles` (never fed
//!   in Beta: nothing to do) and `updateScreen()`
//!   (Minecraft.java:987-993). The plugin owns the hook
//!   ([`ScreenInputOwner`]).
//! - Every frame after the shell's ([`GameFrameSet`]) and the textures'
//!   ([`TexturesSet`]) systems: [`GuiSet::Begin`] clears the batches,
//!   [`GuiSet::Overlay`] is for `ingameGUI.renderGameOverlay` (WP-3.6b:
//!   draw into [`GuiLayer::Overlay`]), [`GuiSet::Screen`] draws
//!   `currentScreen.drawScreen(mouseX, mouseY, partialTicks)`
//!   (EntityRenderer.java:420-427) with the mouse scaled as
//!   EntityRenderer.java:359-363 does, [`GuiSet::Flush`] turns the batches
//!   into meshes ([`render::flush_gui`]).
//!
//! # For WP-3.6b/c
//!
//! - A screen is a type implementing [`screen::Screen`] with a
//!   [`screen::ScreenBase`]; Java's `super.drawScreen(..)` and friends are
//!   [`screen::default_draw_screen`], [`screen::default_key_typed`],
//!   [`screen::default_mouse_clicked`], [`screen::default_mouse_moved_or_up`].
//!   Draw with [`draw`] (`drawRect`, `drawGradientRect`,
//!   `drawTexturedModalRect`, `drawString`, `drawCenteredString`) and raw GL
//!   through [`batch::GuiGl`]; `this.mc` is [`screen::ScreenCtx`]
//!   (`ctx.game`, `ctx.font`, `ctx.lang`, `ctx.display_gui_screen`,
//!   `ctx.display_this`, `ctx.display_wrapping_this` for
//!   `new GuiYesNo(this, …)`, `ctx.play_sound_fx`).
//! - Register a screen the shell opens by kind (the main menu, the pause
//!   menu, the game-over screen, the inventory) with
//!   [`screen::ScreenRegistry::register`] under its class name; until then
//!   such a kind gets a [`screen::ShellScreen`] (the background and the
//!   shell's old default keys).
//! - The in-game overlay draws in a system in [`GuiSet::Overlay`] after
//!   `gl.set_layer(GuiLayer::Overlay)` (the default at the frame start).

pub mod achievement;
pub mod batch;
pub mod chat;
pub mod conflict_warning;
pub mod container;
pub mod debug_screens;
pub mod draw;
pub mod error_screen;
pub mod font;
pub mod ingame;
pub mod render;
pub mod render_item;
pub mod scaled;
pub mod screen;
pub mod unused;
pub mod widgets;

use bevy::prelude::*;

use self::batch::{GuiGl, GuiLayer};
use self::font::{ChatAllowedCharacters, FONT_PNG, FONT_TXT, FontRenderer};
use self::render::{GuiMaterial, GuiMeshPool, GuiTextures};
use self::scaled::{ScaledResolution, mouse_to_gui};
use self::screen::{After, Binder, Screen, ScreenCtx, ScreenRegistry, screen_kind};
use crate::GameFrameSet;
use crate::assets::{JarAssets, Lang};
use crate::game::{Game, RunTickHook, ScreenInputOwner};
use crate::input::InputEvent;
use crate::textures::{ArgbImage, Textures, TexturesSet};

/// The GUI's frame (module docs), in order.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GuiSet {
    /// The batches cleared, the GL state reset.
    Begin,
    /// `ingameGUI.renderGameOverlay` (WP-3.6b).
    Overlay,
    /// `currentScreen.drawScreen`.
    Screen,
    /// Batches into meshes.
    Flush,
}

/// The GUI plugin (module docs). Added by [`crate::ClientPlugin`].
pub struct GuiPlugin;

impl Plugin for GuiPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<crate::gl_material::GlMaterialPlugin>() {
            app.add_plugins(crate::gl_material::GlMaterialPlugin);
        }
        let font = {
            let w = app.world();
            let anaglyph = w
                .get_non_send::<Game>()
                .is_some_and(|g| g.game_settings.anaglyph);
            let jar = w.get_resource::<JarAssets>();
            build_font(jar, anaglyph)
        };
        let mut registry = ScreenRegistry::default();
        register_screens(&mut registry);
        app.add_plugins(MaterialPlugin::<GuiMaterial>::default())
            .insert_resource(font)
            .insert_resource(registry)
            .insert_resource(ScreenInputOwner(true))
            .init_resource::<Binder>()
            .init_resource::<GuiGl>()
            .init_resource::<GuiMeshPool>()
            .init_resource::<GuiTextures>()
            .configure_sets(
                Update,
                (
                    GuiSet::Begin,
                    GuiSet::Overlay,
                    GuiSet::Screen,
                    GuiSet::Flush,
                )
                    .chain()
                    .after(GameFrameSet)
                    .after(TexturesSet),
            )
            .add_systems(Startup, (render::spawn_gui_cameras, upload_font_texture))
            .add_systems(RunTickHook::ScreenInput, screen_input)
            .add_systems(Update, begin_gui_frame.in_set(GuiSet::Begin))
            .add_systems(Update, draw_current_screen.in_set(GuiSet::Screen))
            .add_systems(Update, render::flush_gui.in_set(GuiSet::Flush));
        // WP-3.6b: the in-game overlay, the chat, the achievement window,
        // the screens the player opens.
        add_ingame_systems(app);
    }
}

/// WP-3.6b's resources and systems ([`ingame`], [`achievement`],
/// [`container`]): the messages are collected in every tick hook (before
/// the screen's input) and after the frame; `updateTick` in its hook; the
/// overlay in [`GuiSet::Overlay`]; the debug graph and the achievement
/// window after the screen.
fn add_ingame_systems(app: &mut App) {
    use self::achievement::{GuiAchievement, apply_achievement_requests, draw_achievement_window};
    use self::container::{PendingPlayerGuis, open_pending_player_guis};
    use self::ingame::{
        GuiIngame, GuiMessageCursors, collect_gui_messages, draw_debug_graph, draw_game_overlay,
        ingame_update_tick,
    };
    app.init_resource::<GuiIngame>()
        .init_resource::<GuiAchievement>()
        .init_resource::<GuiMessageCursors>()
        .init_resource::<PendingPlayerGuis>();
    for hook in RunTickHook::ALL {
        match hook {
            RunTickHook::IngameGuiUpdateTick => {
                app.add_systems(
                    hook,
                    (
                        collect_gui_messages,
                        open_pending_player_guis,
                        ingame_update_tick,
                    )
                        .chain(),
                );
            }
            RunTickHook::ScreenInput => {
                app.add_systems(
                    hook,
                    (collect_gui_messages, open_pending_player_guis)
                        .chain()
                        .before(screen_input),
                );
            }
            _ => {
                app.add_systems(
                    hook,
                    (collect_gui_messages, open_pending_player_guis).chain(),
                );
            }
        }
    }
    app.add_systems(
        Update,
        (
            collect_gui_messages,
            open_pending_player_guis,
            apply_achievement_requests,
        )
            .chain()
            .after(GameFrameSet)
            .before(GuiSet::Begin),
    )
    .add_systems(Update, draw_game_overlay.in_set(GuiSet::Overlay))
    .add_systems(
        Update,
        (draw_debug_graph, draw_achievement_window)
            .chain()
            .in_set(GuiSet::Screen)
            .after(draw_current_screen),
    );
}

/// Registers this WP's screens (and the dev screens) by class name.
pub fn register_screens(registry: &mut ScreenRegistry) {
    registry.register("GuiConflictWarning", || {
        Box::new(conflict_warning::GuiConflictWarning::default())
    });
    registry.register("GuiErrorScreen", || {
        Box::new(error_screen::GuiErrorScreen::default())
    });
    registry.register("GuiUnused", || Box::new(unused::GuiUnused::default()));
    debug_screens::register(registry);
    // WP-3.6b
    container::register(registry);
}

/// `new FontRenderer(gameSettings, "/font/default.png", renderEngine)`'s
/// measurements, from the jar (a missing font logs and leaves every width
/// at 1 and no allowed character).
// src: Minecraft.java:241
pub fn build_font(jar: Option<&JarAssets>, anaglyph: bool) -> FontRenderer {
    let allowed = jar
        .and_then(|j| j.get(FONT_TXT))
        .map(ChatAllowedCharacters::parse)
        .unwrap_or_default();
    let sheet = match jar.map(|j| j.rgba(FONT_PNG)) {
        Some(Ok((w, h, px))) => ArgbImage::from_rgba(w, h, &px),
        Some(Err(e)) => {
            error!("{e}");
            ArgbImage::new(128, 128)
        }
        None => ArgbImage::new(128, 128),
    };
    FontRenderer::new(&sheet, anaglyph, allowed)
}

/// `fontTextureName = renderEngine.allocateAndSetupTexture(image)`.
// src: FontRenderer.java:62
fn upload_font_texture(
    mut font: ResMut<FontRenderer>,
    jar: Res<JarAssets>,
    textures: Option<ResMut<Textures>>,
    images: Option<ResMut<Assets<Image>>>,
) {
    let (Some(mut textures), Some(mut images)) = (textures, images) else {
        return;
    };
    match jar.rgba(FONT_PNG) {
        Ok((w, h, px)) => {
            let img = ArgbImage::from_rgba(w, h, &px);
            font.font_texture_name = Some(textures.allocate_and_setup_texture(img, &mut images));
        }
        Err(e) => error!("{e}"),
    }
}

/// Runs `f` with a [`ScreenCtx`] over the world's resources (for script
/// actions and commands).
pub fn with_ctx<R>(world: &mut World, f: impl FnOnce(&mut ScreenCtx) -> R) -> Option<R> {
    world.resource_scope(|world, mut binder: Mut<Binder>| {
        world.resource_scope(|world, registry: Mut<ScreenRegistry>| {
            world.resource_scope(|world, font: Mut<FontRenderer>| {
                world.resource_scope(|world, lang: Mut<Lang>| {
                    let mut game = world.get_non_send_mut::<Game>()?;
                    let mut ctx =
                        ScreenCtx::new(&mut game, &font, &lang.0, &mut binder, &registry, None);
                    Some(f(&mut ctx))
                })
            })
        })
    })
}

/// Opens the screen registered as `name` (or a placeholder of that class),
/// as `mc.displayGuiScreen(new X())` would (the `open_screen` and
/// `debug_gui` script actions).
pub fn open_screen_by_name(world: &mut World, name: &str) {
    let r = with_ctx(world, |ctx| {
        let obj = ctx.create_screen(&screen_kind(name));
        ctx.display_gui_screen(Some(obj));
    });
    if r.is_none() {
        error!("open_screen {name}: the GUI is not running");
    }
}

/// [`GuiSet::Begin`].
fn begin_gui_frame(mut g: ResMut<GuiGl>) {
    g.begin_frame();
}

/// Reads the system clipboard (`GuiScreen.getClipboardString`).
fn clipboard_reader(
    cb: Option<&mut bevy::clipboard::Clipboard>,
) -> impl FnMut() -> Option<String> + '_ {
    let mut cb = cb;
    move || {
        let c = cb.as_mut()?;
        match c.fetch_text() {
            bevy::clipboard::ClipboardRead::Ready(Ok(s)) => Some(s),
            _ => None,
        }
    }
}

/// The screen hook of `runTick` (module docs).
// src: Minecraft.java:987-993
pub fn screen_input(
    mut game: NonSendMut<Game>,
    mut binder: ResMut<Binder>,
    font: Res<FontRenderer>,
    lang: Res<Lang>,
    registry: Res<ScreenRegistry>,
    mut clipboard: Option<ResMut<bevy::clipboard::Clipboard>>,
) {
    let mut read = clipboard_reader(clipboard.as_deref_mut());
    let mut ctx = ScreenCtx::new(
        &mut game,
        &font,
        &lang.0,
        &mut binder,
        &registry,
        Some(&mut read),
    );
    handle_input(&mut ctx);
    // currentScreen.guiParticles.func_25088_a(): no GuiParticle is ever
    // added in Beta (GuiParticle.java), so there is nothing to update.
    if ctx.game.current_screen.is_some() {
        ctx.sync();
        if let Some((o, s)) = ctx.take_bound() {
            let _ = ctx.run_bound(o, s, |s, ctx| s.update_screen(ctx));
        }
    }
}

/// The screen object events go to (Java's `this` in `handleInput`).
enum Held {
    /// The bound object of this serial.
    Bound(u64),
    /// Replaced, still receiving the rest of the events.
    Detached(Box<dyn Screen>),
    /// Handed to another screen (`new GuiYesNo(this, …)`).
    Gone,
}

/// Runs `f` on the held object and keeps track of it.
fn dispatch(
    ctx: &mut ScreenCtx,
    held: Held,
    f: impl FnOnce(&mut dyn Screen, &mut ScreenCtx),
) -> Held {
    let after = match held {
        Held::Bound(s) => match ctx.take_bound() {
            Some((o, s2)) if s2 == s => ctx.run_bound(o, s, f),
            Some((o, _)) => {
                // Not ours: put it back untouched.
                ctx.restore_bound(o);
                return Held::Gone;
            }
            None => return Held::Gone,
        },
        Held::Detached(o) => ctx.run_detached(o, f),
        Held::Gone => return Held::Gone,
    };
    match after {
        After::Bound(s) => Held::Bound(s),
        After::Detached(o) => Held::Detached(o),
        After::Gone => Held::Gone,
    }
}

/// `currentScreen.handleInput()`: every mouse event, then every key event
/// (module docs).
// src: GuiScreen.java:90-126
pub fn handle_input(ctx: &mut ScreenCtx) {
    ctx.sync();
    let Some(serial) = ctx.bound_serial() else {
        ctx.game.input.clear_events();
        return;
    };
    let mut held = Held::Bound(serial);
    while let Some(e) = ctx.game.input.mouse.pop_front() {
        held = dispatch(ctx, held, |s, ctx| handle_mouse_input(s, ctx, &e));
    }
    while let Some(e) = ctx.game.input.keyboard.pop_front() {
        held = dispatch(ctx, held, |s, ctx| handle_keyboard_input(s, ctx, &e));
    }
    drop(held);
}

/// `handleMouseInput()`: a press is `mouseClicked`, anything else
/// `mouseMovedOrUp`, at `x * width / displayWidth`,
/// `height - y * height / displayHeight - 1`.
// src: GuiScreen.java:101-114
pub fn handle_mouse_input(s: &mut dyn Screen, ctx: &mut ScreenCtx, e: &InputEvent) {
    let (button, down, x, y) = match *e {
        InputEvent::MouseButton { button, down, x, y } => (button, down, x, y),
        InputEvent::MouseWheel { x, y, .. } => (-1, false, x, y),
        InputEvent::Key { .. } => return,
    };
    let (w, h) = (s.base().width, s.base().height);
    let (var1, var2) = mouse_to_gui(x, y, w, h, ctx.game.display_width, ctx.game.display_height);
    if down {
        s.mouse_clicked(ctx, var1, var2, button);
    } else {
        s.mouse_moved_or_up(ctx, var1, var2, button);
    }
}

/// LWJGL's `Keyboard.KEY_LCONTROL` / `KEY_RCONTROL`.
const KEY_LCONTROL: i32 = 29;
const KEY_RCONTROL: i32 = 157;

/// `handleKeyboardInput()`: on a press, F11 toggles the full screen, any
/// other key is `keyTyped(char, key)`.
// src: GuiScreen.java:116-126
pub fn handle_keyboard_input(s: &mut dyn Screen, ctx: &mut ScreenCtx, e: &InputEvent) {
    let InputEvent::Key { key, down, ch } = *e else {
        return;
    };
    if !down {
        return;
    }
    if key == crate::keys::KEY_F11 {
        ctx.toggle_fullscreen();
        return;
    }
    // LWJGL reports Ctrl+letter as the control character (Ctrl+V is 22,
    // GuiTextField.java:44); Bevy reports the letter.
    let ctrl = ctx.game.input.is_key_down(KEY_LCONTROL) || ctx.game.input.is_key_down(KEY_RCONTROL);
    let ch = if ctrl && ch.is_ascii_alphabetic() {
        char::from(ch.to_ascii_lowercase() as u8 - b'a' + 1)
    } else {
        ch
    };
    s.key_typed(ctx, ch, key);
}

/// [`GuiSet::Screen`]: `currentScreen.drawScreen(mouseX, mouseY,
/// partialTicks)` after the depth clear (the screen camera's).
// src: EntityRenderer.java:357-363, :420-427
#[allow(clippy::too_many_arguments)]
pub fn draw_current_screen(
    mut game: NonSendMut<Game>,
    mut binder: ResMut<Binder>,
    font: Res<FontRenderer>,
    lang: Res<Lang>,
    registry: Res<ScreenRegistry>,
    mut clipboard: Option<ResMut<bevy::clipboard::Clipboard>>,
    mut g: ResMut<GuiGl>,
) {
    let mut read = clipboard_reader(clipboard.as_deref_mut());
    let mut ctx = ScreenCtx::new(
        &mut game,
        &font,
        &lang.0,
        &mut binder,
        &registry,
        Some(&mut read),
    );
    ctx.sync();
    if ctx.game.pending.is_some() || ctx.game.skip_render_world || ctx.game.current_screen.is_none()
    {
        return;
    }
    let (dw, dh) = (ctx.game.display_width, ctx.game.display_height);
    let r = ScaledResolution::new(ctx.game.game_settings.gui_scale, dw, dh);
    let (mx, my) = mouse_to_gui(
        ctx.game.input.mouse_x,
        ctx.game.input.mouse_y,
        r.scaled_width,
        r.scaled_height,
        dw,
        dh,
    );
    let partial = ctx.game.timer.render_partial_ticks;
    g.set_layer(GuiLayer::Screen);
    if let Some((o, s)) = ctx.take_bound() {
        let _ = ctx.run_bound(o, s, |s, ctx| s.draw_screen(ctx, &mut g, mx, my, partial));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::app::ScreenKind;
    use crate::events::ClientSound;
    use crate::lang::StringTranslate;
    use crate::paths::ClientPaths;
    use crate::session::Session;
    use crate::storage::tests::TempDir;
    use mc_sim::stats::AchievementMap;
    use screen::{ScreenBase, default_key_typed};

    struct Env {
        _dir: TempDir,
        game: Game,
        binder: Binder,
        registry: ScreenRegistry,
        font: FontRenderer,
        lang: StringTranslate,
    }

    fn env() -> Env {
        let dir = TempDir::new("gui");
        let paths = ClientPaths {
            data_dir: dir.0.clone(),
            jar: "/nonexistent.jar".into(),
            resources: "/nonexistent".into(),
        };
        let game = Game::new(paths, AchievementMap::default(), Session::new("T", "-")).unwrap();
        let mut registry = ScreenRegistry::default();
        register_screens(&mut registry);
        Env {
            _dir: dir,
            game,
            binder: Binder::default(),
            registry,
            font: FontRenderer::new(
                &ArgbImage::new(128, 128),
                false,
                ChatAllowedCharacters::from_chars("abc".chars().collect()),
            ),
            lang: StringTranslate::default(),
        }
    }

    impl Env {
        fn ctx(&mut self) -> ScreenCtx<'_> {
            ScreenCtx::new(
                &mut self.game,
                &self.font,
                &self.lang,
                &mut self.binder,
                &self.registry,
                None,
            )
        }

        fn kind(&self) -> Option<ScreenKind> {
            self.binder.screen().map(|s| s.kind())
        }

        /// A left click at scaled `(x, y)` on an 854×480 window.
        fn click(&mut self, x: i32, y: i32) {
            let (mx, my) = scaled::gui_to_mouse(x, y, 427, 240, 854, 480);
            self.game.input.mouse_x = mx;
            self.game.input.mouse_y = my;
            self.game.input.push_button(0, true);
            self.game.input.push_button(0, false);
        }
    }

    #[test]
    fn displaying_binds_and_inits_the_object() {
        let mut e = env();
        let mut ctx = e.ctx();
        ctx.display_gui_screen(Some(Box::new(debug_screens::DebugWidgets::default())));
        drop(ctx);
        assert_eq!(e.kind(), Some(ScreenKind::Other("DebugWidgets".into())));
        let s = e.binder.screen().unwrap();
        assert_eq!((s.base().width, s.base().height), (427, 240));
        assert_eq!(s.base().control_list.len(), 7);
        assert_eq!(
            e.game.current_screen.as_ref().map(|s| s.kind.clone()),
            Some(ScreenKind::Other("DebugWidgets".into()))
        );
        // A resize runs setWorldAndResolution again.
        e.game.resize(1920, 1080);
        e.ctx().sync();
        let s = e.binder.screen().unwrap();
        assert_eq!((s.base().width, s.base().height), (480, 270));
        assert_eq!(s.base().control_list.len(), 7);
    }

    #[test]
    fn escape_closes_and_the_shell_substitutes_the_main_menu() {
        let mut e = env();
        e.ctx()
            .display_gui_screen(Some(Box::new(debug_screens::DebugFont::default())));
        e.game.input.push_key(crate::keys::KEY_ESCAPE, true, '\0');
        handle_input(&mut e.ctx());
        // No world: displayGuiScreen(null) opens GuiMainMenu (a placeholder).
        assert_eq!(e.kind(), Some(ScreenKind::MainMenu));
        // Its keyTyped is empty: Escape does nothing.
        e.game.input.push_key(crate::keys::KEY_ESCAPE, true, '\0');
        handle_input(&mut e.ctx());
        assert_eq!(e.kind(), Some(ScreenKind::MainMenu));
        // A screen the shell opens by itself gets an object at the next sync.
        e.game.display_gui_screen(Some(crate::app::GuiScreen::new(
            ScreenKind::ConflictWarning,
        )));
        e.ctx().sync();
        assert_eq!(e.kind(), Some(ScreenKind::ConflictWarning));
        assert_eq!(e.binder.screen().unwrap().base().control_list.len(), 1);
    }

    #[test]
    fn yes_no_answers_its_parent_which_comes_back() {
        let mut e = env();
        let obj = e.registry.create(&ScreenKind::Other("DebugYesNo".into()));
        e.ctx().display_gui_screen(Some(obj));
        assert_eq!(e.kind(), Some(ScreenKind::Other("GuiYesNo".into())));
        // The "Yes" button: (427/2 - 155, 240/6 + 96), 150 × 20.
        e.click(213 - 155 + 5, 40 + 96 + 5);
        handle_input(&mut e.ctx());
        assert_eq!(e.kind(), Some(ScreenKind::Other("DebugWidgets".into())));
        let clicks = e
            .game
            .outbox
            .sounds
            .iter()
            .filter(
                |s| matches!(s, ClientSound::PlaySoundFx { name, .. } if name == "random.click"),
            )
            .count();
        assert_eq!(clicks, 1);
        // And the widget screen opens a new GuiYesNo around itself.
        e.click(213 + 5 + 5, 60 + 96 + 5);
        handle_input(&mut e.ctx());
        assert_eq!(e.kind(), Some(ScreenKind::Other("GuiYesNo".into())));
    }

    /// Counts its key events; the first closes it.
    struct Counter {
        base: ScreenBase,
        keys: Arc<AtomicUsize>,
        closed: Arc<AtomicUsize>,
    }

    impl Screen for Counter {
        fn base(&self) -> &ScreenBase {
            &self.base
        }
        fn base_mut(&mut self) -> &mut ScreenBase {
            &mut self.base
        }
        fn kind(&self) -> ScreenKind {
            ScreenKind::Other("Counter".into())
        }
        fn key_typed(&mut self, ctx: &mut ScreenCtx, _ch: char, key: i32) {
            self.keys.fetch_add(1, Ordering::SeqCst);
            default_key_typed(ctx, key);
        }
        fn on_gui_closed(&mut self, _ctx: &mut ScreenCtx) {
            self.closed.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn the_rest_of_handle_input_goes_to_the_closed_screen() {
        let mut e = env();
        let keys = Arc::new(AtomicUsize::new(0));
        let closed = Arc::new(AtomicUsize::new(0));
        e.ctx().display_gui_screen(Some(Box::new(Counter {
            base: ScreenBase::default(),
            keys: keys.clone(),
            closed: closed.clone(),
        })));
        e.game.input.push_key(crate::keys::KEY_ESCAPE, true, '\0');
        e.game.input.push_key(30, true, 'a');
        e.game.input.push_key(31, true, 's');
        handle_input(&mut e.ctx());
        // Java's handleInput is a method of the old screen: all three keys
        // reach it, though the first closed it (once).
        assert_eq!(keys.load(Ordering::SeqCst), 3);
        assert_eq!(closed.load(Ordering::SeqCst), 1);
        assert_eq!(e.kind(), Some(ScreenKind::MainMenu));
        assert!(e.game.input.keyboard.is_empty());
    }

    /// Counts its `initGui` calls.
    struct Inits {
        base: ScreenBase,
        inits: Arc<AtomicUsize>,
    }

    impl Screen for Inits {
        fn base(&self) -> &ScreenBase {
            &self.base
        }
        fn base_mut(&mut self) -> &mut ScreenBase {
            &mut self.base
        }
        fn kind(&self) -> ScreenKind {
            ScreenKind::Other("Inits".into())
        }
        fn init_gui(&mut self, _ctx: &mut ScreenCtx) {
            self.inits.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn every_resize_reinits_even_at_the_same_scaled_size() {
        // src: Minecraft.java:900-918: resize always calls
        // setWorldAndResolution with a screen open.
        let mut e = env();
        let inits = Arc::new(AtomicUsize::new(0));
        e.ctx().display_gui_screen(Some(Box::new(Inits {
            base: ScreenBase::default(),
            inits: inits.clone(),
        })));
        assert_eq!(inits.load(Ordering::SeqCst), 1);
        // 854 -> 853 wide: scale factor 2 both times, scaled width
        // ceil(426.5) = 427 = 854 / 2 (ScaledResolution.java).
        e.game.resize(853, 480);
        e.ctx().sync();
        assert_eq!(inits.load(Ordering::SeqCst), 2);
        let s = e.binder.screen().unwrap();
        assert_eq!((s.base().width, s.base().height), (427, 240));
        // No resize: no init.
        e.ctx().sync();
        assert_eq!(inits.load(Ordering::SeqCst), 2);
        // toggleFullscreen resizes with a screen open, same size included.
        e.ctx().toggle_fullscreen();
        assert_eq!(inits.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn enable_repeat_events_sets_the_input_flag() {
        let mut e = env();
        assert!(!e.game.input.repeat_events);
        e.ctx().enable_repeat_events(true);
        assert!(e.game.input.repeat_events);
        e.ctx().enable_repeat_events(false);
        assert!(!e.game.input.repeat_events);
    }

    #[test]
    fn font_from_a_missing_jar_is_empty() {
        let f = build_font(None, false);
        assert_eq!(f.get_string_width("abc"), 0);
        assert_eq!(f.char_width[65], 1);
    }
}
