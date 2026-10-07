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
| `events` | `IWorldAccess`, `SoundManager` call sites | `WorldEventMsg`, `ClientSound`, `ClientEvent` |
| `script` | — | the scripted-input dev mode |
| `jfmt`, `clock`, `paths` | JDK behaviour | `Float/Double.toString`, `parseFloat`/`parseInt`, `String.split`/`hashCode`; the clocks and `new Random()`'s seed; the default paths |

`mc-sim/src/world/teleporter.rs` (`Teleporter.java`) is the simulation half of
nether travel.

## For the other client work packages

- The frame runs in `Update` in the `GameFrameSet` system set. Order your
  systems `.after(GameFrameSet)` to read this frame's state.
- `NonSend<Game>` / `NonSendMut<Game>` is `Minecraft`: `the_world`,
  `the_player`, `player()`, `game_settings`, `current_screen`,
  `loading_screen`, `object_mouse_over`, `timer`, `entity_renderer`,
  `input` (the LWJGL buffers), `chunks_updated`, `frame_times`/`tick_times`.
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
  mouse-over), `CameraState` (the full camera setup, eye in f64),
  `DebugText` / `DebugSources`, `JarAssets`, `Lang`, `ResourcesDir`,
  `PendingScreenshots`; state `AppState`.

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
| `dump` | `path` | Writes a JSON of the state: app state, `ticks_ran`, screen, focus, pause, third person, F3, the mouse-over, the world (dir, level name, seed, dimension, world time, loaded chunks, entities, spawn, rain) and the player (id, position, rotation, motion, on ground, health, dimension, hotbar slot, name) |
| `save` | — | `saveWorldIndirectly` (saves the world and the player now) |
| `use_portal` | — | Dev: `Minecraft.usePortal()` at once |
| `respawn` | — | Dev: `Minecraft.respawn(false, 0)` |
| `quit` | — | `Minecraft.shutdown()`: leaves the world (saving it and the player) and exits |

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
