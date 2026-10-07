//! `GameSettings` (GameSettings.java), `KeyBinding` (KeyBinding.java),
//! `EnumOptions` (EnumOptions.java) and `EnumOptionsMappingHelper`
//! (EnumOptionsMappingHelper.java): the options and `options.txt`.
//!
//! `options.txt` lives in the data directory and goes through the native
//! [`Storage`](mc_sim::save::Storage) ([`load_options`], [`save_options`]);
//! the text conversion itself is [`GameSettings::load_options_text`] /
//! [`GameSettings::save_options_text`], which keep Java's formatting
//! (`Float.toString`, `Boolean.toString`) and its tolerance (a bad line is
//! skipped, the rest still loads).
//!
//! Side effects of `setOptionValue` / `setOptionFloatValue` on other
//! subsystems (`sndManager.onSoundOptionsChanged`,
//! `renderGlobal.loadRenderers`, `renderEngine.refreshTextures`) are returned
//! as a [`SettingsEffect`] for the caller to forward ([`crate::events`]);
//! `saveOptions` after a change is the caller's too (it needs the storage).

use mc_sim::save::Storage;

use crate::jfmt;
use crate::lang::StringTranslate;

/// `KeyBinding`.
// src: KeyBinding.java:3-11
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyBinding {
    /// `keyDescription`: the translation key (`key.forward`, …), also the
    /// `options.txt` name after `key_`.
    pub key_description: &'static str,
    /// `keyCode`: an LWJGL key code ([`crate::keys`]).
    pub key_code: i32,
}

/// `EnumOptions`, in declaration (ordinal) order.
// src: EnumOptions.java:3-16
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EnumOptions {
    /// `MUSIC`.
    Music,
    /// `SOUND`.
    Sound,
    /// `INVERT_MOUSE`.
    InvertMouse,
    /// `SENSITIVITY`.
    Sensitivity,
    /// `RENDER_DISTANCE`.
    RenderDistance,
    /// `VIEW_BOBBING`.
    ViewBobbing,
    /// `ANAGLYPH`.
    Anaglyph,
    /// `ADVANCED_OPENGL`.
    AdvancedOpengl,
    /// `FRAMERATE_LIMIT`.
    FramerateLimit,
    /// `DIFFICULTY`.
    Difficulty,
    /// `GRAPHICS`.
    Graphics,
    /// `AMBIENT_OCCLUSION`.
    AmbientOcclusion,
    /// `GUI_SCALE`.
    GuiScale,
}

impl EnumOptions {
    /// `values()`.
    pub const VALUES: [EnumOptions; 13] = [
        EnumOptions::Music,
        EnumOptions::Sound,
        EnumOptions::InvertMouse,
        EnumOptions::Sensitivity,
        EnumOptions::RenderDistance,
        EnumOptions::ViewBobbing,
        EnumOptions::Anaglyph,
        EnumOptions::AdvancedOpengl,
        EnumOptions::FramerateLimit,
        EnumOptions::Difficulty,
        EnumOptions::Graphics,
        EnumOptions::AmbientOcclusion,
        EnumOptions::GuiScale,
    ];

    /// `(enumString, enumFloat, enumBoolean)`.
    // src: EnumOptions.java:4-16
    const fn data(self) -> (&'static str, bool, bool) {
        match self {
            EnumOptions::Music => ("options.music", true, false),
            EnumOptions::Sound => ("options.sound", true, false),
            EnumOptions::InvertMouse => ("options.invertMouse", false, true),
            EnumOptions::Sensitivity => ("options.sensitivity", true, false),
            EnumOptions::RenderDistance => ("options.renderDistance", false, false),
            EnumOptions::ViewBobbing => ("options.viewBobbing", false, true),
            EnumOptions::Anaglyph => ("options.anaglyph", false, true),
            EnumOptions::AdvancedOpengl => ("options.advancedOpengl", false, true),
            EnumOptions::FramerateLimit => ("options.framerateLimit", false, false),
            EnumOptions::Difficulty => ("options.difficulty", false, false),
            EnumOptions::Graphics => ("options.graphics", false, false),
            EnumOptions::AmbientOcclusion => ("options.ao", false, true),
            EnumOptions::GuiScale => ("options.guiScale", false, false),
        }
    }

    /// `getEnumOptions(ordinal)`.
    // src: EnumOptions.java:22-34
    pub fn get_enum_options(var0: i32) -> Option<EnumOptions> {
        EnumOptions::VALUES
            .into_iter()
            .find(|o| o.return_enum_ordinal() == var0)
    }

    /// `getEnumFloat()`: a slider.
    pub const fn get_enum_float(self) -> bool {
        self.data().1
    }

    /// `getEnumBoolean()`: an on/off button.
    pub const fn get_enum_boolean(self) -> bool {
        self.data().2
    }

    /// `returnEnumOrdinal()`.
    pub fn return_enum_ordinal(self) -> i32 {
        EnumOptions::VALUES
            .iter()
            .position(|o| *o == self)
            .expect("in VALUES") as i32
    }

    /// `getEnumString()`: the translation key.
    pub const fn get_enum_string(self) -> &'static str {
        self.data().0
    }
}

/// `RENDER_DISTANCES`.
// src: GameSettings.java:12
pub const RENDER_DISTANCES: [&str; 4] = [
    "options.renderDistance.far",
    "options.renderDistance.normal",
    "options.renderDistance.short",
    "options.renderDistance.tiny",
];
/// `DIFFICULTIES`.
// src: GameSettings.java:13
pub const DIFFICULTIES: [&str; 4] = [
    "options.difficulty.peaceful",
    "options.difficulty.easy",
    "options.difficulty.normal",
    "options.difficulty.hard",
];
/// `GUISCALES`.
// src: GameSettings.java:14
pub const GUISCALES: [&str; 4] = [
    "options.guiScale.auto",
    "options.guiScale.small",
    "options.guiScale.normal",
    "options.guiScale.large",
];
/// `LIMIT_FRAMERATES`.
// src: GameSettings.java:15
pub const LIMIT_FRAMERATES: [&str; 3] = [
    "performance.max",
    "performance.balanced",
    "performance.powersaver",
];

/// Index of `keyBindForward` in [`GameSettings::key_bindings`].
pub const KEY_FORWARD: usize = 0;
/// `keyBindLeft`.
pub const KEY_LEFT: usize = 1;
/// `keyBindBack`.
pub const KEY_BACK: usize = 2;
/// `keyBindRight`.
pub const KEY_RIGHT: usize = 3;
/// `keyBindJump`.
pub const KEY_JUMP: usize = 4;
/// `keyBindSneak`.
pub const KEY_SNEAK: usize = 5;
/// `keyBindDrop`.
pub const KEY_DROP: usize = 6;
/// `keyBindInventory`.
pub const KEY_INVENTORY: usize = 7;
/// `keyBindChat`.
pub const KEY_CHAT: usize = 8;
/// `keyBindToggleFog`.
pub const KEY_TOGGLE_FOG: usize = 9;

/// What a settings change asks of other subsystems (module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsEffect {
    /// `mc.sndManager.onSoundOptionsChanged()` (WP-3.8).
    SoundOptionsChanged,
    /// `mc.renderGlobal.loadRenderers()` (WP-3.3).
    LoadRenderers,
    /// `mc.renderEngine.refreshTextures()` (WP-3.2).
    RefreshTextures,
}

/// `GameSettings`.
#[derive(Clone, Debug, PartialEq)]
pub struct GameSettings {
    /// `musicVolume`.
    // src: GameSettings.java:16
    pub music_volume: f32,
    /// `soundVolume`.
    // src: GameSettings.java:17
    pub sound_volume: f32,
    /// `mouseSensitivity`.
    // src: GameSettings.java:18
    pub mouse_sensitivity: f32,
    /// `invertMouse`.
    // src: GameSettings.java:19
    pub invert_mouse: bool,
    /// `renderDistance`: 0 far … 3 tiny.
    // src: GameSettings.java:20
    pub render_distance: i32,
    /// `viewBobbing`.
    // src: GameSettings.java:21
    pub view_bobbing: bool,
    /// `anaglyph`.
    // src: GameSettings.java:22
    pub anaglyph: bool,
    /// `advancedOpengl`.
    // src: GameSettings.java:23
    pub advanced_opengl: bool,
    /// `limitFramerate`: 0 max, 1 balanced, 2 power saver.
    // src: GameSettings.java:24
    pub limit_framerate: i32,
    /// `fancyGraphics`.
    // src: GameSettings.java:25
    pub fancy_graphics: bool,
    /// `ambientOcclusion`.
    // src: GameSettings.java:26
    pub ambient_occlusion: bool,
    /// `skin`: the texture pack's name (texture packs are deferred, D3).
    // src: GameSettings.java:27
    pub skin: String,
    /// `keyBindings`, in the constructor's order (forward, left, back,
    /// right, jump, sneak, drop, inventory, chat, fog); the `keyBind*` fields
    /// are the [`KEY_FORWARD`]… indices.
    // src: GameSettings.java:28-38, :54
    pub key_bindings: [KeyBinding; 10],
    /// `difficulty`.
    // src: GameSettings.java:41, :55
    pub difficulty: i32,
    /// `hideGUI` (F1).
    // src: GameSettings.java:42, :56
    pub hide_gui: bool,
    /// `thirdPersonView` (F5).
    // src: GameSettings.java:43, :57
    pub third_person_view: bool,
    /// `showDebugInfo` (F3).
    // src: GameSettings.java:44, :58
    pub show_debug_info: bool,
    /// `lastServer` (multiplayer, D3; kept for the file).
    // src: GameSettings.java:45, :59
    pub last_server: String,
    /// `noclip`.
    // src: GameSettings.java:46, :60
    pub noclip: bool,
    /// `smoothCamera` (F8).
    // src: GameSettings.java:47, :61
    pub smooth_camera: bool,
    /// `debugCamEnable`.
    // src: GameSettings.java:48, :62
    pub debug_cam_enable: bool,
    /// `noclipRate`.
    // src: GameSettings.java:49, :63
    pub noclip_rate: f32,
    /// `debugCamRate`.
    // src: GameSettings.java:50, :64
    pub debug_cam_rate: f32,
    /// `guiScale`: 0 auto, 1 small, 2 normal, 3 large.
    // src: GameSettings.java:51, :65
    pub gui_scale: i32,
}

impl Default for GameSettings {
    /// `new GameSettings()`: the field initialisers and the constructor's
    /// assignments.
    // src: GameSettings.java:16-38, :71-84
    fn default() -> Self {
        GameSettings {
            music_volume: 1.0,
            sound_volume: 1.0,
            mouse_sensitivity: 0.5,
            invert_mouse: false,
            render_distance: 0,
            view_bobbing: true,
            anaglyph: false,
            advanced_opengl: false,
            limit_framerate: 1,
            fancy_graphics: true,
            ambient_occlusion: true,
            skin: "Default".to_string(),
            key_bindings: [
                KeyBinding {
                    key_description: "key.forward",
                    key_code: 17,
                },
                KeyBinding {
                    key_description: "key.left",
                    key_code: 30,
                },
                KeyBinding {
                    key_description: "key.back",
                    key_code: 31,
                },
                KeyBinding {
                    key_description: "key.right",
                    key_code: 32,
                },
                KeyBinding {
                    key_description: "key.jump",
                    key_code: 57,
                },
                KeyBinding {
                    key_description: "key.sneak",
                    key_code: 42,
                },
                KeyBinding {
                    key_description: "key.drop",
                    key_code: 16,
                },
                KeyBinding {
                    key_description: "key.inventory",
                    key_code: 18,
                },
                KeyBinding {
                    key_description: "key.chat",
                    key_code: 20,
                },
                KeyBinding {
                    key_description: "key.fog",
                    key_code: 33,
                },
            ],
            difficulty: 2,
            hide_gui: false,
            third_person_view: false,
            show_debug_info: false,
            last_server: String::new(),
            noclip: false,
            smooth_camera: false,
            debug_cam_enable: false,
            noclip_rate: 1.0,
            debug_cam_rate: 1.0,
            gui_scale: 0,
        }
    }
}

impl GameSettings {
    /// The binding at index `i` ([`KEY_FORWARD`]…).
    pub fn key(&self, i: usize) -> i32 {
        self.key_bindings[i].key_code
    }

    /// `getKeyBindingDescription(i)`: the translated binding name.
    // src: GameSettings.java:86-89
    pub fn get_key_binding_description(&self, var1: usize, t: &StringTranslate) -> String {
        t.translate_key(self.key_bindings[var1].key_description)
    }

    /// `getOptionDisplayString(i)`: `Keyboard.getKeyName` of the binding
    /// (LWJGL returns null for an unnamed code; Java string concatenation
    /// would print `null`).
    // src: GameSettings.java:91-93
    pub fn get_option_display_string(&self, var1: usize) -> String {
        crate::keys::key_name(self.key_bindings[var1].key_code)
            .unwrap_or("null")
            .to_string()
    }

    /// `setKeyBinding(i, code)` without the `saveOptions()` (the caller's).
    // src: GameSettings.java:95-98
    pub fn set_key_binding(&mut self, var1: usize, var2: i32) {
        self.key_bindings[var1].key_code = var2;
    }

    /// `setOptionFloatValue(option, value)`.
    // src: GameSettings.java:100-115
    pub fn set_option_float_value(
        &mut self,
        var1: EnumOptions,
        var2: f32,
    ) -> Option<SettingsEffect> {
        let mut effect = None;
        if var1 == EnumOptions::Music {
            self.music_volume = var2;
            effect = Some(SettingsEffect::SoundOptionsChanged);
        }
        if var1 == EnumOptions::Sound {
            self.sound_volume = var2;
            effect = Some(SettingsEffect::SoundOptionsChanged);
        }
        if var1 == EnumOptions::Sensitivity {
            self.mouse_sensitivity = var2;
        }
        effect
    }

    /// `setOptionValue(option, step)` without the final `saveOptions()`
    /// (the caller's).
    // src: GameSettings.java:117-163
    pub fn set_option_value(&mut self, var1: EnumOptions, var2: i32) -> Option<SettingsEffect> {
        let mut effect = None;
        if var1 == EnumOptions::InvertMouse {
            self.invert_mouse = !self.invert_mouse;
        }
        if var1 == EnumOptions::RenderDistance {
            self.render_distance = self.render_distance.wrapping_add(var2) & 3;
        }
        if var1 == EnumOptions::GuiScale {
            self.gui_scale = self.gui_scale.wrapping_add(var2) & 3;
        }
        if var1 == EnumOptions::ViewBobbing {
            self.view_bobbing = !self.view_bobbing;
        }
        if var1 == EnumOptions::AdvancedOpengl {
            self.advanced_opengl = !self.advanced_opengl;
            effect = Some(SettingsEffect::LoadRenderers);
        }
        if var1 == EnumOptions::Anaglyph {
            self.anaglyph = !self.anaglyph;
            effect = Some(SettingsEffect::RefreshTextures);
        }
        if var1 == EnumOptions::FramerateLimit {
            // Java `%`: the remainder has the dividend's sign.
            self.limit_framerate = (self.limit_framerate.wrapping_add(var2).wrapping_add(3)) % 3;
        }
        if var1 == EnumOptions::Difficulty {
            self.difficulty = self.difficulty.wrapping_add(var2) & 3;
        }
        if var1 == EnumOptions::Graphics {
            self.fancy_graphics = !self.fancy_graphics;
            effect = Some(SettingsEffect::LoadRenderers);
        }
        if var1 == EnumOptions::AmbientOcclusion {
            self.ambient_occlusion = !self.ambient_occlusion;
            effect = Some(SettingsEffect::LoadRenderers);
        }
        effect
    }

    /// `getOptionFloatValue(option)`.
    // src: GameSettings.java:165-173
    pub fn get_option_float_value(&self, var1: EnumOptions) -> f32 {
        match var1 {
            EnumOptions::Music => self.music_volume,
            EnumOptions::Sound => self.sound_volume,
            EnumOptions::Sensitivity => self.mouse_sensitivity,
            _ => 0.0,
        }
    }

    /// `getOptionOrdinalValue(option)` (through
    /// `EnumOptionsMappingHelper.enumOptionsMappingHelperArray`, which maps
    /// the five boolean options to cases 1-5).
    // src: GameSettings.java:175-190, EnumOptionsMappingHelper.java:5-34
    pub fn get_option_ordinal_value(&self, var1: EnumOptions) -> bool {
        match var1 {
            EnumOptions::InvertMouse => self.invert_mouse,
            EnumOptions::ViewBobbing => self.view_bobbing,
            EnumOptions::Anaglyph => self.anaglyph,
            EnumOptions::AdvancedOpengl => self.advanced_opengl,
            EnumOptions::AmbientOcclusion => self.ambient_occlusion,
            _ => false,
        }
    }

    /// `getKeyBinding(option)`: the options screen's button label. An
    /// out-of-range `renderDistance`/`difficulty`/`guiScale`/`limitFramerate`
    /// read from a hand-edited file throws `ArrayIndexOutOfBoundsException`
    /// in Java; here it panics the same way.
    // src: GameSettings.java:192-222
    pub fn get_key_binding(&self, var1: EnumOptions, var2: &StringTranslate) -> String {
        let var3 = format!("{}: ", var2.translate_key(var1.get_enum_string()));
        if var1.get_enum_float() {
            let var5 = self.get_option_float_value(var1);
            if var1 == EnumOptions::Sensitivity {
                if var5 == 0.0 {
                    var3 + &var2.translate_key("options.sensitivity.min")
                } else if var5 == 1.0 {
                    var3 + &var2.translate_key("options.sensitivity.max")
                } else {
                    format!("{var3}{}%", (var5 * 200.0) as i32)
                }
            } else if var5 == 0.0 {
                var3 + &var2.translate_key("options.off")
            } else {
                format!("{var3}{}%", (var5 * 100.0) as i32)
            }
        } else if var1.get_enum_boolean() {
            if self.get_option_ordinal_value(var1) {
                var3 + &var2.translate_key("options.on")
            } else {
                var3 + &var2.translate_key("options.off")
            }
        } else if var1 == EnumOptions::RenderDistance {
            var3 + &var2.translate_key(RENDER_DISTANCES[self.render_distance as usize])
        } else if var1 == EnumOptions::Difficulty {
            var3 + &var2.translate_key(DIFFICULTIES[self.difficulty as usize])
        } else if var1 == EnumOptions::GuiScale {
            var3 + &var2.translate_key(GUISCALES[self.gui_scale as usize])
        } else if var1 == EnumOptions::FramerateLimit {
            // StatCollector.translateToLocal = StringTranslate.translateKey
            // (StatCollector.java:6-8).
            var3 + &var2.translate_key(LIMIT_FRAMERATES[self.limit_framerate as usize])
        } else if var1 == EnumOptions::Graphics {
            if self.fancy_graphics {
                var3 + &var2.translate_key("options.graphics.fancy")
            } else {
                var3 + &var2.translate_key("options.graphics.fast")
            }
        } else {
            var3
        }
    }

    /// `loadOptions()`'s line loop over the file's text. A line whose value
    /// does not parse (or is missing) is skipped ("Skipping bad option"),
    /// leaving the earlier lines applied. Returns the skipped lines.
    // src: GameSettings.java:224-312
    pub fn load_options_text(&mut self, text: &str) -> Vec<String> {
        let mut skipped = Vec::new();
        for var2 in read_lines(text) {
            if self.load_option_line(var2).is_err() {
                // System.out.println("Skipping bad option: " + var2)
                log::warn!("Skipping bad option: {var2}");
                skipped.push(var2.to_string());
            }
        }
        skipped
    }

    /// One line of `loadOptions` (the body of its `try`): `Err` where Java
    /// throws (`ArrayIndexOutOfBoundsException` for a missing value,
    /// `NumberFormatException`).
    // src: GameSettings.java:234-300
    fn load_option_line(&mut self, var2: &str) -> Result<(), ()> {
        let var3 = jfmt::split(var2, ':');
        // `var3[0]` itself throws for a line of colons only ("::").
        let k = *var3.first().ok_or(())?;
        let v = || var3.get(1).copied().ok_or(());
        let float = |s: &str| parse_float_option(s).map_err(|_| ());
        let int = |s: &str| jfmt::parse_int(s).map_err(|_| ());
        if k == "music" {
            self.music_volume = float(v()?)?;
        }
        if k == "sound" {
            self.sound_volume = float(v()?)?;
        }
        if k == "mouseSensitivity" {
            self.mouse_sensitivity = float(v()?)?;
        }
        if k == "invertYMouse" {
            self.invert_mouse = v()? == "true";
        }
        if k == "viewDistance" {
            self.render_distance = int(v()?)?;
        }
        if k == "guiScale" {
            self.gui_scale = int(v()?)?;
        }
        if k == "bobView" {
            self.view_bobbing = v()? == "true";
        }
        if k == "anaglyph3d" {
            self.anaglyph = v()? == "true";
        }
        if k == "advancedOpengl" {
            self.advanced_opengl = v()? == "true";
        }
        if k == "fpsLimit" {
            self.limit_framerate = int(v()?)?;
        }
        if k == "difficulty" {
            self.difficulty = int(v()?)?;
        }
        if k == "fancyGraphics" {
            self.fancy_graphics = v()? == "true";
        }
        if k == "ao" {
            self.ambient_occlusion = v()? == "true";
        }
        if k == "skin" {
            self.skin = v()?.to_string();
        }
        if k == "lastServer" && var3.len() >= 2 {
            self.last_server = var3[1].to_string();
        }
        for var4 in 0..self.key_bindings.len() {
            if k == format!("key_{}", self.key_bindings[var4].key_description) {
                self.key_bindings[var4].key_code = int(v()?)?;
            }
        }
        Ok(())
    }

    /// `saveOptions()`'s text: one `println` per option (`\n` line ends,
    /// the Linux `line.separator`), floats as `Float.toString`.
    // src: GameSettings.java:322-351
    pub fn save_options_text(&self) -> String {
        let f = jfmt::float_to_string;
        let mut s = String::new();
        let mut line = |l: String| {
            s.push_str(&l);
            s.push('\n');
        };
        line(format!("music:{}", f(self.music_volume)));
        line(format!("sound:{}", f(self.sound_volume)));
        line(format!("invertYMouse:{}", self.invert_mouse));
        line(format!("mouseSensitivity:{}", f(self.mouse_sensitivity)));
        line(format!("viewDistance:{}", self.render_distance));
        line(format!("guiScale:{}", self.gui_scale));
        line(format!("bobView:{}", self.view_bobbing));
        line(format!("anaglyph3d:{}", self.anaglyph));
        line(format!("advancedOpengl:{}", self.advanced_opengl));
        line(format!("fpsLimit:{}", self.limit_framerate));
        line(format!("difficulty:{}", self.difficulty));
        line(format!("fancyGraphics:{}", self.fancy_graphics));
        line(format!("ao:{}", self.ambient_occlusion));
        line(format!("skin:{}", self.skin));
        line(format!("lastServer:{}", self.last_server));
        for var2 in &self.key_bindings {
            line(format!("key_{}:{}", var2.key_description, var2.key_code));
        }
        s
    }
}

/// `parseFloat(s)` (private): `"true"` is 1, `"false"` 0, else
/// `Float.parseFloat`.
// src: GameSettings.java:314-320
fn parse_float_option(var1: &str) -> Result<f32, jfmt::NumberFormatException> {
    if var1 == "true" {
        Ok(1.0)
    } else if var1 == "false" {
        Ok(0.0)
    } else {
        jfmt::parse_float(var1)
    }
}

/// `BufferedReader.readLine()` over a whole text: lines end at `\n`, `\r`
/// or `\r\n`; a final line without an end is still a line.
fn read_lines(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        match rest.find(['\n', '\r']) {
            None => {
                out.push(rest);
                break;
            }
            Some(i) => {
                out.push(&rest[..i]);
                let skip = if rest[i..].starts_with("\r\n") { 2 } else { 1 };
                rest = &rest[i + skip..];
            }
        }
    }
    out
}

/// The `options.txt` storage path (`new File(mcDataDir, "options.txt")`,
/// GameSettings.java:67): at the storage root, which is the data directory.
pub const OPTIONS_FILE: &str = "options.txt";

/// `new GameSettings(mc, dataDir)`: the defaults, then `loadOptions()` when
/// `options.txt` exists. A read failure is printed ("Failed to load
/// options") and the defaults kept.
// src: GameSettings.java:53-69, :224-312
pub fn load_options(storage: &mut dyn Storage) -> GameSettings {
    let mut s = GameSettings::default();
    match storage.read(OPTIONS_FILE) {
        Ok(Some(bytes)) => {
            // `FileReader`: the platform charset; UTF-8 here.
            let text = String::from_utf8_lossy(&bytes);
            s.load_options_text(&text);
        }
        Ok(None) => {}
        Err(e) => log::warn!("Failed to load options: {e}"),
    }
    s
}

/// `saveOptions()`: writes `options.txt`; a failure is printed ("Failed to
/// save options").
// src: GameSettings.java:322-351
pub fn save_options(settings: &GameSettings, storage: &mut dyn Storage) {
    if let Err(e) = storage.write(OPTIONS_FILE, settings.save_options_text().as_bytes()) {
        log::warn!("Failed to save options: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_sim::save::MemoryStorage;

    /// The file vanilla writes for the defaults (the Prism instance's
    /// options.txt has exactly these lines).
    const DEFAULT_FILE: &str = "music:1.0\nsound:1.0\ninvertYMouse:false\nmouseSensitivity:0.5\n\
viewDistance:0\nguiScale:0\nbobView:true\nanaglyph3d:false\nadvancedOpengl:false\nfpsLimit:1\n\
difficulty:2\nfancyGraphics:true\nao:true\nskin:Default\nlastServer:\nkey_key.forward:17\n\
key_key.left:30\nkey_key.back:31\nkey_key.right:32\nkey_key.jump:57\nkey_key.sneak:42\n\
key_key.drop:16\nkey_key.inventory:18\nkey_key.chat:20\nkey_key.fog:33\n";

    #[test]
    fn defaults_save_as_vanilla_does() {
        assert_eq!(GameSettings::default().save_options_text(), DEFAULT_FILE);
    }

    #[test]
    fn every_field_round_trips() {
        let mut s = GameSettings {
            music_volume: 0.25,
            sound_volume: 0.0,
            mouse_sensitivity: 0.33333334,
            invert_mouse: true,
            render_distance: 3,
            view_bobbing: false,
            anaglyph: true,
            advanced_opengl: true,
            limit_framerate: 2,
            fancy_graphics: false,
            ambient_occlusion: false,
            skin: "Pack.zip".to_string(),
            difficulty: 0,
            last_server: "example.org".to_string(),
            gui_scale: 2,
            ..GameSettings::default()
        };
        for (i, b) in s.key_bindings.iter_mut().enumerate() {
            b.key_code = 100 + i as i32;
        }
        let text = s.save_options_text();
        assert!(text.contains("mouseSensitivity:0.33333334\n"));
        assert!(text.contains("music:0.25\n"));
        assert!(text.contains("sound:0.0\n"));
        let mut back = GameSettings::default();
        assert!(back.load_options_text(&text).is_empty());
        assert_eq!(back, s);
    }

    #[test]
    fn bad_lines_are_skipped_and_the_rest_applies() {
        let mut s = GameSettings::default();
        let skipped = s.load_options_text(
            "music:loud\r\nsound:0.5\r\nskin:\r\nviewDistance:2\rkey_key.jump:x\nkey_key.drop:45\nunknown:1\nlastServer:\n",
        );
        assert_eq!(skipped, vec!["music:loud", "skin:", "key_key.jump:x"]);
        assert_eq!(s.music_volume, 1.0);
        assert_eq!(s.sound_volume, 0.5);
        assert_eq!(s.skin, "Default");
        assert_eq!(s.render_distance, 2);
        assert_eq!(s.key(KEY_JUMP), 57);
        assert_eq!(s.key(KEY_DROP), 45);
        assert_eq!(s.last_server, "");
    }

    #[test]
    fn float_option_accepts_booleans() {
        let mut s = GameSettings::default();
        s.load_options_text("music:false\nsound:true\nmouseSensitivity:1\n");
        assert_eq!(s.music_volume, 0.0);
        assert_eq!(s.sound_volume, 1.0);
        assert_eq!(s.mouse_sensitivity, 1.0);
    }

    #[test]
    fn option_steps_wrap_as_java() {
        let mut s = GameSettings::default();
        s.set_option_value(EnumOptions::RenderDistance, -1);
        assert_eq!(s.render_distance, 3);
        s.set_option_value(EnumOptions::RenderDistance, 1);
        assert_eq!(s.render_distance, 0);
        s.set_option_value(EnumOptions::FramerateLimit, 1);
        assert_eq!(s.limit_framerate, 2);
        s.set_option_value(EnumOptions::FramerateLimit, 1);
        assert_eq!(s.limit_framerate, 0);
        assert_eq!(
            s.set_option_value(EnumOptions::Graphics, 1),
            Some(SettingsEffect::LoadRenderers)
        );
        assert!(!s.fancy_graphics);
        s.set_option_value(EnumOptions::Difficulty, 1);
        assert_eq!(s.difficulty, 3);
        s.set_option_value(EnumOptions::Difficulty, 1);
        assert_eq!(s.difficulty, 0);
        s.set_option_value(EnumOptions::InvertMouse, 1);
        assert!(s.invert_mouse);
        assert_eq!(
            s.set_option_float_value(EnumOptions::Music, 0.5),
            Some(SettingsEffect::SoundOptionsChanged)
        );
        assert_eq!(s.get_option_float_value(EnumOptions::Music), 0.5);
    }

    #[test]
    fn enum_options_table() {
        assert_eq!(EnumOptions::VALUES.len(), 13);
        assert_eq!(
            EnumOptions::get_enum_options(3),
            Some(EnumOptions::Sensitivity)
        );
        assert_eq!(EnumOptions::get_enum_options(13), None);
        assert_eq!(
            EnumOptions::AmbientOcclusion.get_enum_string(),
            "options.ao"
        );
        assert!(EnumOptions::Sensitivity.get_enum_float());
        assert!(EnumOptions::ViewBobbing.get_enum_boolean());
        assert!(!EnumOptions::GuiScale.get_enum_boolean());
    }

    #[test]
    fn button_labels() {
        let t = StringTranslate::parse(
            b"options.sensitivity=Sensitivity\noptions.sensitivity.min=*yawn*\noptions.music=Music\n\
options.off=OFF\noptions.on=ON\noptions.viewBobbing=View Bobbing\noptions.renderDistance=Render Distance\n\
options.renderDistance.far=Far\n",
            b"",
        );
        let mut s = GameSettings::default();
        assert_eq!(
            s.get_key_binding(EnumOptions::Sensitivity, &t),
            "Sensitivity: 100%"
        );
        s.mouse_sensitivity = 0.0;
        assert_eq!(
            s.get_key_binding(EnumOptions::Sensitivity, &t),
            "Sensitivity: *yawn*"
        );
        assert_eq!(s.get_key_binding(EnumOptions::Music, &t), "Music: 100%");
        assert_eq!(
            s.get_key_binding(EnumOptions::ViewBobbing, &t),
            "View Bobbing: ON"
        );
        assert_eq!(
            s.get_key_binding(EnumOptions::RenderDistance, &t),
            "Render Distance: Far"
        );
        // Untranslated keys fall back to the key.
        assert_eq!(
            s.get_key_binding(EnumOptions::Anaglyph, &t),
            "options.anaglyph: OFF"
        );
    }

    #[test]
    fn storage_round_trip() {
        let mut st = MemoryStorage::default();
        let s = load_options(&mut st);
        assert_eq!(s, GameSettings::default());
        let mut s2 = s.clone();
        s2.fancy_graphics = false;
        save_options(&s2, &mut st);
        assert_eq!(load_options(&mut st), s2);
    }

    #[test]
    fn key_display_names() {
        let s = GameSettings::default();
        assert_eq!(s.get_option_display_string(KEY_FORWARD), "W");
        assert_eq!(s.get_option_display_string(KEY_JUMP), "SPACE");
        assert_eq!(s.get_option_display_string(KEY_SNEAK), "LSHIFT");
    }
}
