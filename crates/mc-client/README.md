# mc-client

The Bevy shell around `mc-sim` (PLAN_v4 D1, §3.5): the window, input, the game
loop of `Minecraft.java`, the camera, assets, saves on disk, and — in later
work packages — rendering, GUI and audio. The simulation is plain Rust and never
calls into Bevy; this crate drives it and turns its event queue into Bevy
messages.

Nothing of Mojang's is embedded (D8): textures and `lang/` come from the user's
Beta 1.7.3 client jar, sounds and music from a `resources/` directory, both
read at runtime from the user's Prism Launcher `b1.7.3` instance.

## Running

```text
cargo run -p mc-app -- [--data-dir DIR] [--prism-instance DIR] [--username NAME]
                       [--world NAME [--seed SEED] | --import DIR] [--script FILE.json]
```

| Flag | Meaning | Default |
|---|---|---|
| `--data-dir` | options, `saves/`, `screenshots/`, `stats/` | `$XDG_DATA_HOME/mc_b173` or `~/.local/share/mc_b173` (macOS `~/Library/Application Support/mc_b173`, Windows `%APPDATA%\mc_b173`); never the real `.minecraft` |
| `--prism-instance` | the Prism Launcher `b1.7.3` instance directory (`.../PrismLauncher/instances/b1.7.3`): the jar is `<instance>/../../libraries/com/mojang/minecraft/b1.7.3/minecraft-b1.7.3-client.jar`, `resources/` is `<instance>/minecraft/resources` (or `.minecraft/resources`) | the first `instances/b1.7.3` in the standard Prism data directories (`paths::prism_roots`: `$XDG_DATA_HOME/PrismLauncher` or `~/.local/share/PrismLauncher`, then the Flatpak's; macOS `~/Library/Application Support/PrismLauncher`; Windows `%APPDATA%\PrismLauncher`) |
| `--username` | the player name (names the stats file) | vanilla's `Player<millis % 1000>` |
| `--world NAME` | open `saves/NAME` (as the Select World screen does), or create it (as the Create World screen does) | title screen |
| `--seed SEED` | the new world's seed text: a `long`, else its `String.hashCode()`; empty or `0` = random | random |
| `--import DIR` | copy a vanilla save folder into `saves/` (refused, with nothing copied, at the first block or item id Beta 1.7.3 does not define — D5), then open it | — |
| `--script FILE` | the scripted-input dev mode (below) | — |

No instance found, a directory without `instance.cfg`, or an instance whose
jar or `resources/` is missing (never launched from Prism) stops the program
with a message naming the paths (`paths::PrismError`). At start-up three
stderr lines name the paths chosen (`mc_b173: prism instance: <path>`,
`mc_b173: minecraft.jar: <path>`, `mc_b173: resources: <path>`; see the
`paths` module docs and `docs/RUNNING.md`). Since WP-4.3:

| Flag | Meaning | Default |
|---|---|---|
| `--vsync on\|off` | `on` presents with `PresentMode::AutoVsync` (waits for the display), `off` with `PresentMode::AutoNoVsync` (project addition) | `off` since WP-4.9 (was `on`) |

Since WP-4.9 the frame rate follows vanilla's limit (PARITY_CHECKLIST P6):
b1.7.3 never turns on LWJGL's vsync, so "Performance: Balanced" (the
default) and "Max FPS" run uncapped and "Power saver" sleeps to 40 fps
(EntityRenderer.java:364-416, `terrain::power_saver_sleep`). `--vsync on`
is the override for a capped, tear-free window. A nested or covered window
may still be paced by the compositor (the nested Xwayland of the parity
harness presents the Vulkan surface at 60 Hz whatever the mode).

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
| `export` (WP-4.5) | — (D5) | the export of a world: validate, copy, re-read and compare (below, "Export") |
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
| `terrain` (WP-3.3a) | `RenderBlocks`, `WorldRenderer`, `RenderGlobal` (terrain half: `loadRenderers`, `updateRenderers`, `sortAndRender`, `clipRenderersByFrustrum`, `markBlock*NeedsUpdate`, `drawBlockBreaking`), `RenderSorter`, `EntitySorter`, `ChunkCache`, `Tessellator`, `Frustrum`, `ClippingHelper(Impl)` | the terrain meshes, two passes, the crack overlay, `renderBlockOnInventory` (below) |
| `model` (WP-3.5a) | `ModelBase` and every `Model*`, `ModelRenderer`, `TexturedQuad`, `PositionTextureVertex`, `RenderHelper.enableStandardItemLighting` (the lights) | the models as part tables with their pose functions, part meshes and transforms, `ModelMaterial` (GL state with GL lighting), the `debug_model` dev action |
| `gui` (WP-3.6a) | `Gui`, `GuiScreen`, `GuiButton`, `GuiSmallButton`, `GuiSlider`, `GuiTextField`, `GuiSlot`, `GuiYesNo`, `GuiErrorScreen`, `GuiConflictWarning`, `GuiUnused`, `FontRenderer`, `ChatAllowedCharacters`, `ScaledResolution` (arithmetic) | the screen objects bound to `currentScreen`, their input and drawing; `GuiGl` (the GL state and Tessellator as data) and the two GUI cameras (below) |
| `gui` (WP-3.6b) | `GuiIngame`, `ChatLine`, `GuiChat`, `GuiAchievement`, `RenderItem` (GUI half), `RenderHelper` (item lighting), `GuiContainer`, `GuiInventory`, `GuiChest`, `GuiFurnace`, `GuiCrafting`, `GuiDispenser`, `GuiEditSign`, `GuiSleepMP`, `GuiGameOver`, `Minecraft.displayDebugInfo` (graph) | the HUD, the chat, the achievement popup, the container and in-world screens (below) |
| `gui::menu`, `gui::loading`, `gui::screens_menu` (WP-3.6c) | `GuiMainMenu`, `GuiIngameMenu`, `GuiOptions`, `GuiVideoSettings`, `GuiControls`, `GuiSelectWorld`, `GuiWorldSlot`, `GuiCreateWorld`, `GuiRenameWorld`, `GuiAchievements`, `GuiStats` and its `GuiSlotStats*` / `SorterStats*`, `GuiTexturePacks`, `GuiTexturePackSlot`, `LoadingScreenRenderer` (drawing) | the title, pause, options, world and statistics screens, the D5 import screen and refusal dialog, the loading screen (below) |
| `particles` (WP-3.7a) | `EffectRenderer`, `EntityFX` and every `Entity*FX`, `RenderGlobal.spawnParticle`, `playAuxSFX` (particle half), `EntityRenderer.addRainParticles` | the `EffectRenderer` resource, the particle updates in `RunTickHook::UpdateEffects`, the rain splashes in `UpdateRenderer`, the particle meshes (below) |
| `item_renderer` (WP-3.7b) | `ItemRenderer`, `MapItemRenderer`, `EntityRenderer.renderHand`, `RenderPlayer.drawFirstPersonHand` (the call) | the first-person hand and held item, the fire / inside-a-block / water overlays, `updateEquippedItem`, `renderItem` for the entity renderers (`HeldItemHook`), the hand camera (below) |
| `render` (WP-3.5b) | `RenderManager`, `Render` and every `Render*` subclass, `RenderGlobal.renderEntities`, `TileEntityRenderer`, `TileEntitySpecialRenderer`, `TileEntitySignRenderer`, `TileEntityMobSpawnerRenderer`, `TileEntityRendererPiston`, `SignModel`, `RenderHelper` (the lit Tessellator draws), `EffectRenderer.renderLitParticles` (the pickups' entities) | the entity and tile-entity draws (below), `HeldItemHook` for 3.7b, the `spawn_entity` / `set_block` dev actions |

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
  waits for the GUI containers (3.6); *WP-4.7:* it runs in the
  simulation's player update (`entity::player::on_update`, with the
  drops), see "In-game GUI". `usePortal` is split: its head (the
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
- **Colours:** `GlMaterial` does GL's arithmetic on the Java values, so
  opaque pixels are the bytes vanilla writes (checked: the sky plane's
  pixels equal `getSkyColor` fogged by `setupFog(-1)` exactly). Textures it
  samples are uploaded as `Rgba8Unorm`. Blending: every camera composites
  in sRGB since WP-3.7b (see "Compositing in sRGB"): the main target is
  `Rgba8Unorm`, `GlMaterial` returns the Java values unconverted
  (`GL_TARGET_UNORM`) and every blend is GL's (outline 111, stars 64, the
  sun's halo visible). History: on Bevy's default sRGB target (before
  WP-3.7b) the shader converted to linear and the hardware blended linear
  values; additive draws returned `linear(c·a)/a` (exact over black) and
  alpha-blended draws were off where `0 < a < 1` (clouds 1-3 bytes; the
  selection outline 147 where GL writes 111; the sun's halo nearly
  invisible by day). That path is still in the shader for a camera without
  `CompositingSpace::Srgb` (`gl_material` module docs).
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
- `.mus` records are decoded (`sound::mus`) before kira sees them. Short
  sounds are decoded once and cached as `bevy_kira_audio` sources; music and
  records are read on the async task pool and then *streamed*, as paulscode
  streamed them, on a second kira manager (`sound::StreamOutput`, opened on
  the first music or record play), since `bevy_kira_audio` only plays fully
  decoded sounds.
- Within a frame the shell's `ClientEvent`s, `ClientSound`s and the
  simulation's `WorldEventMsg`s are handled as three streams in that order,
  not interleaved in call order, and `RenderGlobal.playSound`'s range check
  uses the view entity's position at the end of the frame's ticks. This only
  matters in a frame that runs several ticks: it can change which
  clock-seeded variant a pool picks, or tip a sound right at the 16-block
  edge.
- `MC_SOUND_LOG=1` logs every decision (`sound: <name> -> <file> vol .. pitch
  .. range .. at (x, y, z)` or `fx`), misses (`sound: no such sound: ..`),
  out-of-range skips, music and record starts and stops.
- Without an audio device kira cannot start; after a few seconds of plays
  that never leave the queue the plugin logs one warning and keeps deciding
  (and logging) without output.

## Terrain (WP-3.3a)

`terrain::TerrainPlugin` (added by `ClientPlugin`) draws the world's blocks.
Module docs: `terrain/mod.rs` (the frame, the entities), `render_blocks.rs`
(the shared state and the dispatch), `world_renderer.rs` (grid, rebuild
policy), `snapshot.rs` (what a mesh job reads).

- **Sections.** `RenderGlobal`'s grid of 16×16×16 `WorldRenderer`s
  (`64 << (3 - renderDistance)` blocks wide, at most 400: 26, 17, 9 or 5
  sections across, 8 tall), wrapped around the view as
  `markRenderersForNewPosition` does. A section with geometry in a pass is
  one Bevy entity (`terrain::SectionMesh`) with a `Mesh3d` in the section's
  coordinates and `terrain::material::TerrainMaterial`: pass 0 in the opaque
  phase (alpha test `> 0.1`, back faces culled), pass 1 blended in the
  transparent phase at `terrain::order::TRANSLUCENT`, preceded with fancy
  graphics by a depth-only draw (`TRANSLUCENT_DEPTH`), as `renderWorld`
  does (EntityRenderer.java:494-545). Bevy culls the entities; vanilla's
  own frustum flag (`clipRenderersByFrustrum`, transcribed with
  `ClippingHelperImpl`'s planes) hides the sections vanilla would skip and
  drives the rebuild priority and the F3 counts.
- **Rebuilds.** Marks come from `WorldEventMsg`
  (`MarkBlockAndNeighborsNeedsUpdate`, `MarkBlockRangeNeedsUpdate`,
  `UpdateAllRenderers`), the grid moving with the view, `loadRenderers`
  (world change, render distance, the options that ask for it). Each frame
  `updateRenderers` runs as vanilla runs it: every dirty section within 16
  blocks (nearest first) plus one more (the best by `RenderSorter`), once
  per frame without a frame-rate limit, repeated until the limit's time is
  up with one (1/120 or 1/40 s from `renderEndNanoTime`, taken at the start
  of the frame: `terrain::mark_render_end` docs). The near ones are meshed at once, the far one on Bevy's
  `AsyncComputeTaskPool`. No separate chunk load/unload trigger: as
  `ChunkCache` does, a rebuild loads (or generates) the 3×3 chunks around
  its section first, and blocks a populator writes into a neighbour arrive
  as marks.
- **Snapshot** (`terrain::snapshot::ChunkSnapshot`, built on the main
  thread): ids, metadata, sky and block light of the section grown by 3
  blocks, `skylightSubtracted`, the brightness table, the `Block`
  singletons' bounds and leaves state (`BlockStatics`, a copy the job
  mutates), the `getBlockTexture` and `setBlockBoundsBasedOnState` results
  of the kinds that read the world (computed by `mc-sim`), the special-render
  tile entities (found through `Chunk.getChunkBlockTileEntity`, which
  creates a container's missing tile entity and drops an invalid one, as
  vanilla's renderer does), a copy of the `WorldChunkManager` (biome colours) and the
  `Colorizers`.
- **Colours.** Vertex colours are the Tessellator's bytes; brightness is
  `lightBrightnessTable[max(sky - skylightSubtracted, block, lightValue)]`
  (`ChunkCache.getBrightness`), the AO corners averaged in the Java's float
  order. With AO on and fast graphics pass 1 is drawn flat-shaded (GL's
  provoking vertex), as `renderWorld` leaves `GL_FLAT` set for it.
- **Render types.** `RenderBlocks::render_block_by_render_type` dispatches
  on `getRenderType()` (RenderBlocks.java:92-132); each arm is a method in
  the file of the WP that owns it. Until filled, an arm draws nothing and
  logs `terrain: render type N (…) is not drawn yet` once.

  | Type | Java | Blocks | Owner, file |
  |---|---|---|---|
  | 0 | `renderStandardBlock` (+ AO, colour multiplier, the six faces) | cubes and cuboids (stone, dirt, grass with its side overlay, logs, leaves, glass, slabs, ice, …) | 3.3a, `render_blocks.rs` |
  | 1 | `renderBlockReed` (`renderCrossedSquares`) | saplings, flowers, mushrooms, reeds, tall grass, dead bush, web | 3.3c, `plants.rs` |
  | 2 | `renderBlockTorch` (`renderTorchAtAngle`) | torches | 3.3c, `plants.rs` |
  | 3 | `renderBlockFire` | fire | 3.3c, `plants.rs` |
  | 4 | `renderBlockFluids` (`func_1224_a`; `BlockFluid.getFlowDirection` on the snapshot) | water (pass 1, not culled: seen from below too), lava (pass 0) | 3.3b, `fluids.rs` (done; no `renderBlockOnInventory` case in vanilla) |
  | 5 | `renderBlockRedstoneWire` | redstone dust | 3.3d, `special.rs` |
  | 6 | `renderBlockCrops` (`func_1245_b`) | wheat | 3.3c, `plants.rs` |
  | 7 | `renderBlockDoor` | doors | 3.3d, `special.rs` |
  | 8 | `renderBlockLadder` | ladder | 3.3d, `special.rs` |
  | 9 | `renderBlockMinecartTrack` | rails | 3.3d, `special.rs` |
  | 10 | `renderBlockStairs` | stairs | 3.3d, `special.rs` |
  | 11 | `renderBlockFence` | fence | 3.3d, `special.rs` |
  | 12 | `renderBlockLever` | lever | 3.3d, `special.rs` |
  | 13 | `renderBlockCactus` (`func_1230_b`) | cactus | 3.3c, `plants.rs` |
  | 14 | `renderBlockBed` | bed | 3.3d, `special.rs` |
  | 15 | `renderBlockRepeater` | repeater | 3.3d, `special.rs` |
  | 16 | `func_31074_b(…, false)` | piston base | 3.3d, `special.rs` |
  | 17 | `func_31080_c(…, true)` | piston extension | 3.3d, `special.rs` |

  To fill an arm, replace the stub's body in your file (same signature);
  the shared state is `RenderBlocks`' fields (`override_block_texture`,
  `flip_texture`, `render_all_faces`, `uv_rotate_*`, `enable_ao`, the AO
  values), the `Block` methods are `RenderBlocks::{brightness,
  block_texture, should_side, set_block_bounds}` and
  `terrain::blocks::*` on the snapshot, and the Tessellator is
  `RenderBlocks::tess`. A renderer must not read more than 3 blocks from the
  block it draws (the snapshot logs a read outside it once).
- **Special render types (WP-3.3d, `terrain/special.rs`).** Types 5, 7-12
  and 14-17 draw as vanilla's: the bounds `setBlockBoundsBasedOnState`
  sets (doors, beds, rails, levers, pistons, stairs) come from the
  snapshot; the stairs, fence, lever base, repeater slab and piston faces
  go through `render_standard_block` with changed bounds (AO applies to
  them as in vanilla); the wire's connections use
  `special::is_power_provider_or_wire` on the snapshot (a world-free
  `canProvidePower`). For WP-3.5b: `func_31078_d` (the extended base with
  every face) and `func_31079_a(…, full_rod)` (the head, `false` = the
  moving piston's half rod) are `TileEntityRendererPiston`'s entry points.
  The repeater's torches go through 3.3c's `render_torch_at_angle`. Every
  other block vanilla draws in the world (portal, snow layer, pressure
  plates, buttons, farmland, cake, trapdoor, slabs, glass) is type 0;
  signs and the moving piston are type -1 (tile-entity renderers).
- **Crack overlay** (`drawBlockBreaking`): the mouse-over block drawn again
  with tile `240 + (int)(damagePartialTime * 10)`, colours off, `glColor(1,
  1, 1, 0.5)`, blended `(DST_COLOR, SRC_COLOR)` with a polygon offset,
  before the selection box (`terrain::order::{CRACK, CRACK_IN_WATER}`).
- **For WP-3.5b:** `terrain::TerrainTileEntities` is `RenderGlobal.tileEntities`
  (positions of the sign, mob spawner and moving-piston tile entities of
  the built sections) with `renderEntitiesStartupCounter`; `renderEntities`
  runs between the two passes (bands in the README's "Draw order").
- **For WP-3.7b:** `terrain::render_blocks::render_block_as_item(statics,
  colorizers, id, meta, brightness)` is `new RenderBlocks().renderBlockOnInventory`
  for render types 0 and 16 (and 10, 11, 13 built from faces; 1, 2, 6 call
  the 3.3c stubs): the quads around the origin with per-face normals, and
  `glColor` from `getRenderColor(meta) × brightness`. The normals are the
  Tessellator's bytes with its overflow (the +x face has the −x normal;
  `terrain::tessellator` module docs), and the two vertices
  `convertQuadsToTriangles` copies carry a stale normal in vanilla, which
  the shared-vertex mesh does not reproduce (it matters only with smooth
  shading under item lighting).
- **Stats:** `terrain::TerrainStats` (rebuilds this frame and in total,
  jobs in flight, sections drawn per pass, the update list's length, the
  snapshot and meshing time) and the F3 line `RenderGlobal.getDebugInfoRenders`
  in `DebugSources::renders`.
- Not done: occlusion queries (`advancedOpengl`; wgpu has none, so
  `occlusionEnabled` is false as on a GL without `ARB_occlusion_query`, F3
  `O: 0`); `RenderList`'s 1024-block anchoring: the section
  transforms are world-space `f32` and Bevy's `clip_from_world` is too, so
  far from the origin vertices lose precision where vanilla's do not (at
  about 1,000,000 blocks the `f32` step is 1/16 block: jitter); a floating
  origin would be a camera-wide change (WP-3.1). b1.7.3 has no
  `bytesDrawn` statistic. Pass 1's sections are sorted by Bevy (back to
  front by entity); vanilla calls them nearest first (`sortedWorldRenderers`),
  which shows only with fast graphics, where pass 1 writes depth while
  blending (overlapping water of two sections; WP-3.3b). *WP-4.9:* with
  fast graphics each pass-1 section now has its own material whose sort
  bias is its place in vanilla's call order
  (`RenderGlobalTerrain::render_list_order`: nearest first, grouped by
  `RenderList` anchor), so the nearer section is drawn first as in
  vanilla; the floating origin is designed in `docs/PERF.md` §7, not
  built. `Chunk.isLit`
  is a static that vanilla clears before `new ChunkCache(…)`, so chunks
  that cache loads or populates can set it (population reads
  `Chunk.getBlockLightValue`, e.g. `BlockFlower.canBlockStay`,
  Chunk.java:341-345, WorldRenderer.java:93-101): such a section is marked
  `isChunkLit` even if its own blocks read no sky light. Here the flag is
  the snapshot's own and covers only the drawing reads (mc-sim keeps no
  `isLit`), so after a sky-light change `updateAllRenderers` can rebuild
  fewer sections than vanilla; nothing drawn differs.

### Plants, torches, fire, crops, cactus (WP-3.3c)

`terrain/plants.rs` fills render types 1, 2, 3, 6 and 13 (module docs list
the blocks and textures):

- **1** `renderBlockReed` → `renderCrossedSquares`: two diagonal planes
  0.45 around the centre, each emitted with both windings (vanilla culls
  back faces), colour = brightness × `colorMultiplier`. Tall grass (31) is
  drawn at an offset hashed from the position (`plants::tall_grass_offset`,
  ±0.25 in x and z, −0.2..0 in y; RenderBlocks.java:1327-1333), and tinted
  by the biome at a second hashed offset (`blocks::color_multiplier`).
- **2** `renderBlockTorch` → `renderTorchAtAngle`: metadata 1-4 lean the
  torch off its wall (0.1 toward it, 0.2 up, 0.4 per block of tilt), other
  values stand. A block with a light value (torch, lit redstone torch) is
  drawn at brightness 1, so it stays bright at night.
- **3** `renderBlockFire`: over a normal cube or a flammable block (the
  `chanceToEncourageFire` table, `plants::can_block_catch_fire`), four
  slanted planes 1.4 high; otherwise one leaning flame per flammable side
  neighbour and two hanging planes under a flammable block. The first and
  second animation tiles (31, 47) and the mirrored u alternate with the
  position's parity.
- **6** `renderBlockCrops` → `func_1245_b`: four planes in a `#` 1/16
  below the cell, texture `88 + meta`.
- **13** `renderBlockCactus` → `func_1230_b`: the plain-shaded faces with
  the four sides drawn 1/16 inside the cell.

`renderBlockOnInventory` already calls the type 1, 2 and 6 helpers at
(−0.5, −0.5, −0.5) (WP-3.7b's `render_block_as_item`); the helpers set no
colour, as the Java leaves it to `glColor`.

Smoke-test tip: `set_pos` with `y` exactly `ground + 1.62` leaves the
player one block down: `yOffset` is the float `1.62F` (1.6200000047683716),
so the box's bottom lands just below the ground's top, inside the ground
block, which the collision then ignores. Add 0.01.

## Models (WP-3.5a)

`model` is the `Model*` family as data (PLAN_v4 D1 rule 6, D7): every model
the renderers construct is a `model::ModelKind` (one variant per
constructor call: `Biped` = `new ModelBiped()`, `BipedArmor` =
`ModelBiped(0.5F)`, `BipedArmorChestplate` = `ModelBiped(1.0F)`, `Zombie`,
`Skeleton`, `Pig`, `PigSaddle` = `ModelPig(0.5F)`, `Cow`, `Sheep` =
`ModelSheep2`, `SheepFur` = `ModelSheep1`, `Chicken`, `Wolf`, `Creeper`,
`CreeperArmor` = `ModelCreeper(2.0F)`, `Spider`, `Slime` = `ModelSlime(16)`,
`SlimeOuter` = `ModelSlime(0)`, `Ghast`, `Squid`, `Minecart`, `Boat`), and a
`model::Model` holds its parts (`ModelPart` = `ModelRenderer`, in the Java's
field order; index constants per model module, e.g. `model::biped::RIGHT_ARM`)
plus `ModelBase`'s/`ModelBiped`'s state (`on_ground` = the swing progress,
`is_riding`, `held_item_left/right`, `is_sneak`). `ModelBed` is three tables
(`model::bed`) for the bed block (WP-3.3).

All b1.7.3 models texture over 64×32 (`TexturedQuad` hard-codes it; there is
no `textureWidth`/`textureHeight` yet); UVs are already divided, with
vanilla's inset of a tenth of a texel (`0.0015625` = 0.1/64, `0.003125` =
0.1/32).

### Drawing a model (for WP-3.5b)

Recommended: **an entity with one child entity per part** (the Java's
display list per `ModelRenderer` is a Bevy mesh built once and shared by
every entity of that kind; only transforms change per frame):

1. Keep one `Model` per renderer, as `RenderManager` keeps one model per
   entity class (state the Java does not reset leaks between entities the
   same way: the wolf's rotation points, the minecart's inner floor, the
   armour models' `showModel`).
2. Spawn: `model::spawn_model(&mut commands, &mut ModelMeshes, &mut
   Assets<Mesh>, kind, material, root_transform)` → the root entity
   (`ModelEntity { kind, parts }`), with one hidden child per part
   (`ModelPartEntity`, `Mesh3d` from the `ModelMeshes` cache, keyed by kind).
3. Each frame, as `RenderLiving.doRenderLiving` does: set
   `model.on_ground = getSwingProgress(partial)` (`func_167_c`),
   `is_riding`, and for bipeds `held_item_right`/`is_sneak`
   (`RenderPlayer`, `RenderBiped`); call `model.set_living_animations(&
   LivingInputs::Wolf(WolfAnimInputs::from_entity(e, partial,
   brightness)), limbSwing, limbSwingAmount, partial)` for wolves (it
   returns the shaking wolf's `glColor3f`); then
   `let draws = model.render(limbSwing, limbSwingAmount, ticks, headYaw,
   headPitch, MODEL_SCALE)` (`ticks` = `func_170_d`: age + partial, the
   chicken's wing angle, the wolf's tail angle, the squid's tentacle angle)
   and `model::apply_pose(&model_entity, &draws, &mut part_query)`, which
   places and shows the drawn parts and hides the others.
4. The root `Transform` is the renderer's matrix (RenderLiving.java:36-43):
   `translate(entity) · rotY(180 - renderYawOffset) · [death: rotZ] ·
   scale(-1, -1, 1) · preRenderCallback scale · translate(0, -24 × 0.0625 -
   0.0078125, 0)` (`Transform::from_matrix` handles it; `scale(-1, -1, 1)` is
   a rotation). Minecarts and boats: their own `translate · rotY · rotZ ·
   … · scale(-1, -1, 1)` with no −1.5 offset. `model::debug::root_transform`
   is a reduced example. Part transforms are `ModelPart::transform(mode,
   scale)` (translate to the rotation point × 0.0625, then rotate Z, Y, X for
   `render`/`postRender`, Y, X, Z for `renderWithRotation`, which the wolf's
   head, ears, snout and tail use); `model::part_transform(part)` is the
   common case.
5. Held items and the helmet block hang off `model.post_render(part,
   MODEL_SCALE)` (`bipedRightArm.postRender`, `bipedHead.postRender`) as a
   child of the root. `None` (the part hidden) means the identity: Java's
   `postRender` then leaves the matrix alone and the item is still drawn
   at the root's origin, so do not skip it. `render_ears` / `render_cloak` give the extra parts'
   draws for `RenderPlayer` (skins and cloaks are downloads: deferred, D2).
6. Hide (or despawn) model entities while no world is drawn (`FogState::
   active`): Bevy re-specialises visible meshes' pipelines when the world
   camera's `DistanceFog` goes away, and a pipeline compile still running at
   exit crashes the NVIDIA driver at teardown (seen with `debug_model`).

Drawing through `TessBatch` instead (re-tessellating every part of every
entity per frame) is not recommended for models; it suits the one-off quads
of `RenderArrow`, `RenderPainting`, `RenderSnowball`, `RenderFish` &c.

### The material

`model::ModelMaterial` is `GlMaterial`'s arithmetic (same colour
conversion and sRGB caveats) plus GL lighting:
`RenderHelper.enableStandardItemLighting` is on for every entity draw
(EntityRenderer.java:500-503), so a face shows `clamp(c × 0.4 + c × 0.6 ×
Σ max(n·Lᵢ, 0))` of `glColor` `c`, the two lights fixed in the world
(`model::material::standard_light_directions`). Normals come from
`Tessellator.setNormal`'s byte packing, **vanilla bug kept**: `x × 128` wraps
`+1` to `-128` and the sign extension spills into y and z, so both x faces
of every box are lit as `-x` (`model::part::tess_normal`).

- Main pass: `ModelMaterial::living(texture, brightness)` (`glColor3f(
  getEntityBrightness)`, RenderManager.java:107-108; alpha test 0.1; cull
  off; opaque phase), `.with_fog(&FogState)`, and `set_fog` when the fog
  changes. Textures: `TextureLoader::get(path)` (the shared cache's sRGB
  images; `texture_srgb` makes the shader read the PNG's values).
- Passes (`ModelKey`, `ModelParams`): hurt/death red and the creeper flash
  (`textured: 0`, alpha test off, `GlBlend::Alpha`, `depth_equal: true`,
  `color` (b, 0, 0, 0.4) or the multiplier's ARGB); the powered creeper
  (`CreeperArmor`, `/armor/power.png`, `lighting: 0`, `GlBlend::Additive`
  with `key.src_one = true` (`glBlendFunc(GL_ONE, GL_ONE)`,
  RenderCreeper.java:67; plain `GlBlend::Additive` is `GL_SRC_ALPHA,
  GL_ONE` and would weight power.png's texels by their alpha), the alpha
  test kept at 0.1 (the pass leaves `GL_ALPHA_TEST` on), `color` 0.5,
  `uv_offset` = `(t × 0.01, t × 0.01)`; `debug_model`'s `additive` is this
  state); spider eyes (`GlBlend::Alpha`, alpha test off, `color`
  (1, 1, 1, (1 − brightness) × 0.5)); the slime's outer cube
  (`GlBlend::Alpha`); sheep wool (`color` = brightness × fleece colour).
  Blended passes go to `Transparent3d`: give them a `sort_bias` in the
  entity band (−0.95e6 … −0.91e6, `gl_material::order`). Minecarts and
  boats keep `GL_CULL_FACE`: `key.cull = true`.

### Textures per model (the renderers' `loadTexture` / `getEntityTexture`)

| Entity | Renderer | Models | Texture |
|---|---|---|---|
| Player | `RenderPlayer` | `Biped`; armour `BipedArmorChestplate` (helmet, chestplate, boots), `BipedArmor` (leggings) | `/mob/char.png` (EntityPlayer.java:51; skin downloads deferred); `/armor/{cloth,chain,iron,diamond,gold}_{1,2}.png`, `_2` for leggings (RenderPlayer.java:25) |
| Zombie | `RenderBiped` | `Zombie` | `/mob/zombie.png` |
| Zombie pigman | `RenderBiped` (the zombie's) | `Zombie` | `/mob/pigzombie.png` (EntityPigZombie.java:12) |
| Giant | `RenderGiantZombie` (scale 6) | `Zombie` | `/mob/zombie.png` |
| Skeleton | `RenderBiped` | `Skeleton` | `/mob/skeleton.png` |
| Creeper | `RenderCreeper` | `Creeper`; powered pass `CreeperArmor` | `/mob/creeper.png`; `/armor/power.png` |
| Spider | `RenderSpider` | `Spider` twice | `/mob/spider.png`; eyes pass `/mob/spider_eyes.png` |
| Pig | `RenderPig` | `Pig`; saddle pass `PigSaddle` | `/mob/pig.png`; `/mob/saddle.png` |
| Sheep | `RenderSheep` | `Sheep`; wool pass `SheepFur` | `/mob/sheep.png`; `/mob/sheep_fur.png` |
| Cow | `RenderCow` | `Cow` | `/mob/cow.png` |
| Chicken | `RenderChicken` | `Chicken` | `/mob/chicken.png` |
| Wolf | `RenderWolf` | `Wolf` | `/mob/wolf.png`, `/mob/wolf_tame.png` (tamed), `/mob/wolf_angry.png` (angry) (EntityWolf.java:34-39) |
| Slime | `RenderSlime` | `Slime`; outer pass `SlimeOuter` | `/mob/slime.png` |
| Ghast | `RenderGhast` | `Ghast` | `/mob/ghast.png`, `/mob/ghast_fire.png` while attacking (EntityGhast.java:28) |
| Squid | `RenderSquid` | `Squid` | `/mob/squid.png` |
| Other `EntityLiving` | `RenderLiving` | `Biped` | `/mob/char.png` |
| Minecart | `RenderMinecart` | `Minecart` | `/item/cart.png` (the chest/furnace block from `/terrain.png`) |
| Boat | `RenderBoat` | `Boat` | `/item/boat.png` |

### Checked

`model/testdata/java_models.txt` is the vanilla classes run on the oracle
JVM (`model/testdata/ModelDump.java`, regenerated as its header says):
every part's rotation point, angles, mirror flag, corners and faces, each
face's `TexturedQuad.draw` normal and the int the real
`Tessellator.setNormal` packs from it, and the parts after a sequence of
`setRotationAngles` (and the wolf's `setLivingAnimations`, shaking
included, the ears) calls; the tests compare them bit for bit. The
`render` draw lists (they need GL) are checked against the Java methods'
call sequences written out by hand.

## GUI (WP-3.6a)

### Spike: a 2D batcher, not `bevy_ui`

PLAN_v4 §3.5's default is confirmed. What was weighed (against the
`bevy_ui` 0.19 sources; no `bevy_ui` prototype was built):

- Vanilla's screens are immediate-mode: every frame `drawScreen` issues
  `drawTexturedModalRect`/`drawRect`/`drawGradientRect`/`drawString` calls
  in scaled pixels, interleaved with raw GL state changes (`glEnable`,
  `glBlendFunc`, `glColor`, `glTranslatef`, `glDisable(GL_DEPTH_TEST)`) and
  `Tessellator` quads with per-vertex colours. `bevy_ui` is a retained node
  tree laid out by flexbox: a line-for-line transcription would need a
  per-frame diff of draw calls into nodes, and the state-dependent parts
  have no node equivalent.
- `bevy_ui` blends every node with one fixed `ALPHA_BLENDING` state
  (`bevy_ui_render` pipelines); `GuiIngame`'s crosshair needs
  `glBlendFunc(775, 769)` (colour inversion), the overlays need
  `GL_ALPHA_TEST`, flat versus smooth shading, the depth test against
  `zLevel` (items at z 100+ in containers).
- Text: `bevy_text` lays out TTF/OTF fonts; the Beta font is an 8×8 bitmap
  whose advances come from pixel columns, with `§` colour codes and a
  shadow pass. It would be per-glyph image nodes either way.
- Gradients (`BackgroundGradient`) interpolate in a chosen colour space,
  not GL's Gouraud on the Java values; 3D items in inventory slots
  (WP-3.6b/3.7) need a 3D pass anyway.

So the GUI is `GuiGl` (`gui::batch`): the fixed-function state and the
`Tessellator` as data. `draw()` records the quads under the current state
(vertex colour = the Tessellator's or `glColor`, flat shading = the quad's
last vertex, the matrix applied); consecutive draws under one state merge,
so a screen is a handful of batches (one per texture or state switch) in
call order. `Gui*` code transcribes nearly line for line (`gui::draw` is
`Gui`). The batches become one mesh each with `gui::render::GuiMaterial`
(WP-3.4's `gl_material.wgsl`, any `glBlendFunc` pair, GL's depth test and
mask, the alpha test), drawn by two orthographic `Camera3d`s after the
world camera (order −1) and the first-person hand camera (order 0,
WP-3.7b): order 1 is the in-game overlay, order 2 the screen (each
clears the depth buffer as vanilla does before each). The projection is
`setupOverlayRendering`'s `glOrtho(0, scaledWidthD, scaledHeightD, 0,
1000, 3000)`: one scaled pixel is `scaleFactor` window pixels.

Blending: opaque pixels are vanilla's bytes, and since WP-3.7b (every
camera composites in sRGB, see "Compositing in sRGB") blends with
`0 < alpha < 1` are GL's byte-space blends: `drawDefaultBackground` over
the noon sky `(145, 163, 255)` gives `(48, 52, 75)`, GL's arithmetic.
History: on the earlier sRGB target these blends happened in linear space,
so the dark gradient dimmed the world less than vanilla (sky
`(42, 61, 183)` became `(25, 33, 97)` where GL writes `(22, 27, 57)`) and
`drawRect` translucency and the list shadows were lighter. The three cameras share one main texture, but Bevy's
upscale pass by default alpha-blends each later camera's copy of it over
the window by that texture's alpha, which the world's clouds and sunrise
band leave below 1: camera 0's world-only frame then showed through every
GUI pixel there (the text field's black came out `(67, 73, 90)` under a
cloud). The GUI cameras therefore write with `CameraOutputMode::Write {
blend_state: Some(BlendState::REPLACE), .. }`, and GUI draws keep the
destination alpha anyway.

### For WP-3.6b/c

- A screen implements `gui::screen::Screen` with a `ScreenBase`
  (`width`, `height`, `control_list`, `allow_user_input`, `z_level`). Java's
  `super.drawScreen/keyTyped/mouseClicked/mouseMovedOrUp` are
  `default_draw_screen`, `default_key_typed`, `default_mouse_clicked`,
  `default_mouse_moved_or_up`; `drawDefaultBackground` and friends are
  in `gui::screen`.
- `this.mc` is `ScreenCtx`: `ctx.game` (`Minecraft`), `ctx.font`,
  `ctx.lang`, `ctx.display_gui_screen(Some(Box::new(..)))` (at once, as
  Java), `ctx.display_this()` (`displayGuiScreen(this)`),
  `ctx.display_wrapping_this(|this| Box::new(GuiYesNo::new(this, ..)))`
  (`new GuiYesNo(this, ..)`), `ctx.play_sound_fx`, `ctx.create_screen(kind)`
  (`new GuiMainMenu()` and other registered classes),
  `ctx.get_clipboard_string()`, `ctx.enable_repeat_events(on)`
  (`Keyboard.enableRepeatEvents`, for GuiChat, GuiCreateWorld,
  GuiRenameWorld, GuiEditSign: on in `initGui`, off in `onGuiClosed`).
- The shell opens some screens by kind (`GuiMainMenu`, `GuiIngameMenu`,
  `GuiGameOver`, `GuiInventory`, `GuiConflictWarning`); register a factory
  for the class name with `ScreenRegistry::register` (a resource) and the
  binder makes the object. Unregistered kinds get a `ShellScreen` (the
  background and the keys the shell gave them before).
- Screen objects are bound to the slot by `Game::screen_serial`; the
  binder catches up after each `display_gui_screen` and at the start of
  the screen hook and of the drawing. A screen that replaces itself gets
  `onGuiClosed` when its method returns; the rest of a `handleInput`'s
  events still go to it (Java's `this`).
- The in-game overlay draws in a system in `GuiSet::Overlay` (layer
  `GuiLayer::Overlay`, the default at frame start). Textures: bind a jar
  name (`g.bind_texture("/gui/icons.png")`) or a `Handle<Image>` from
  `Textures`; the GUI samples an `Rgba8Unorm` copy that follows the engine
  image (so the animated atlases work for item icons).
- Keys: Ctrl+letter reaches `keyTyped` as the control character (LWJGL's
  22 for Ctrl+V). The workspace enables Bevy's `system_clipboard`
  feature (arboard, WP-3.6b), so `getClipboardString` reads the system
  clipboard.

## In-game GUI (WP-3.6b)

- **HUD:** `gui::ingame::GuiIngame` (a resource, `Minecraft.ingameGUI`)
  draws in `GuiSet::Overlay` when vanilla's `renderGameOverlay` runs (a
  world drawn, GUI shown or a screen open): vignette (fancy), pumpkin blur,
  portal overlay, hotbar with its items, crosshair (`glBlendFunc(775,
  769)`), armour/hearts/air (flash, jitter from `updateCounter * 312871`),
  sleep fade, F3 text (`DebugText`), the record line (`RecordPlayingMessage`)
  and the chat (last 10 lines, 20 with `GuiChat` open; 200-tick fade).
  `updateTick` runs in `RunTickHook::IngameGuiUpdateTick`. The F3
  frame-time graph (`displayDebugInfo`) and the achievement popup
  (`gui::achievement::GuiAchievement`) draw after the screen
  (`GuiSet::Screen`), as vanilla draws them after `updateCameraAndRender`.
- **Messages:** `gui::ingame::collect_gui_messages` runs in every tick hook
  (before `screen_input`) and after the frame: `ClientEvent::ChatMessage` /
  `ChatMessageTranslate` / `ClearChat` (chat), `AchievementHint` (the
  popup's hint), `DisplayGui` (queued in `container::PendingPlayerGuis` and
  opened by `open_pending_player_guis` with a `ScreenCtx`). A click that
  opens a chest therefore opens it in the same tick's next hook
  (`UpdateRenderer`), not inside the click: the rest of that tick's input
  events still go to the game. Unlocked achievements come from the player's
  `StatFileWriter::taken_achievements`, drained every frame.
- **Items in the GUI:** `gui::render_item` (`renderItemIntoGUI`,
  `renderItemOverlayIntoGUI`). `GuiGl` has no lighting, normals or face
  culling, so `render_item::ItemGl` mirrors the modelview matrix (every
  matrix call around an item goes through it), keeps the two lights of the
  last `enableStandardItemLighting` (transformed by the matrix of that
  moment), and lights (`0.4 c + Σ 0.6 c max(n·L, 0)`, `GL_RESCALE_NORMAL`)
  and culls (`GL_CULL_FACE`, back faces) each quad on the CPU before it
  reaches the batcher. Block items come from
  `terrain::render_blocks::render_block_as_item` with the world's
  `block_statics` (stale singleton bounds show, as in vanilla).
  `draw_quads` / `draw_tessellator` serve any lit 3D draw in the GUI.
- **Screens:** registered by class name: `GuiInventory`, `GuiGameOver`,
  `GuiChat`, `GuiSleepMP` (`gui::container::register`). `GuiChest`,
  `GuiFurnace`, `GuiCrafting`, `GuiDispenser`, `GuiEditSign` are built from
  `PlayerGui` (`gui::container::open_player_gui`). A `GuiContainer` owns its
  `mc_sim` `Container` (the simulation keeps no `inventorySlots` /
  `craftingInventory`): clicks are `Container::slot_click` with the world
  and the player, closing runs `on_craft_gui_closed`, and the
  `isUsableByPlayer` check of `EntityPlayer.onUpdate` runs in
  `updateScreen`. Not openable: the chest minecart (`InventoryRef` has no
  entity variant), a chest/furnace/dispenser request without its tile
  entity (vanilla would throw); both are logged.
  *WP-4.6:* the chest minecart opens now: `PlayerGui::MinecartChest`
  builds `chest::new_gui_chest` over `InventoryRef::Entity(entity_id)`
  (`mc_sim`'s `ContainerChest` over the cart's `cargoItems`), titled
  `"Minecart"` (`inv_name`, EntityMinecart.java:752-754).
  *WP-4.7:* `initGui` hands the screen's container to the player
  (`craftingInventory`, `mc_sim::entity::player::set_crafting_inventory`,
  GuiContainer.java:16-19) and the screen works on it there
  (`GuiContainer::with_container`; `GuiContainer::slots` keeps the slot
  layout for hit-testing and drawing). The `isUsableByPlayer` close runs
  inside the player's update in the simulation (EntityPlayer.java:80-84):
  the cursor stack and the grid drop there, the container leaves the
  player and `ClientRequest::CloseScreen` closes the screen after
  `updateEntities`, so the next tick's input goes to the game, as in
  vanilla; `on_gui_closed` then finds nothing to drop. `updateScreen`
  keeps only GuiContainer.java's own dead-player test.
- **Previews drawn through the batcher:** the inventory's player
  (`container::inventory::render_player_preview`: `RenderPlayer` /
  `RenderLiving` matrices, `ModelBiped`, the armour passes and
  `renderSpecials`' pumpkin helmet and held item through a batcher copy of
  `ItemRenderer.renderItem`, lit; no hurt overlay, shadow or fire) and the
  sign being edited
  (`container::edit_sign::render_tile_entity_sign_at`, `SignModel` and the
  text). WP-3.5b's renderers may replace them. *WP-4.9:* the inventory's
  player now goes through them (`render::GuiEntityRenderer` →
  `gui::entity_draws`, module docs there): `RenderPlayer` with the hurt
  and death overlay, `doRenderShadowAndFire`'s shadow and fire; the batcher
  copy of `RenderPlayer` is gone.
- **Draw order:** `gui::render` sorts batches by `GuiMaterial::sort_bias`
  = index × 4096, which outweighs the batch meshes' AABB centre depth Bevy
  adds (z −1000..1000); with the plain index the HUD's counts, drawn with
  the depth test off, went under the 3D block icons.
- **Clipboard:** the workspace enables Bevy's `system_clipboard` (`arboard`).
- **Known deviations** (each needs a change outside WP-3.6b's files):
  - *Screens opened by a click.* `ClientEvent::DisplayGui` opens its screen
    at the next tick hook, so the rest of that tick's input events still
    reach the game (a number key, Q, a second right-click), where vanilla
    routes them to the new screen (Minecraft.java:995-1126). Needs
    `Game::process_input` (WP-3.1's) to open the screen, or at least set
    `current_screen`, right after each `PlayerController` call that raises
    it.
  - *Portal with a workbench open.* `EntityPlayerSP.onLivingUpdate` calls
    `displayGuiScreen(null)` directly (EntityPlayerSP.java:45-47), skipping
    `closeScreen`, so vanilla drops the 3×3 grid twice (the second
    `EntityItem`s share the first ones' `ItemStack` objects and end up
    empty). The simulation's stacks are values: it drops once.
  - *Chest minecart.* `PlayerGui::MinecartChest` is logged and not opened:
    `InventoryRef` has no entity variant in `mc-sim`.
    Closed by WP-4.6 (`InventoryRef::Entity`, see above).
  - *Inventory preview.* No hurt/death red overlay (needs
    `glDepthFunc(GL_EQUAL)` in `GuiGl`), no shadow or fire
    (`Render.doRenderShadowAndFire`, WP-3.5b).
    Closed by WP-4.9 (`GuiGl::depth_func`, the preview through the
    renderers). Its blends inherit the GUI's (`gui::render` module docs).

## Menus (WP-3.6c)

`gui::screens_menu::MenuPlugin` (added by `ClientPlugin` after `GuiPlugin`)
registers the menus by class name (`GuiMainMenu` and `GuiIngameMenu` are
the ones the shell opens by kind; the others, e.g. `GuiOptions`,
`GuiSelectWorld`, `GuiAchievements`, `GuiStats`, `GuiImportWorld`, can be
opened with `open_screen`, without a parent), reads the title splashes
from the jar's `/title/splashes.txt`, and draws the loading screen.

- **Title** (`GuiMainMenu`): logo, splash (rotated −20°, pulsing, the
  date overrides), Singleplayer, Multiplayer (always disabled: deferred,
  D3), Mods and Texture Packs, Options..., Quit Game (shown: no applet,
  `Minecraft.hideQuitButton` false), the version and copyright lines.
- **Parents**: a screen opened from another keeps it
  (`ctx.display_wrapping_this`) and shows it again on Done/Cancel, which
  re-runs its `initGui` as Java's `displayGuiScreen(parent)` does.
- **Worlds**: `GuiSelectWorld` lists `getSaveList()` sorted latest first;
  the second line is `folder (M/d/yy h:mm a, N.NN MB)` (Java 6 `en_US`
  `SimpleDateFormat()`, local time). Play, double click, Create
  (`GuiCreateWorld`: the folder is the trimmed name with `/ \ : * ? " < > |`,
  the backquote and the control characters replaced by `_`, and `-`
  appended while a world has it; the seed is a `long`, else its
  `String.hashCode()`, random when empty or 0), Rename (the level name
  only), Delete (with `GuiYesNo`). A world needing conversion is
  listed with "Must be converted!"; opening it fails (deferred, D3).
- **Import** (D5, project-owned): the select-world screen's "Import..."
  button (top right, 70 × 20, in the title band) opens a path box (typed
  or pasted; `~/` is the home directory; Bevy has no native file dialog),
  Import and Cancel. Import runs `import::import_world` (the whole folder
  is read and checked first; a refusal copies nothing) and opens the world
  as Play does; a refusal opens a `GuiErrorScreen`-style dialog: "Block id
  N is not a Beta 1.7.3 block. First found in chunk (x, z) (…)" (or the
  item id, or the file it cannot read), Back returns to the path. The copy
  runs inside the click.
- **Export** (D5, WP-4.5, project-owned): the select-world screen's
  "Export..." button (top left, 70 × 20, mirroring Import...; enabled
  while a world is selected) opens `export_world::GuiExportWorld` for the
  selected world: a folder box (default `<data-dir>/exports`), Export and
  Cancel. Export runs `export::export_from_game` inside the click; the
  result dialog (`GuiExportResult`) says "World exported" with the path
  (wrapped at `/`; a path too long for the room above the button keeps its
  first piece, `.../` and its last pieces), the files, region files and chunks checked and the player position (Done:
  the world list), or "Export refused" / "Export failed" with the problem
  and "Nothing was exported." (Back: the export screen). See "Export" below.
- **Pause menu**: Back to game, Achievements, Statistics, Options..., Save
  and quit to title (`changeWorld1(null)` then a new title screen), the
  blinking "Saving level.." while `quickSaveWorld` has chunks left (it is
  called every drawn frame, as in vanilla).
- **Achievements**: vanilla clips the map's lines and icons to the tiled
  window with `glDepthFunc(GL_GEQUAL)` and z −200; the GUI GL has no depth
  function, and the GUI batches are ordered by Bevy's transparent sort,
  which also weighs a batch's distance (a batch at z −200 sorts before the
  background drawn at z 0). The map is therefore drawn at z 0 and the
  lines, frames and icons are cut to the tiled rectangle on the CPU
  (`gui::menu::Clip`). (`gui::render::flush_gui` orders the batches by
  `sort_bias` = index × 4096 since WP-3.6b, which outweighs the z
  differences, so screens drawing at other z keep call order; see "Draw
  order" under WP-3.6b.)
- **Item icons** (WP-3.9): the achievement map and the statistics rows
  call WP-3.6b's `gui::render_item::draw_item_into_gui_with(ig, g,
  statics, colorize, clip, id, damage, icon, x, y)`
  (`RenderItem.drawItemIntoGui`): blocks as the lit 3D cube
  (`renderItemIn3d`), other items as the flat icon. `colorize` is
  `field_27004_a` (`GuiAchievements` clears it for an achievement that
  cannot be taken yet: the icon keeps the 0.1 grey); `clip` cuts the
  drawing to the map window in GUI pixels (a 3D item's quads are clipped
  polygon by polygon, `gui::render_item` module docs, "Clipping").
  `draw_item_into_gui` (the hotbar, containers, the popup) is the same
  with `true, None`. The old flat stand-in `gui::menu::draw_item_into_gui`
  is gone.
- **Statistics**: General, Blocks and Items (disabled while empty); column
  sorting by the header icons (descending, ascending, off), the tooltips;
  values in each stat's format (`1,234`, `1.50 km`, `2.00 h`).
- **Texture packs**: the default pack only (deferred, D3); "Open texture
  pack folder" disabled.
- **Loading screen** (`gui::loading`): drawn every frame while a world
  change is pending (`Game::pending`): the dirt, the title and message
  lines, and the 100 × 2 progress bar.
- **Scripts**: key and mouse events reach the screen that is current when
  the next tick's `handleInput` begins (vanilla's `this`): wait at least a
  tick (`{"action": "wait", "ticks": 2}`) after a click that opens a screen
  before typing into it, or the keys go to the old screen. A world-list row
  is clicked with `click_slot` (WP-3.9: by index, where `GuiSlot` draws
  it; `GuiSlot` centres a short list, so the first row is not at the
  list's top), or by hand with `mouse_to` + `mouse_down`, a few frames,
  `mouse_up` (`GuiSlot` polls the held button while drawing).

## Export (WP-4.5)

`export::export_world(saves_dir, name, target)` copies `saves/<name>` to
`<target>/<name>` so that the real Beta 1.7.3 client can open it (the
native format already is McRegion, D5: nothing is converted or stripped).
In order: the target must be absent or an empty folder (never overwritten,
never inside the world); the world is read into memory and validated
(`mc_sim::save::import::validate` for `level.dat` and every block and item
id, then McRegion version 19132, `SaveOldDir.loadWorldInfo`, and every
chunk a region header lists must read back and load through
`McRegionChunkLoader`; a truncated region file, which the import's scan
lets through as vanilla would regenerate those chunks, is refused); its
digest is taken (`export::WorldDigest`: blake3 of each loaded chunk's five
arrays, the Tier S `chunk_digest` `"loaded"`, of its NBT tree, of the
`level.dat` tree and of every file); the files are written through
`std::fs` into a hidden staging folder `<target>/.mc_export-<name>-<pid>`,
read back from disk, digested again and compared (`export::compare`, the
first differing file or chunk); then the staging folder is renamed to
`<target>/<name>`. Any failure removes the staging folder and the folders
created for `<target>` (while empty).
`export::export_from_game` saves the open world first
(`World.try_save_world(true)`) when it is the one exported and flushes the
region cache. How it was checked against the vanilla client:
`docs/EXPORT.md`.

## Particles (WP-3.7a)

`particles::ParticlesPlugin` (added by `ClientPlugin`) owns
`particles::EffectRenderer` (`Minecraft.effectRenderer`, a `Resource`).

- Requests arrive as messages and are queued in `EffectRenderer::pending`:
  `WorldEventMsg::SpawnParticle` (`RenderGlobal.spawnParticle`, with its
  16-block range), `WorldEventMsg::PlayAuxSfx` 2000 (dispenser smoke) and
  2001 (block-break debris), `ClientEvent::BlockHitEffects`,
  `ClientEvent::PickupFx`, `ClientEvent::WorldChanged` (`clearEffects`).
  They are applied at the next `RunTickHook::UpdateRenderer` /
  `UpdateEffects`, so every particle of a tick exists before that tick's
  `updateEffects` (a world change clears at once). Other parts may also
  call `EffectRenderer::add_effect`, `spawn_particle`,
  `add_block_hit_effects`, `add_block_destroy_effects` directly.
- `RunTickHook::UpdateRenderer`: `addRainParticles` (splashes and smoke on
  lava around the viewer while it rains; the rain sound is queued on the
  world as `World.playSoundEffect`, WP-3.8 plays it). It reseeds WP-3.4's
  `weather::WeatherRandom` (`EntityRenderer.random`).
- `RunTickHook::UpdateEffects`: `updateEffects` (each particle's
  `onUpdate`; physics through the simulation's collision boxes, read-only
  apart from the `random.fizz` sound event a burning, rained-on particle
  queues).
- Drawing (after `GameFrameSet`): one `GlMaterial` mesh per layer (0
  `/particles.png`, 1 `/terrain.png`, 2 `/gui/items.png`; alpha test 0.1,
  no blending, depth writes, `setupFog(0)`), plus the footsteps (layer 3,
  `/misc/footprint.png`, blended). Sort biases `particles::render::order`
  (between the entities and the translucent terrain). The layers sample
  `Rgba8Unorm` mirrors of the `Textures` CPU copies (`GlMaterial` wants
  Unorm, `Textures` uploads sRGB).
- For WP-3.5b: `particles::PickupFxFrame` lists the `EntityPickupFX`s of the
  frame (position relative to `ParticlesFrame::anchor`, yaw, partial tick,
  light) for `RenderManager.renderEntityWithPosYaw`; the particle module
  draws no entity.
- F3's `P:` count is written into `DebugSources::particles` every frame,
  inside `GameFrameSet` after `game_frame` and before
  `debug::publish_debug_text` (the HUD reads this frame's count).
- The digging particles' tint (`applyColourMultiplier`) uses the client's
  `textures::Colorizers` (the simulation's colour map lookup answers 0)
  and, for the 2001 debris, the metadata in the event (the block is air by
  the time the request is applied).
- No simulation `Random` is drawn: the particles' `rand` (one clock-seeded
  `JavaRandom` per particle, as `new Random()`), `Math.random()` (a client
  stream) and `playAuxSFX` 2000's `worldObj.rand` (a stand-in, as in the
  sound half) are the client's own; particles take no entity id.

## Item renderer and the hand (WP-3.7b)

`item_renderer::ItemRendererPlugin` (added by `ClientPlugin`) owns
`item_renderer::ItemRenderer` (`entityRenderer.itemRenderer`'s fields).

- **Tick:** `updateEquippedItem` runs in `RunTickHook::UpdateRenderer`,
  after the `ClientEvent::ResetEquippedProgress` events flushed before it
  (`resetEquippedProgress(2)` from `clickMouse`). The equip animation is
  `equippedProgress` moving 0.4 a tick toward 1 (same item) or 0 (a new
  one; the new stack is taken once it is under 0.1).
- **`itemToRender` identity:** vanilla compares the hotbar's `ItemStack`
  object by identity; stacks are values here, so it is approximated
  (`item_renderer::same_object`: same slot as when taken, same id, same
  damage unless the item is damageable). A stack moved between slots by
  the inventory screen counts as a new object.
- **The hand pass:** `renderWorld` clears the depth buffer and calls
  `renderHand` last. Here that is a second `Camera3d`
  (`item_renderer::HandCamera`, order 0; the world camera is order −1
  (`camera::WORLD_CAMERA_ORDER`), the GUI cameras 1 and 2), depth cleared,
  colour loaded, output written with `BlendState::REPLACE` like the GUI
  cameras, render layer `item_renderer::HAND_RENDER_LAYER` (29). Its view is
  the identity and its projection the world's `McProjection` (FOV modifier
  included: 60° under water) without the portal distortion, which vanilla
  applies to the world's modelview only. Mesh transforms are the Java's
  eye-space matrices: `identity · hurtCameraEffect · setupViewBobbing`
  (`camera::hurt_camera_effect`, `camera::setup_view_bobbing`) then
  `renderItemInFirstPerson`'s calls.
- **What is drawn:** the empty hand (the right arm of `ModelKind::Biped`,
  `/mob/char.png`, posed by `setRotationAngles(0, …)` with `onGround` 0),
  an item (`renderItem`: 3D blocks via `render_block_as_item`, anything else
  the extruded icon), or a map (both arms, `/misc/mapbg.png`, the map's
  pixels, the markers from `/misc/mapicons.png`, the name in the font).
  Lit with `ModelMaterial` (the standard item lights placed under the hand's
  modelview turned by the player's pitch and yaw), back faces culled
  (`GL_CULL_FACE` is on at `renderHand`), alpha test 0.1, no fog. Hidden in
  third person, while sleeping and with F1 (`hideGUI` hides the hand too,
  EntityRenderer.java:313).
- **Overlays** (`renderOverlays`, unless third person or sleeping, also
  with F1): fire (`isBurning`), inside a block (`isEntityInsideOpaqueBlock`;
  vanilla's texture rectangle misses the tile size, so it shows a dark
  window near the atlas corner, transcribed), water (`/misc/water.png`
  scrolled by the rotation). `GlMaterial` quads at z −0.5 in eye space,
  in that order (`item_renderer::order`), on an `Rgba8Unorm` mirror of the
  live terrain atlas (`particles::render::sync_mirror`). The portal overlay
  is `GuiIngame`'s (WP-3.6b).
- **Held items in third person (WP-3.9):** `RenderPlayer.renderSpecials`
  and `RenderBiped.renderEquippedItems` reach `renderItem` through
  WP-3.5b's `render::held_item::HeldItemHook`, filled here with
  `render_item::EntityGlItemRenderer` (the "Entity renderers" section).
  The mesh-returning hook below is what the hand pass uses:
  `(hook.render_item)(&mut ItemMeshCache, &mut Assets<Mesh>, &RenderItemArgs
  { item_id, damage, icon: render_item::get_item_icon(entity, stack),
  brightness: getEntityBrightness(1.0), statics, colorizers })` returns an
  `ItemDraw { mesh, atlas, matrix, gl_color }`: draw `mesh` at `your matrix ·
  draw.matrix` with `ModelMaterial` on `atlas.path()`'s texture
  (`texture_srgb` 1, lighting on, alpha test 0.1, cull off), `glColor` =
  `gl_color` or the entity's brightness. `render_item::is_full_3d`,
  `should_rotate_around_when_rendering`, `is_block_3d` give the branches of
  those methods. (It was defined here before WP-3.5b's hook was in the
  tree; the third-person draws now go through `render_item_on_entity_gl`,
  the same geometry on `EntityGl`.)
- **For WP-3.5b and WP-3.6b (the arm's `isRiding`):** the arm is
  `RenderPlayer.modelBipedMain`'s, whose `isRiding` every draw of the
  player sets (RenderLiving.java:26) and nothing resets. Whenever the local
  player is drawn with the player model (`GuiInventory`'s preview,
  GuiInventory.java:58; any third-person draw), call
  `ItemRenderer::player_model_drawn(player.base.is_riding())` on the
  `item_renderer::ItemRenderer` resource. `render_hand` does it for third
  person and sleeping, and the entity pass after each `RenderPlayer` draw
  (WP-3.9); `GuiInventory`'s preview does not yet. *WP-4.9:* it does
  (`render::GuiEntityRenderer::render`).
- **Not done:** the skin download (`getTextureForDownloadableImage`,
  deferred, D2: always `/mob/char.png`); anaglyph's second pass (the shell
  draws one eye; WP-4.9 keeps this as an accepted divergence, `docs/PERF.md`
  §7); `cameraZoom` (only the unused high-resolution screenshot
  changes it, so `renderHand` always runs).

## Compositing in sRGB (WP-3.7b)

Every camera now carries `bevy::camera::CompositingSpace::Srgb` (the
world camera explicitly, the others as a required component of `Camera`
registered by `ClientPlugin`; cameras share one main texture only when its
format matches). The main texture is `Rgba8Unorm` holding gamma-encoded
values, so the `GL_TARGET_UNORM` path of `gl_material.wgsl`,
`model_material.wgsl`, `terrain.wgsl` and the GUI material writes the
Java's colour values unconverted and the hardware blends them as GL blends
its framebuffer bytes; Bevy's upscale pass decodes them for the window.
The caveats in the Sky/GUI sections and the `gl_material` module docs about
linear blending no longer apply. Measured (same world, before → after):
`drawDefaultBackground` over the noon sky `(145, 163, 255)` →
`(77, 87, 137)` before, `(48, 52, 75)` after, which is GL's
`16·a + d·(1 − a)` with `a = 0xc0/255` exactly; over grass `(51, 74, 28)`:
`(27, 37, 19)` → `(23, 28, 18)` (GL: `(23, 28, 18)`); the selection outline
(black at 0.4) over sand `(218, 212, 156)`: `(175, 171, 125)` →
`(132, 128, 95)` (GL: `(131, 127, 94)`); the sun's halo now brightens the
sky around it; opaque pixels are unchanged (at most 1 byte, now GL's
rounding). Bevy's own `StandardMaterial`/gizmo shaders do not support an
sRGB-compositing target (they would write linear values); nothing in the
client uses them.

## Entity renderers (WP-3.5b)

`render::RenderPlugin` (added by `ClientPlugin`) owns `render::RenderManager`
(a non-`Send` resource: `RenderManager.instance` and
`TileEntityRenderer.instance` with every renderer and its models) and draws
the entities. Module docs: `render/mod.rs` (the frame), `render/gl.rs` (the
GL emulation), `render/apply.rs` (the Bevy side).

- **The pass.** `render::render_entities` runs `RenderGlobal.renderEntities`
  inside `GameFrameSet` (after `game_frame`, before the F3 text is
  published, so F3's `E: rendered/total. B: 0, I: n` line —
  `DebugSources::entities` — is this frame's): the
  `renderEntitiesStartupCounter` (two skipped frames after every
  `loadRenderers`; kept here, reset on the same events as the terrain's
  reload), `cacheActiveRenderInfo`, the weather effects, every loaded entity
  passing `isInRangeToRenderVec3D` (with the render-only overrides the
  simulation's dispatch leaves to the client: the lightning bolt, the egg,
  fireball, fish hook and snowball), the frustum (`terrain::frustum`) or
  `ignoreFrustumCheck`, the view entity only in third person or asleep, and
  `blockExists`; then the tile entities of `terrain::TerrainTileEntities`
  (looked up in their chunks; nothing is loaded). Entities are taken out of
  their slots one at a time so a renderer gets `&Entity` and `&mut World`
  (light, blocks, the minecart's rail queries), as the simulation does.
  `render::render_pickups` (after the particles' frame) draws
  `particles::PickupFxFrame`'s entities on the same GL state.
- **`EntityGl`.** The renderers are transcribed line for line onto
  `render::gl::EntityGl`: `glPushMatrix`/`glTranslatef`/`glRotatef`/`glScalef`,
  `glEnable`/`glDisable`, `glBlendFunc`, `glDepthFunc`, `glDepthMask`,
  `glColor`, `glNormal3f`, `glMatrixMode(GL_TEXTURE)`, `bindTexture` and the
  `Tessellator` (quads, line and triangle strips). State carries from one
  entity to the next as GL's does. A Tessellator draw is transformed and lit
  on the CPU (GL's per-vertex lighting with `RenderHelper`'s two lights; the
  normal treated as `GL_RESCALE_NORMAL` or `GL_NORMALIZE` say: vanilla
  arrows are lit at 5.6 % until a slime latches `GL_NORMALIZE` for the
  session — vanilla bug, kept) and flat-shaded by GL's provoking vertex;
  consecutive draws under one state merge. `ModelBase.render` becomes a
  model draw.
- **Bevy side.** Model draws become instances of WP-3.5a's shared part
  meshes, kept per owner (a sim entity id, a tile entity, a pickup; one
  instance per `render` call, in call order: main model, pass models, the
  hurt overlay's copies), with their own `model::ModelMaterial`, created
  posed and despawned once the entity leaves the world (or draws nothing
  for 600 frames). Opaque passes are in the opaque phase; blended ones
  (`order::MODEL_BLEND`) and every Tessellator batch
  (`render::material::EntityGlMaterial`, pooled meshes at
  `order::TESS + n` in call order) in the entity band before the particles.
  Textures are the shared cache's sRGB images (no mirrors).
- **Colours.** As everything else drawn on the main target (`gl_material`
  "Colours", "Compositing in sRGB"): with the cameras compositing in sRGB
  since WP-3.7b, `EntityGlMaterial` and `ModelMaterial` take the
  `GL_TARGET_UNORM` path and return the Java values unconverted, so opaque
  pixels are vanilla's and the shadows (alpha ≈ 0.25-0.5), the hurt overlay
  and the lightning's additive strips (`GL_SRC_ALPHA, GL_ONE`) blend as GL
  blends its framebuffer bytes. (Before WP-3.7b they blended in linear
  space: lighter shadows, the lightning much dimmer over the sky.)

### For WP-3.7b

**Integration status (WP-3.9):** WP-3.7b, merged first, defined its own
hook for the hand pass, `item_renderer::render_item::HeldItemHook` (a
function returning an `ItemDraw { mesh, atlas, matrix, gl_color }`).
WP-3.9 bridges the two: `ItemRendererPlugin` inserts
`render::held_item::HeldItemHook(Some(Arc::new(EntityGlItemRenderer)))`,
whose `render_item` is `renderItem` transcribed on `EntityGl`
(`item_renderer::render_item::render_item_on_entity_gl`: a 3D block
through `render_block_as_item`, else WP-3.7b's `extruded_icon` under
`GL_RESCALE_NORMAL` and `renderItem`'s matrix calls). Skeletons hold their
bow, pig zombies their golden sword, the third-person player its held
item and its helmet block. The entity pass calls
`ItemRenderer::player_model_drawn(isRiding)` after each `RenderPlayer`
draw (`render_hand` also still does for third person and sleeping).

`ItemRenderer.renderItem(entity, stack)` (the held items of `RenderPlayer`
and `RenderBiped`, the helmet slot's block) calls
`render::held_item::HeldItemHook`: implement `HeldItemRenderer` and insert
`HeldItemHook(Some(Arc::new(..)))`. `render_item(gl, args)` draws on the
same `EntityGl` under the hand's matrix (transcribe `renderItem` on it:
`gl.push_matrix()`, `gl.bind_texture("/gui/items.png")`, `gl.translatef`,
`gl.start_drawing_quads()`, `gl.set_normal`, `gl.add_vertex_with_uv`,
`gl.draw()`, …; a 3D block with `terrain::render_blocks::render_block_as_item`,
`gl.color4f` of its `gl_color` and `gl.draw_tessellator`); `args` has the
entity, the stack (`Item.stick` while fishing), `getEntityBrightness(1.0F)`
and the block statics and colour maps. Without a hook nothing is drawn
(skeletons hold no bow, the player's hand is empty). The first-person hand
is not drawn by this pass: `ItemRenderer.renderItemInFirstPerson` gets
`RenderPlayer` from `RenderManager` and calls `drawFirstPersonHand`, which
is `render::living::RenderLiving::draw_first_person_hand(&mut EntityGl)` on
`NonSendMut<RenderManager>`'s `renderers.living_mut(RendererId::Player)`
(RenderPlayer.java:226-230: `onGround = 0`, the rest pose, then only
`bipedRightArm`, on the shared `modelBipedMain`, so `isSneak` and
`heldItemRight` carry over from the last third-person draw as in vanilla);
the arm lands in that `EntityGl`'s `out.models` as one `ModelDraw` (the
modelview and GL state at the call), which 3.7b draws in its own hand pass
(`render::apply` only places `EntityFrame`'s world-space draws). WP-3.6b reuses
nothing of this module
(`renderItemIntoGUI` is GUI drawing).

### Not drawn

Skins and cloaks (`loadDownloadableImageTexture` binds the entity's own
texture), `deadmau5`'s ears, other players' name tags (none in
singleplayer). The F3 entity-id labels of `RenderLiving.passSpecialRender`
are drawn.

## Performance counters and saves (WP-4.3)

- `perf` (a project addition, no vanilla counterpart): `PerfStats` keeps the
  last 600 frames' frame time (wall time between two app updates) and
  update time (the main schedule's own time, `First` to `Last`: the CPU
  work of the frame, without the wait for the render thread and the
  display), the time per phase, and the start-up milestones. A phase is
  timed inside its systems with `let _perf = perf::time(Phase::X);`
  (`Phase`: `Sim` = `game_frame`, `Terrain`, `Entities`, `Particles`,
  `Gui` = the screen and HUD draws, `GuiFlush` = `flush_gui`, `Sky` = sky,
  clouds, weather, fog and selection, `Hand`). A new system that should
  count toward a phase adds that line.
- `dump_debug` also writes `perf` (`frame`, `update`, `phases.<name>`: each
  `n`, `min_ms`, `avg_ms`, `p95_ms`, `max_ms`; `startup`: `plugin_built_ms`,
  `title_ms`, `in_game_ms`, from the process start) and `storage` (the
  writes on the calling thread, the write-behind queue's region writes:
  queued, coalesced, journaled, whole, unchanged, bytes, and the writer
  thread's time).
- `dump_debug` also writes `gaps` (WP-4.8, a project addition): the open
  world's `World::skipped_systems` and `World::unimplemented_hits` as
  `{name: times reached}` objects with sorted keys (`null` with no world;
  `script::gaps_json`). The counters belong to the `World`, so they start
  at zero with each world the shell creates or loads (a dimension change
  too). In normal play both stay empty; `docs/DEFERRED_REVIEW.md` §3.1 says
  what each record means.
- With F3 on, the right column gets four `[rust]` lines from row 5
  (y = 52) down, below the rows of the left column's five lines so no row
  holds both: `[rust] frame` and `[rust] update` (min/avg/p95/max of the
  window, ms) and two lines of the phases' means (`sim ter ent`,
  `fx gui flush`).
- `storage::FsStorage::with_write_behind` (the game's storage): `write`,
  `rename` and `delete` are queued for a background thread in call order,
  reads see the queued state, a region file is written once per batch and
  only in its changed sectors, through a journal (module docs). Call
  `storage::flush_writes()` before reading save files with anything but
  the game's `Storage` (the game does before it exits). A failed queued
  operation is logged and does not stop the next (RegionFile.java:233-234
  prints and goes on); the first error per root comes back only from
  `flush_writes()` / `FsStorage::flush_writes()`. If the writer thread
  cannot start, operations run at once on the calling thread.

## Stats autosave and the dead player (WP-4.7)

- `Game::run_tick_begin` calls `StatFileWriter::func_27178_d` every
  `runTick` (Minecraft.java:949), on the title screen too: once a stat has
  changed, `stats/stats_<user>_unsent.dat` is rewritten every 100 ticks
  (StatsSyncher's send, run at once; local files, nothing online). At
  start-up an existing `stats/stats_<user>.dat` (the "received" copy) is
  read and merged into the totals on the first tick (`func_27180_b`).
- A dead player leaves `loadedEntityList` 21 ticks after its death but
  stays in `playerEntities`; `mc_sim::world::World` keeps that object
  (`World::entity_object`). `Game::player`, `Game::render_view` and
  `Game::stats_mut` read it there; `Game::detached_player` is a copy of it
  refreshed after every `updateEntities`. Leaving the world saves it to
  `level.dat` (PARITY_CHECKLIST P4), so the world reopens on the
  game-over screen (the loaded player has `Health` <= 0, `DeathTime` 21)
  and Respawn works there; `respawn` takes it out of `playerEntities`
  (`World::with_entity_object_mut` + `set_entity_dead`). The new player
  takes the dead player's id (Minecraft.java:1451, :1468); the fields that
  named the dead player (a mob's `currentTarget`, `playerToAttack`, a
  ghast's `targetedEntity`, an arrow's `owner`, a bobber's `angler`) keep
  naming the dead object, as Java's fields do: `respawn` moves it and them
  to a fresh negative id (`World::move_removed_entity_to_fresh_id`).

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
| `use_portal` | — | Dev: `Minecraft.usePortal()` at once. It bypasses `EntityPlayerSP.onLivingUpdate`'s portal timers: `timeUntilPortal` is not set to 10 and `timeInPortal` is not reset, so used while standing in a portal the arrival portal sends the player straight back (Gate 3 O2). The real path is walking into a portal (`key.forward`): `timeInPortal` rises 0.0125 a tick and `usePortal` runs at 1.0 (80 ticks), `timeUntilPortal` 10 keeps the arrival portal from firing until the player has been out of it for 10 ticks (EntityPlayerSP.java:39-69, EntityPlayer.java:811-817) |
| `respawn` | — | Dev: `Minecraft.respawn(false, 0)` |
| `quit` | — | `Minecraft.shutdown()`: leaves the world (saving it and the player) and exits |
| `dump_texture` | `name`, `path` | Dev (WP-3.2): writes the texture's current pixels (for `"terrain"` / `"items"` the atlas with its animated tiles; else a jar path as `getTexture` takes it) to a PNG |
| `set_time` | `world_time` | Dev (WP-3.4): `World.setWorldTime` (0 sunrise, 6000 noon, 12000 sunset, 18000 midnight) |
| `set_rain` | `strength` (0..1), `thunder` (bool, default false) | Dev (WP-3.4): sets `rainingStrength` (and the previous value), `raining`/`thundering` with long timers, and the thunder strength to 1 or 0; `updateWeather` then moves the strength 0.01 a tick toward 1 or 0 |
| `set_pos` | `x`, `y`, `z` | Dev (WP-3.4): moves the player (`posY` is the eye, as `dump` prints it), no interpolation, motion and fall distance cleared |
| `aux_sfx` | `kind`, `x`, `y`, `z`, `data` | Dev (WP-3.8): queues `world.playAuxSFX(kind, x, y, z, data)` as the simulation would (`1005` with `data` 2256 plays record `13` at the block, `data` 0 stops it; `2001` with a block id plays its break sound) |
| `set_option` | `option`, `step` (default 1) | Dev (WP-3.3a): `gameSettings.setOptionValue(option, step)` with its side effects and `saveOptions`, as the options screen's button (`option` is the translation key: `"options.ao"`, `"options.graphics"`, `"options.renderDistance"`, …) |
| `dump_debug` | `path` | Dev (WP-3.3a): writes the F3 text (`DebugText`, `DebugSources`, the fps line) and `terrain::TerrainStats` as JSON |
| `debug_model` | `model`, `path`; optional `texture`, `distance` (3), `turn`, `limb_swing`, `limb_yaw`, `ticks`, `head_yaw`, `head_pitch`, `swing`, `scale`, `brightness` (1), `blend_alpha`, `additive` (the powered creeper's pass), `keep`, `sitting` | Dev (WP-3.5a): spawns a model (`"pig"`, `"sheep_fur"`, `"biped_armor"`, … = `model::ModelKind`) standing `distance` blocks in front of the player at feet level, facing the camera turned by `turn` degrees, posed with those `render` arguments, its skin from the jar (default: the renderer's texture); with `path`, waits 20 frames and saves a screenshot. Earlier debug models are removed unless `keep` (to draw a pass model over its main model). Hidden while no world is drawn |
| `debug_gui` | `screen`, `path` (optional) | Dev (WP-3.6a): opens a GUI test screen — `"widgets"` (buttons normal/disabled, small buttons, a slider, a focused text field; the button under the pointer shows hovered), `"list"` (a `GuiSlot`), `"font"` (every `§` colour code and glyph), `"yesno"` (a `GuiYesNo` over the widget screen) or any registered screen name — and with `path` screenshots it three frames later |
| `open_screen` | `name` | Dev (WP-3.6a): `displayGuiScreen(new X())` for a screen class name (`"GuiConflictWarning"`, `"GuiErrorScreen"`, `"GuiIngameMenu"`, …; an unregistered one is a placeholder) |
| `mouse_to` | `x`, `y` | Dev (WP-3.6a): puts the pointer on scaled GUI pixel `(x, y)` (hover; later button events carry the position) |
| `type_text` | `text` | Dev (WP-3.6a): a key press and release per character, carrying the character (`keyTyped`) |
| `click_slot` | `index`, `double` (false) | Dev (WP-3.9): clicks row `index` (0 = the first) of the current screen's `GuiSlot` list (the world list, the statistics tables, the texture packs): the pointer moves to the row's centre where `GuiSlot` draws it (`script::slot_row_center`; a short list is centred, a scrolled one moved), the left button is held two frames (`GuiSlot` polls it while drawing); with `double`, a second click four frames later (on the world list: Play). `GuiSlot` counts it as a double click only if the second press comes under 250 ms of wall-clock time after the first (`System.currentTimeMillis()`, GuiSlot.java:138), so `double` assumes frames under about 60 ms (above about 16 fps): on slower frames (a loaded machine, a debug build in CI) the second press is a single click, the log still says `clicking row N twice` and Play does not run; follow it with a `dump` or `screenshot` to check. A row scrolled out of the list is not clicked (an error is logged). Waits up to 120 frames for such a row |
| `click_button` | `label` | Dev (WP-3.6c): presses the enabled button labelled `label` (its text as drawn, e.g. `"Singleplayer"`, `"Options..."`, `"Done"`, `"Save and quit to title"`, `"Import..."`) on the current screen: the pointer moves to its centre, then a left press and release. Waits up to 120 frames for such a button |
| `particle` | `name`, `x`, `y`, `z`, `dx`, `dy`, `dz` (default 0), `ahead` (optional) | Dev (WP-3.7a): queues `world.spawnParticle(name, x, y, z, dx, dy, dz)` as the simulation would (`smoke`, `flame`, `reddust`, `explode`, `heart`, `note`, `bubble`, `portal`, `lava`, `splash`, `largesmoke`, `snowballpoof`, `snowshovel`, `slime`, `footstep`); with `ahead: d`, `x`, `y`, `z` are offsets from the point `d` blocks along the player's look from the eye |
| `give` (alias `give_item`) | `id`, `count` (1), `damage` (0), `slot` (optional), `armor` (optional) | Dev (WP-3.6b, WP-3.7b; merged in WP-3.9): puts `new ItemStack(id, count, damage)` into armour slot `armor` (0 boots … 3 helmet) when given, else into main-inventory slot `slot` (0-8: the hotbar; replacing what is there) when given, else into the inventory as a pickup does (`addItemStackToInventory`). Select it with the number keys (`key_down` `"1"`…`"9"`) or `wheel`. `give_item` is the same action (old scripts keep working) |
| `set_fire` | `ticks` | Dev (WP-3.7b): sets the player's `fire` (`isBurning()`, the fire overlay; the simulation counts it down, burns the player every 20 ticks, and water puts it out) |
| `spawn_entity` | `kind` (an `EntityList` name, or `"LightningBolt"`, `"Fireball"`, `"Egg"`), `x`, `y`, `z`, `ahead`, `yaw`, `pitch`; per kind `item` (`[id, count, damage]`), `minecart_type`, `direction` (painting), `block_id` (falling block), `fuse` (primed TNT, default 80), `fire`, `hurt_time`, `health`, `powered`, `saddled`, `sheared`, `fleece`, `size`, `angry`, `tamed`, `shake` | Dev (WP-3.5b): spawns the entity as the simulation would (its constructor, `spawnEntityInWorld`; a bolt with `addWeatherEffect`); with `ahead: d`, `x, y, z` are offsets from the point `d` blocks ahead of the player horizontally, at its feet |
| `set_block` | `x`, `y`, `z`, `id`, `meta` (default 0), `to` (`[x, y, z]`: a box), `ahead`, `text` (a sign's four lines), `raw` (default false), `rel` (default false) | Dev (WP-3.3b, WP-3.3d, WP-3.5b, WP-3.7b): `world.setBlockAndMetadataWithNotify(x, y, z, id, meta)` as the simulation would (`onBlockAdded` and the neighbours' updates run, as for a placed block: a redstone torch powers its wire, a piston beside a torch extends, a placed flowing source, id 8 or 10, then flows); with `raw`, `setBlockAndMetadata` without neighbour updates, the metadata forced again (`onBlockAdded`, e.g. a rail's, may change it) and the block marked for re-render: shapes the game would not keep (a lone curved rail, a powered wire with no source). A new world starts with tens of thousands of scheduled ticks from population (sand and gravel placed by the populators), run at most 1000 a tick in time order, so there a new block's first tick can wait hundreds of ticks (seed 123: more than 400, less than 1500); an opened save starts with none (b1.7.3 saves no scheduled ticks). With `rel` (WP-3.7b), the coordinates are offsets from the player's block (`floor(posX)`, `floor(posY)`, `floor(posZ)`; `posY` is the eye): `{"id": 9, "rel": true}` puts water at the eye (the water overlay), `1` stone (the inside-of-block overlay). With `to` (WP-3.5b), every block of the box from `x, y, z` to `to`; with `ahead: d`, the coordinates are offsets from the point `d` blocks ahead of the player horizontally, at its feet; with `text`, a sign's four lines (walls for paintings, signs, mob spawners) |
| `equip` | `slot` (0-35 inventory, 100-103 armour: boots … helmet), `item` (`[id, count, damage]`, absent: empty) | Dev (WP-3.5b): puts a stack into one of the player's slots (armour on the third-person player) |
| `chat` | `text` | Dev (WP-3.6b): `ingameGUI.addChatMessage(text)` |
| `dump_inventory` | `path` | Dev (WP-3.6b): the player's 36 main and 4 armour slots and the cursor stack, each `[id, count, damage]` or null, as JSON |
| `screenshot_frames` | `dir`, `frames` | Dev (WP-4.9): captures each of the next `frames` frames into `dir/fNNNNN.png` (NNNNN = the script's frame number) without waiting for the files, and appends a line per frame to `dir/frames.jsonl` (`frame`, `nanos`, `millis` (wall clock, to line up with the log), `state`, `ticks_ran`, `loading`, the loading screen's `title`, `message` and `progress`, `world`). Follow it with a `wait` long enough for the captures to land before `quit`. `dump_debug` also writes F3's right column (`right`: the memory lines) since WP-4.9 |
| `export_world` | `name`, `path`, `report` (optional) | Dev (WP-4.5): exports `saves/<name>` to `<path>/<name>` (`export::export_from_game`: the open world is saved first when it is that world; then validated, copied and read back). In game the save is a full `saveWorld(true)`, which vanilla runs only on quit or a world change, so the run's later autosaves write other chunks than an unexported run would. With `report`, writes `{"ok": true, "report": {path, files, bytes, regions, chunks, digest}}` (the digest lists every file's and every chunk's blake3) or `{"ok": false, "error": …}` |

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

## Wave F fixes (WP-4.9)

The client half of docs/PARITY_CHECKLIST.md §2, each checked against the
original client (§13 there has the evidence):

- **The view entity when dead (P3).** `render::render_entities` and
  `item_renderer::compute_hand` resolve `renderViewEntity` / `thePlayer`
  as `Game::render_view` does: the world's entity, else the detached copy
  the shell keeps after the world dropped a dead player. The Game Over
  screen shows the world's entities, the tile entities and the arm.
- **F3's memory lines (P5).** `debug::process_memory` fills
  `DebugSources::{max,total,free}_memory` from `/proc` (a quarter of the
  physical memory as the JVM's default `-Xmx`, the resident set, its
  file-backed part); elsewhere they read 0.
- **The frame rate (P6).** See "Running" above.
- **`portal.travel` (P7).** The tick's `UsePortal` is run by the frame
  driver after `sound::play_client_sounds_now` has put that tick's sound
  calls through the `SoundManager` and started their static plays on
  kira's audio thread, so the sound starts before the world change blocks
  the frame (EntityPlayerSP.java:58-59).
- **Pipelines compile in the frame that needs them (P8, the exit
  segfault).** `mc-app` sets `RenderPlugin::synchronous_pipeline_compilation`:
  the first frame after a load draws the terrain (with `terrain.png`'s
  still fluid tiles: the title screen pauses the game, so no
  `updateDynamicTextures` has run yet), not two blank frames, and no
  pipeline is compiling on a task-pool thread when the process exits. The
  cost is a one-off hitch the first time a pipeline is needed. The flat
  water and lava of the next frames are vanilla's too in a fresh process:
  `TextureWaterFX` and `TextureLavaFX` start from zeroed arrays and take
  seconds to develop (PARITY_CHECKLIST §13).
- **No Power saver sleep and no `renderEndNanoTime` while a world loads**
  (`terrain::power_saver_sleep`, `terrain::mark_render_end`): vanilla's
  `func_6255_d` blocks the loop that runs them.
- **GL draws sample at the centroid** (`gl_material.wgsl`, as
  `terrain.wgsl`): under the cameras' MSAA an edge pixel's centre can lie
  outside the polygon and sample the next atlas tile (lines along the
  preview's fire, stray texels at 3D icons' edges).
- **The loading bar (P9)** was always drawn; `screenshot_frames` shows it.
  The achievement window is no longer drawn over the loading screen.
- **`updateRenderers`' budget (P10)** starts at the end of the previous
  frame (`terrain::mark_render_end` in `Last`), as `renderEndNanoTime`.
- **Pass 1 with fast graphics** is drawn nearest first (above).
- **The inventory's player** is drawn by the renderers (above).
- **Not done:** anaglyph's second eye (accepted) and a floating origin
  (designed in `docs/PERF.md` §7).

