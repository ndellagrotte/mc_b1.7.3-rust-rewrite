# mc-client

The Bevy shell around `mc-sim` (PLAN_v4 D1, §3.5): the window, input, the game
loop of `Minecraft.java`, the camera, assets, saves on disk, and — in later
work packages — rendering, GUI and audio. The simulation is plain Rust and never
calls into Bevy; this crate drives it and turns its event queue into Bevy
messages.

Nothing of Mojang's is embedded (D8): textures and `lang/` come from the user's
`minecraft.jar`, sounds and music from a `resources/` directory, both read at
runtime.

## Running

```text
cargo run -p mc-app -- [--data-dir DIR] [--jar FILE] [--resources DIR] [--username NAME]
                       [--world NAME [--seed SEED] | --import DIR] [--script FILE.json]
```

| Flag | Meaning | Default |
|---|---|---|
| `--data-dir` | options, `saves/`, `screenshots/`, `stats/` | `$XDG_DATA_HOME/mc_b173` or `~/.local/share/mc_b173` (macOS `~/Library/Application Support/mc_b173`, Windows `%APPDATA%\mc_b173`); never the real `.minecraft` |
| `--jar` | your Beta 1.7.3 `minecraft.jar` | `.reference/mc_b1.7.3_release/1.7.3-LTS/jars/bin/minecraft.jar` in the workspace |
| `--resources` | the `resources/` directory (`sound/`, `newsound/`, `music/`, …) | the Prism Launcher (Flatpak) `b1.7.3` instance's |
| `--username` | the player name (names the stats file) | vanilla's `Player<millis % 1000>` |
| `--world NAME` | open `saves/NAME` (as the Select World screen does), or create it (as the Create World screen does) | title screen |
| `--seed SEED` | the new world's seed text: a `long`, else its `String.hashCode()`; empty or `0` = random | random |
| `--import DIR` | copy a vanilla save folder into `saves/` (refused, with nothing copied, at the first block or item id Beta 1.7.3 does not define — D5), then open it | — |
| `--script FILE` | the scripted-input dev mode (below) | — |

A missing jar or `resources/` directory stops the program with a message
naming the path.

## Module map

| Module | Vanilla | What |
|---|---|---|
| `game` | `Minecraft.java` | `Game` (`!Send` resource): the frame (`run`'s loop body), `runTick` with its hook schedules (`RunTickHook`), `startWorld`/`changeWorld`/`usePortal`/`respawn`/shutdown, `displayGuiScreen` and focus, `clickMouse`, `func_6254_a`, `clickMiddleMouseButton`, the keyboard and mouse loops; `FrameView` |
| `timer` | `Timer.java` | 20 TPS, the two-clock sync, at most 10 ticks a frame, `renderPartialTicks` |
| `settings` | `GameSettings`, `KeyBinding`, `EnumOptions`, `EnumOptionsMappingHelper` | the options and `options.txt` (Java formatting, bad lines skipped) |
| `keys` | LWJGL `Keyboard` | LWJGL key codes ↔ names ↔ Bevy `KeyCode` |
| `input` | `MouseHelper`, `MovementInputFromOptions`, `MouseFilter`, LWJGL `Mouse`/`Keyboard` | the buffered events `runTick` drains, live key/button state, cursor grab |
| `camera` | `EntityRenderer` (camera half) | `updateRenderer`, `getMouseOver`, `getFOVModifier`, `hurtCameraEffect`, `setupViewBobbing`, `orientCamera`, `setupCameraTransform`; the Bevy `Camera3d` (`MainCamera`, `McProjection`, `CameraState`) |
| `storage` | — (D9) | the native `Storage` (atomic write-then-rename) and the wall `Clock` |
| `assets`, `lang` | `TexturePackDefault`/`TexturePackBase` (paths), `StringTranslate` | the jar (indexed in memory; PNG → RGBA / Bevy `Image`), `resources/`, the translation table |
| `session` | `Session` | the player name |
| `loading` | `LoadingScreenRenderer`, `IProgressUpdate` | the loading screen's text and progress (drawn by WP-3.6) |
| `app` | `GuiScreen` (two flags), `ScaledResolution` | `AppState` (`Title`, `Loading`, `InGame`, `Paused`), the screen slot |
| `screenshot` | `ScreenShotHelper` | F2 → `<data>/screenshots/yyyy-MM-dd_HH.mm.ss[_N].png` |
| `import` | — (D5) | the import of a vanilla save |
| `debug` | `GuiIngame`'s F3 block | the F3 text (`DebugText`, `DebugSources`) |
| `events` | `IWorldAccess`, `SoundManager` call sites, `EntityPlayerSP`'s calls into `Minecraft` | `WorldEventMsg`, `ClientSound`, `ClientEvent` |
| `sound` (WP-3.8) | `SoundManager`, `SoundPool`, `SoundPoolEntry`, `CodecMus`/`MusInputStream`, `RenderGlobal.playSound`/`playRecord`/`playAuxSFX` (sound half) | the `SoundManager` resource on `bevy_kira_audio` (below) |
| `script` | — | the scripted-input dev mode |
| `textures` | `RenderEngine`, `TextureFX` and its subclasses, `ColorizerGrass`/`ColorizerFoliage`/`ColorizerWater` | the texture cache (`Handle<Image>` by jar name, with a CPU copy), the terrain and items atlases, the animated tiles written every tick, the colour maps (WP-3.2) |
| `jfmt`, `clock`, `paths` | JDK behaviour | `Float/Double.toString`, `parseFloat`/`parseInt`, `String.split`/`hashCode`; the clocks and `new Random()`'s seed; the default paths |
| `sky` | `RenderGlobal.renderSky` and its display lists | the sky and void planes, sunrise glow, sun, moon, stars; `SkyPlugin` (all of WP-3.4) and the per-frame world read `compute_render_frame` |
| `fog` | `EntityRenderer.updateFogColor`, `setupFog` | `FogState` (the fog and clear colour, the world's and the sky's fog), the clear colour, a `DistanceFog` on the world camera |
| `clouds` | `RenderGlobal.renderClouds`, `renderCloudsFancy`, `updateClouds` | fast and fancy clouds, `RenderGlobalClouds::cloud_offset_x` |
| `weather` | `EntityRenderer.renderRainSnow` | rain and snow columns |
| `selection` | `RenderGlobal.drawSelectionBox` | the block outline |
| `gl_material` | — (GL state) | `GlMaterial`: glColor × vertex colour × texture, alpha test, GL fog, blend/depth/colour masks, draw-order biases; Tessellator helpers |

`mc-sim/src/world/teleporter.rs` (`Teleporter.java`) is the simulation half of
nether travel.

## For the other client work packages

- The frame runs in `Update` in the `GameFrameSet` system set. Order your
  systems `.after(GameFrameSet)` to read this frame's state.
- `NonSend<Game>` / `NonSendMut<Game>` is `Minecraft`: `the_world`,
  `the_player`, `player()`, `game_settings`, `current_screen`,
  `loading_screen`, `object_mouse_over`, `timer`, `entity_renderer`,
  `input` (the LWJGL buffers), `chunks_updated`, `frame_times`/`tick_times`,
  `damage_partial_time`, and `is_gui_enabled()` /
  `is_fancy_graphics_enabled()` / `is_ambient_occlusion_enabled()` /
  `is_debug_info_enabled()` (the `Minecraft` statics).
- Where `runTick` calls into your part, add systems to the matching schedule
  (they run inside the tick, with `NonSendMut<Game>`):
  `RunTickHook::IngameGuiUpdateTick` (3.6), `UpdateDynamicTextures` (3.2),
  `ScreenInput` (3.6: drain `game.input`'s buffers; insert
  `ScreenInputOwner(true)` to take over from the shell's Escape-only
  default), `UpdateRenderer` (3.7 item renderer, 3.4 rain particles),
  `UpdateClouds` (3.4), `UpdateEffects` (3.7).
- Messages: `WorldEventMsg` (every `IWorldAccess` call, in order, flushed
  before each hook and at the end of the frame), `ClientSound` (3.8),
  `ClientEvent` (world changed, block-hit particles, chat lines, screens
  opened/closed/resized, settings side effects, equipped-item resets).
- Resources: `FrameView` (partial ticks, interpolated eye and rotation,
  mouse-over, the crack overlay's `damage_partial_time` from
  `playerController.setPartialTime`), `CameraState` (the full camera setup,
  eye in f64; anaglyph's two passes call `camera::setup_camera_transform`
  with `Some(field)` and apply `projection_shift_x` before the
  perspective),
  `DebugText` / `DebugSources`, `JarAssets`, `Lang`, `ResourcesDir`,
  `PendingScreenshots`; state `AppState`.
- `EntityPlayerSP`'s calls into `Minecraft` reach the shell as
  `mc_sim::world::client_requests::ClientRequest`s on
  `World::client_requests` (WP-3.1b), collected with the world events
  (`Game::collect_world_events`) and forwarded in call order:
  `displayGUIChest` / `displayWorkbenchGUI` / `displayGUIFurnace` /
  `displayGUIDispenser` / `displayGUIEditSign` as
  `ClientEvent::DisplayGui(PlayerGui)` (3.6 opens the screen with
  `Game::display_gui_screen`; the chest carries its `IInventory`, a double
  chest's `InventoryLargeChest`, the furnace, dispenser and sign their tile
  entity, the minecart chest its entity id), `addChatMessage` from
  `BlockBed` (`tile.bed.occupied`, `tile.bed.noSleep`) as
  `ClientEvent::ChatMessageTranslate`, `onItemPickup`'s `EntityPickupFX`
  as `ClientEvent::PickupFx` (3.7; a copy of the picked entity as the
  pickup left it, the player by id, `yOffs` -0.5), the inventory hint
  `GuiAchievement.queueAchievementInformation(openInventory)` as
  `ClientEvent::AchievementHint` (every player update while it is not
  unlocked; 3.6), and the portal sounds `portal.trigger` /
  `portal.travel` as `ClientSound::PlaySoundFx` (3.8; pitch drawn by the
  simulation). `closeScreen` (`ClientRequest::CloseScreen`,
  EntityPlayerSP.java:108-111) closes the screen
  (`Game::display_gui_screen(None)`); its simulation caller, the
  `craftingInventory.isUsableByPlayer` check of `EntityPlayer.onUpdate`,
  waits for the GUI containers (3.6). `usePortal` is split: its head (the
  dimension toggle, `setEntityDead`, x and z scaled by 8) runs inside the
  player's update (`player::use_portal_head`), so the rest of the old
  world's `updateEntities` no longer sees the player there, as in vanilla;
  the update stops there and raises `ClientRequest::UsePortal { x, z }`.
  The shell runs the rest of `usePortal` after `updateEntities` (the
  old-world `updateEntityWithOptionalForce`, the new world, `changeWorld`,
  the teleporter), then finishes the update on the new world
  (`player::sp_on_living_update_after_portal`) and the rest of the tick.
- For 3.6: vanilla opens a `displayGUI*` screen inside the click or update
  that asked for it, so the rest of that `runTick`'s key and mouse events
  go to the screen. `ClientEvent::DisplayGui` arrives when the shell
  collects the world events; the screen owner should open it before the
  next input event is dispatched (drain right after each player-controller
  call in the click path).

## Textures (WP-3.2)

`textures::Textures` (a `Resource`) is `RenderEngine`:

- `get_texture(name, &jar, &mut images) -> Handle<Image>` is
  `getTexture(name)`: the jar's PNG (`"/terrain.png"`, `"/gui/gui.png"`,
  …, with vanilla's `%clamp%`, `%blur%` and `##` prefixes), uploaded once
  and cached by name; a missing one is the 64×64 "missingtex" image. In a
  system take the `textures::TextureLoader` param and call `get(name)`.
  Images are RGBA8 sRGB with one mip level and a nearest sampler (repeat;
  `%clamp%` clamps, `%blur%` is linear); with the anaglyph option on, the
  colours are converted on upload as vanilla does.
- `entry(name)` / `entry_by_handle(h)` give a `TextureEntry`: the handle,
  size and a CPU copy (`rgba`) of what was uploaded. The atlases are
  `textures::TERRAIN_PNG` and `textures::ITEMS_PNG` (`terrain()`/`items()`
  load them; both are loaded at start-up). Their CPU copies include the
  animated tiles.
- `textures::tile_uv(index) -> [u_min, v_min, u_max, v_max]` is
  `RenderBlocks`' full-tile rectangle (`x = (i & 15) << 4`, `y = i & 240`,
  `/ 256`, the far edge `- 0.01`); `tile_uv_f64` keeps the Java doubles.
- `get_texture_contents(name)` is `func_28149_a` (ARGB ints).
  `allocate_and_setup_texture`, `upload_argb` (`func_28150_a`) and
  `delete_texture` are there for the font and the map. `upload_argb` takes
  `&mut Assets<Image>` and writes the Bevy image at once (vanilla's
  `glTexSubImage2D` runs at render time, also while paused), so call it from
  the system that draws.
- The FX (`texture_list`, registered in `Minecraft.startGame`'s order: lava,
  water, portal, compass, watch, flowing water, flowing lava, fire ×2) tick
  in `RunTickHook::UpdateDynamicTextures` and write their 16×16 tiles into
  the atlas (terrain: fire 31 and 47, water 205 and flowing 206/207/222/223,
  lava 237 and flowing 238/239/254/255, portal 14; items: compass 54, clock
  70). The Bevy image is rewritten after each update (`AssetEvent::Modified`).
  Their `Math.random()` is the client's own clock-seeded stream
  (`Textures::math_random`), never the simulation's.
- `textures::Colorizers` (a `Resource`) holds the grass, foliage and water
  colour maps: `get_grass_color(temperature, rainfall)`,
  `get_foliage_color(..)`, and `colorizer::get_foliage_color_pine/birch/basic()`.
- Switching the anaglyph option (`SettingsEffect::RefreshTextures`) runs
  `refreshTextures` in `Update` after `GameFrameSet`, in
  `textures::TexturesSet`; order systems that read the atlases after it.

Texture packs (`TexturePackList`, `TexturePackCustom`) and the skin and cloak
downloads (`getTextureForDownloadableImage`) are deferred (D3, D2).

## Sky, clouds, weather, fog (WP-3.4)

- Every frame after `GameFrameSet`, `sky::compute_render_frame` reads the
  world in `renderWorld`'s order and publishes `fog::FogState`,
  `sky::SkyFrame`, `clouds::CloudFrame`, `weather::WeatherFrame`,
  `selection::SelectionFrame`; `apply_*` systems update the meshes.
- **Fog for everyone else:** read `FogState` (`color`, `world` =
  `setupFog(0)`, `sky` = `setupFog(-1)`, `far_plane_distance`) and apply
  `FogParams::factor(eye distance)` to the colour *in the Java's colour
  values* (or set `GlParams::fog` from `FogParams::to_vec4` if drawing with
  `GlMaterial`). The world camera also carries a matching `DistanceFog` for
  `StandardMaterial` meshes (blended in linear space, and that material
  applies the camera's tonemapping in the shader on LDR cameras, so its
  colours are not byte-exact).
- **Draw order:** everything here is in `Transparent3d`, sorted by
  `gl_material::order` biases: the sky first (drawn at the far plane, no
  depth writes, so any opaque world covers it), then (after the opaque pass)
  the selection box at `SELECTION_IN_WATER` (−0.91e6) when the eye is in
  water, then translucent terrain, then the selection box at `SELECTION`
  (0.7e6) otherwise, snow, rain, the fancy clouds' depth pass and the
  clouds (EntityRenderer.java:507-564). Bands: transparent items vanilla
  draws before the translucent terrain pass (entities, particles) between
  −0.95e6 and −0.91e6; translucent terrain between −0.9e6 and 0.7e6. Do not
  draw anything that writes depth with
  `TransparentSortingInfo3d::AlwaysOnTop` (Bevy gizmos do): it is drawn
  before the sky and punches the clear colour through it.
- **Colours:** `GlMaterial` does GL's arithmetic on the Java values and
  converts to linear at the end, so opaque pixels are the bytes vanilla
  writes (checked: the sky plane's pixels equal `getSkyColor` fogged by
  `setupFog(-1)` exactly). Textures it samples are uploaded as `Rgba8Unorm`.
  Blending: on the default sRGB target the hardware blends linear values,
  not GL's bytes; additive draws (sun, moon, stars) return `linear(c·a)/a`,
  exact over black (midnight stars: byte 64 as in GL, 92 before), and
  alpha-blended draws are off where `0 < a < 1` (clouds 1-3 bytes; the
  selection outline 147 where GL writes 111; the sun's halo nearly
  invisible by day). With `bevy::camera::CompositingSpace::Srgb` on the
  world camera (main target `Rgba8Unorm`) `GlMaterial` returns the Java
  values unconverted and every blend is GL's (checked in an experiment:
  outline 111, stars 64, halo visible). Switching the camera is a
  client-wide decision (WP-3.1's `spawn_camera`): every other material must
  then return the Java values unconverted too (`gl_material` module docs).
- **Wide lines:** `GlKey::wide_lines` + `gl_material::wide_line_mesh` draw
  `glLineWidth` lines (`GlParams::line_width` pixels, widened along the
  minor screen axis as GL's aliased lines); wgpu itself draws 1-pixel lines.
- `clouds::RenderGlobalClouds::cloud_offset_x` is `RenderGlobal.cloudOffsetX`
  (WP-3.3's occlusion stagger reads it).
- Textures held locally (could move to WP-3.2's cache): `/terrain/sun.png`,
  `/terrain/moon.png`, `/environment/clouds.png`, `/environment/rain.png`,
  `/environment/snow.png`.

## Audio (WP-3.8)

`sound::SoundPlugin` (added by `ClientPlugin`) owns `bevy_kira_audio`'s
`AudioPlugin` and the `SoundManager` resource (`Minecraft.sndManager`).

- At start-up it runs `loadSoundSettings` and installs every file under
  `resources/` (`sound/`, `newsound/` → sounds; `streaming/` → records;
  `music/`, `newmusic/` → music; names sorted, since Java's `listFiles`
  order is unspecified). A file becomes a variant of its pool name:
  `step/grass1.ogg` → `step.grass`.
- Each frame after `GameFrameSet` it applies the `ClientSound` messages, the
  `SoundOptionsChanged` settings effect, and the simulation's
  `WorldEventMsg::{PlaySound, PlayRecord, PlayAuxSfx}` (with `RenderGlobal`'s
  16-block range check for `PlaySound`).
- Other parts call it directly: `ResMut<SoundManager>` then
  `play_sound_fx(name, volume, pitch)` (GUI clicks, WP-3.6),
  `play_sound(name, x, y, z, volume, pitch)`. Plays are handed to kira in
  `PostUpdate`.
- `RecordPlayingMessage("C418 - <name>")` is `ingameGUI.setRecordPlayingMessage`
  for the HUD (WP-3.6).
- `sound::library` models the paulscode/OpenAL behaviour vanilla relied on:
  pitch clamped to 0.5–2, linear distance attenuation (range 16 blocks, `16 ×
  volume` above 1, 64 for records) recomputed as the listener moves, and
  left/right panning of mono sounds from the listener's yaw (an approximation
  of OpenAL's panner; stereo files are not panned).
- `.mus` records are decoded (`sound::mus`) before kira sees them; music and
  records decode on the async task pool.
- `MC_SOUND_LOG=1` logs every decision (`sound: <name> -> <file> vol .. pitch
  .. range .. at (x, y, z)` or `fx`), misses (`sound: no such sound: ..`),
  out-of-range skips, music and record starts and stops.
- Without an audio device kira cannot start; after a few seconds of plays
  that never leave the queue the plugin logs one warning and keeps deciding
  (and logging) without output.

## Scripted-input dev mode

`--script file.json` runs a list of actions against the game instead of the
keyboard and mouse. Real input devices are ignored, the window counts as
focused, and the desktop pointer is never grabbed. Input goes through the same
LWJGL-shaped buffers as real input, so it takes vanilla's paths: key events are
handled in the next `runTick`, mouse motion in the next frame's camera update.

The file is `{"actions": [ ... ]}` (or just the array). Each action is an
object with `"action"` and its fields, plus optionally:

- `"tick": n` — not before `ticksRan` has grown by `n` since the script
  started;
- `"frame": n` — not before the `n`-th frame of the script.

Actions run in order, as many per frame as are due.

| Action | Fields | Effect |
|---|---|---|
| `key_down`, `key_up` | `key` | A key event. `key` is an LWJGL key name as the options screen shows it (`"W"`, `"SPACE"`, `"LSHIFT"`, `"ESCAPE"`, `"F3"`, `"1"`, …) or a binding (`"key.forward"`, `"key.jump"`, `"key.sneak"`, `"key.inventory"`, `"key.drop"`, `"key.fog"`, …, through the current options) |
| `mouse_move` | `dx`, `dy` | Mouse motion in pixels (`dy` > 0 is up, LWJGL's sense); applied by the next frame's mouse look |
| `mouse_down`, `mouse_up` | `button`: `"left"`, `"right"`, `"middle"` | A button event (held buttons repeat every 5 ticks, as vanilla) |
| `wheel` | `notches` | Wheel turns (positive = up) |
| `look` | `yaw`, `pitch` | Sets the player's rotation absolutely (and its previous rotation, so nothing is interpolated). Yaw 0 faces south (+z), 90 west, ±180 north; pitch > 0 looks down |
| `wait` | `ticks`, `frames`, `in_game` (any of them) | Waits that many game ticks / frames, and/or until a world is loaded with its player |
| `screenshot` | `path` | Captures the window into a PNG (parent directories created); the script waits until it is written |
| `dump` | `path` | Writes a JSON of the state: app state, `ticks_ran`, screen, focus, pause, third person, F3, the mouse-over, the world (dir, level name, seed, dimension, world time, loaded chunks, entities, spawn, rain) and the player (id, position, rotation, motion, on ground, health, dimension, hotbar slot, name, the portal state `in_portal` / `time_in_portal` / `time_until_portal`, `y_size`) |
| `save` | — | `saveWorldIndirectly` (saves the world and the player now) |
| `use_portal` | — | Dev: `Minecraft.usePortal()` at once |
| `respawn` | — | Dev: `Minecraft.respawn(false, 0)` |
| `quit` | — | `Minecraft.shutdown()`: leaves the world (saving it and the player) and exits |
| `dump_texture` | `name`, `path` | Dev (WP-3.2): writes the texture's current pixels (for `"terrain"` / `"items"` the atlas with its animated tiles; else a jar path as `getTexture` takes it) to a PNG |
| `set_time` | `world_time` | Dev (WP-3.4): `World.setWorldTime` (0 sunrise, 6000 noon, 12000 sunset, 18000 midnight) |
| `set_rain` | `strength` (0..1), `thunder` (bool, default false) | Dev (WP-3.4): sets `rainingStrength` (and the previous value), `raining`/`thundering` with long timers, and the thunder strength to 1 or 0; `updateWeather` then moves the strength 0.01 a tick toward 1 or 0 |
| `set_pos` | `x`, `y`, `z` | Dev (WP-3.4): moves the player (`posY` is the eye, as `dump` prints it), no interpolation, motion and fall distance cleared |
| `aux_sfx` | `kind`, `x`, `y`, `z`, `data` | Dev (WP-3.8): queues `world.playAuxSFX(kind, x, y, z, data)` as the simulation would (`1005` with `data` 2256 plays record `13` at the block, `data` 0 stops it; `2001` with a block id plays its break sound) |

Example (put screenshots and dumps under `build/screens/`, which git ignores):

```json
{"actions": [
  {"action": "wait", "in_game": true},
  {"action": "wait", "ticks": 60},
  {"action": "look", "yaw": 90, "pitch": 20},
  {"action": "key_down", "key": "key.forward"},
  {"action": "wait", "ticks": 20},
  {"action": "key_up", "key": "key.forward"},
  {"action": "screenshot", "path": "build/screens/a.png"},
  {"action": "dump", "path": "build/screens/a.json"},
  {"action": "quit"}
]}
```

```text
cargo run -p mc-app -- --data-dir /tmp/mcdata --world smoke --seed 123 --script a.json
```
