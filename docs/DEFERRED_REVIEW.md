# Deferred-tag and gap review (WP-4.4)

Written 2026-10-09 for the owner's review. PLAN_v4 §5 Phase 4 row 4.4:
"every `deferred` citation is still out of scope under D3, or scheduled".
This document goes further, as the brief asked. It also covers the gaps
that carry no `deferred` tag, reconciles the §5 WP table with what was
delivered, collects every open owner question, and audits git for
Mojang-derived files (D8).

**WP-4.8 update (2026-10-10, branch `wp-4.8` from `phase-4` at `d222a15`).**
WP-4.8 ("Gap hygiene", §3.6) closed §2.4, §3.1 B and D, the N1/N4/N6/N7 and
R4/R6/R7 rows of §3.3 and Q12/Q13, and traced §3.1 E site by site. Its
results are marked "WP-4.8" in place; the trace and its findings are
§3.1 E, "Update (WP-4.8)". In short: the sim now has 76 `note_unimplemented`
call sites (96 at `d222a15`); a fixture's reached gaps are in
`target/golden/<area>/<name>/gaps.json` and the game's in `dump_debug`'s
`gaps`; the trace found five reachable divergences: four are fixed with
new oracle fixtures, one (the reused player id) has a proposal (§3.1 E).

Reviewed tree: branch `wp-4.4` at `d52ac6b` (the Gate 3 commit). The
`phase-4` tip `70cdb29` adds only WP-4.2 (the soak fixtures, the `SOAK`
modes of the oracle scripts and `golden_soak.rs`). It adds no `deferred`
tag and no `note_unimplemented` site: `git grep` on `phase-4` gives the
same 75 and 98 lines. Java citations were checked against the D2 tree
`.reference/mc_b1.7.3_release/1.7.3-LTS/src/minecraft`. No other Java
tree was read.

**Verdict.**

- **The tags are right, with one exception.** All 75 `deferred` lines
  were checked; 64 of them are scope tags. Each one cites a defunct online
  service (D2), multiplayer, texture packs, the sky dimension or pre-McRegion
  conversion (D3). Each stays out of scope (§2), except one:
  - **Stats "sync" is local file I/O, not an online service.**
    `StatFileWriter.func_27178_d` (Minecraft.java:949) runs every tick. In
    vanilla it rewrites `stats_<user>_unsent.dat` about every 100 ticks. Its
    "sync" threads only read and write files in `stats/` (StatsSyncher.java,
    ThreadStatSyncherSend.java). The port writes stats only on a world change
    and at shutdown, so a crash loses the whole session's stats. This should
    be scheduled (§2.3, proposed WP-4.7). PLAN_v4's own WP-2.7 row repeats
    the mislabel ("`StatsSyncher` deferred per D2"), and so does **D2
    itself**: PLAN_v4.md:100 lists "stats sync" among the defunct online
    services. Scheduling it therefore amends the text of an owner decision,
    not only a WP row (Q34).
- **One tag text was wrong and is fixed.** `crates/mc-client/src/game.rs:88` (89 after the fix)
  named "the snooper", which b1.7.3 does not have. It now names the systems
  in its own parenthesis (§7).
- **The sky dimension costs no singleplayer parity.** Vanilla never opens
  it in singleplayer: World.java:184-190 maps every saved dimension except
  −1 to the surface. Of the D3 items, only pre-McRegion conversion and
  texture packs are reachable in vanilla singleplayer.
- **`note_unimplemented` census.** `crates/mc-sim/src` has 97 call sites
  (98 lines with the definition), grouped in §3.1 by what they mean:

  | Kind | Sites | Notes |
  |---|---|---|
  | Parity gaps reachable in singleplayer | 10 | Six are WP-4.6's brief. Four are **new**: (1) `EntityFireball.getCollisionBorderSize`, reached through the client's picking (camera.rs:567): a ghast fireball's pick box grows by 0.1 instead of 1.0, so it is much harder to hit back. (2) A projectile whose shooter has left the world (arrow.rs:312): switch to Peaceful while a skeleton's arrow is in flight, and the port deals 4 damage where vanilla deals none and the arrow bounces. (3) A mob's look target that has left the world (living.rs:324). (4) A container over a removed tile entity (inventory/container/mod.rs:115): an explosion destroys an open chest |
  | Client-only, dead in the shipping game | 14 | The client has its own implementations |
  | Multiplayer-only | 5 | |
  | Dead code | 6 | |
  | Defensive guards | 62 | One family can be reached in rare play: a reference to an entity already removed from the list (the fishing bobber, a mount; §3.1 E family 3) |

  Two of WP-4.6's sites matter more than the gate documents say:
  - **Cake.** Eating cake within about 10 ticks of a hit is observable, in
    combat and in the HUD's heart blink.
  - **Kill hooks.** The missing `onDeath` hooks mean the "Monster Hunter"
    achievement (`killEnemy`) can never be earned, now that WP-2.7 counts
    stats.
- **Owner questions.** The questions from PLAN_v4 §8 and Gates 0–3, plus
  four new ones from this review, are deduplicated into 34 items in §5:
  - 4 are resolved;
  - 4 are scheduled;
  - 2 are done or moot;
  - 24 are open, 3 of them with a WP proposed here.

  The ones that still block something are the `Entity.rand` rewrite (§8
  item 3) and the `testWorld` licence (Gate 1 Q9).
- **Licensing (D8).** `git ls-files` holds 17 binary files:
  - 13 are the template's Bevy icons;
  - 1 is the oracle's sine-table dump;
  - 3 are byte-for-byte copies of the owner's `testWorld`. Gate 1 Q9 named
    two of them; the third is `tests/golden/nbt/testworld-level-dat/level.dat`.

  Nothing from `minecraft.jar` or `resources/` (textures, `lang/`, sounds,
  `font.txt`, splashes, `achievement/map.txt`) is committed or embedded (§6).

## 1. Method and counts (the smoke run of this WP)

| Command (from the worktree root) | Result |
|---|---|
| `grep -rn 'deferred' crates/ tools/ docs/PLAN_v4.md \| wc -l` | **60** (case-sensitive) |
| `grep -rni 'deferred' crates/ tools/ docs/PLAN_v4.md \| wc -l` | **75**: mc-client `src` 32, mc-client README 7, mc-sim `src` 28, mc-sim tests 1, PLAN_v4 7, `tools/` 0, mc-core/mc-nbt/mc-golden/mc-app 0 |
| … of which scope tags | **64**. Not tags: World.java's "deferred while the tile-entity pass runs" (world/mod.rs:1915), the `SaveError::Deferred` variant (save/mod.rs:109, :121), the test that asserts it (golden_save.rs:1175), and the 7 PLAN lines |
| D2/D3 tags without the word (`grep -rnE '\bD[23]\b' crates --include=*.rs`, minus `TextureDimension::D2`, key names and hex) | **36** lines, §2.2 |
| `grep -rn 'note_unimplemented(' crates/mc-sim/src \| wc -l` | **98** = 97 call sites + the definition (world/mod.rs:688) |
| `World::skip(` sites | **1** (world/entities.rs:185) |
| `untranscribed(` tails in block/dispatch.rs | **5** (lines 1090, 1160, 1199, 1556, 1979), all reaching dispatch.rs:680 |
| `note_unimplemented` outside mc-sim | **1** (mc-golden tick_driver.rs:621, harness only); mc-client never reads `unimplemented_hits` |
| `git ls-files \| wc -l` / binary (`git ls-files --eol`, `i/-text`) | **1,004** / **17** |

Disposition legend:
- **D2**: a defunct online service; stays out of scope, with nothing to
  schedule (vanilla today behaves the same).
- **D3-MP / D3-TP / D3-SKY / D3-CONV**: multiplayer, texture packs, the sky
  dimension or pre-McRegion conversion; stays deferred to Phase 6, which
  needs an owner decision.
- **PLAT**: the Java applet; not applicable.
- **SCHED**: not out of scope, scheduled or proposed.
- **—**: not a scope tag.

## 2. Every `deferred` hit

"Cite" says whether the Java the line names (or the `// src:` next to it)
was opened and matches. A line-number nit is listed in §2.4 and was not
edited.

### 2.1 The 75 lines

**mc-client `src` (32)**

| # | Site | What is skipped (Java) | Kind | Cite | Disposition |
|---|---|---|---|---|---|
| 1 | assets.rs:11 | `TexturePackCustom`, `TexturePackList`; default pack = the jar | D3-TP | ok (line nits, §2.4) | stays, Phase 6 |
| 2 | assets.rs:230 | `ThreadDownloadResources` (downloads from `s3.amazonaws.com/MinecraftResources/`, ThreadDownloadResources.java:34); `resources/` is the user's | D2 | ok (Minecraft.java:1380-1396) | stays |
| 3 | session.rs:3 | login, session id | D2 | ok (field cites one line early, §2.4) | stays |
| 4 | settings.rs:236 | `skin` = texture pack name, kept for `options.txt` | D3-TP | ok (GameSettings.java:27) | stays |
| 5 | game.rs:16 | `func_28001_B` (`ThreadCheckHasPaid` at `ticksRan == 6000`) and `statFileWriter.func_27178_d` | D2 + **SCHED** | ok (Minecraft.java:940-949) | has-paid: stays. `ThreadCheckHasPaid.java:17-21` sets `hasPaidCheckTime` only on an HTTP 400 from `login.minecraft.net`, so vanilla today never shows the "Unlicensed Copy" lines either. `func_27178_d`: **not out of scope**, §2.3 |
| 6 | game.rs:88 (89 after §7) | applet, multiplayer, has-paid check, resource downloads, stats sync | PLAT/D3-MP/D2 + **SCHED** | text was wrong ("the snooper", which b1.7.3 lacks): **fixed** (§7) | stays, except stats (§2.3) |
| 7 | game.rs:521 | `isMultiplayerWorld()` always false | D3-MP | ok (Minecraft.java:1187-1189) | stays |
| 8 | game.rs:1021 | :945-947 has-paid, :949 `func_27178_d` | D2 + **SCHED** | ok | as #5 |
| 9 | game.rs:1022 | continuation of #8 ("stats sync, deferred D2") | **SCHED** | label wrong (not online) | §2.3 |
| 10 | game.rs:1474 | `convertMapFormat` | D3-CONV | ok (Minecraft.java:1194-1195, :1339-1344) | stays, Phase 6 |
| 11 | game.rs:1476 | the refusal message for #10 | D3-CONV | n/a | stays |
| 12 | gui/achievement.rs:17 | "Unlicensed Copy :(" lines (`hasPaidCheckTime`) | D2 | ok (GuiAchievement.java:58-71) | stays |
| 13 | gui/ingame.rs:23 | F3 text shifted 32 px when `hasPaidCheckTime > 0` | D2 | ok (GuiIngame.java:168-170) | stays |
| 14 | gui/menu/mod.rs:34 | header "Deferred (PLAN_v4 D3)": `GuiMultiplayer`/`GuiConnecting`/`GuiConnectFailed`/`GuiDownloadTerrain`; texture packs; conversion; "the stats upload (`StatsSyncher`)" | D3-MP/TP/CONV + **SCHED** | the last item is mislabelled (D3, "upload") | stays, except stats |
| 15 | gui/menu/ingame_menu.rs:102 | `theWorld.sendQuittingDisconnectingPacket()` | D3-MP | ok (GuiIngameMenu.java:30) | stays |
| 16 | gui/menu/main_menu.rs:2 | Multiplayer button always disabled | D3-MP | ok (GuiMainMenu.java:60, :69-70) | stays. **Visible deviation:** vanilla enables it whenever `session != null`, which is always (owner Q33) |
| 17 | gui/menu/main_menu.rs:212 | same | D3-MP | ok | stays |
| 18 | gui/menu/main_menu.rs:228 | `new GuiMultiplayer(this)` | D3-MP | ok (GuiMainMenu.java:75-96) | stays |
| 19 | gui/container/sleep.rs:5 | `GuiSleepMP`'s packet for `EntityClientPlayerMP` | D3-MP | ok (Minecraft.java:975-979; cited 978-983, §2.4) | stays. Singleplayer never opens `GuiSleepMP` |
| 20 | gui/container/sleep.rs:24 | `wakeFromSleep` → `Packet19EntityAction` | D3-MP | ok (GuiSleepMP.java:45-51) | stays |
| 21 | gui/container/inventory.rs:44 | the preview's skin download | D2 | ok (EntityPlayer.java:51 `/mob/char.png`) | stays |
| 22 | gui/menu/texture_packs.rs:2 | the pack list holds only the default | D3-TP | ok | stays |
| 23 | gui/menu/texture_packs.rs:146 | "Open texture pack folder" disabled | D3-TP | ok | stays. **Visible deviation:** vanilla's button is enabled (Q33) |
| 24 | gui/menu/texture_packs.rs:173 | `Sys.openURL("file://" + fileLocation)` | D3-TP | ok (GuiTexturePacks.java:30) | stays |
| 25 | gui/menu/texture_packs.rs:189 | `texturePackList.updateAvaliableTexturePacks()` | D3-TP | ok (GuiTexturePacks.java:52) | stays |
| 26 | render/player.rs:122 | `renderName` (another player's name tag) | D3-MP | ok (RenderPlayer.java:56-104) | stays. Singleplayer has no other player |
| 27 | render/player.rs:130 | same | D3-MP | ok | stays |
| 28 | render/player.rs:157 | deadmau5's ears, the cloak (`loadDownloadableImageTexture` of `skinUrl` / `playerCloakUrl`) | D2 | ok (RenderPlayer.java:123, :141; EntityPlayer.java:135-138) | stays: the downloads fail in vanilla today too |
| 29 | sound/mod.rs:597 | `ThreadDownloadResources` downloading | D2 | ok (ThreadDownloadResources.java:66-82) | stays |
| 30 | textures/mod.rs:17 | `getTextureForDownloadableImage`, `obtainImageData`, `releaseImageData`, `urlToImageDataMap` | D2 | ok | stays |
| 31 | textures/mod.rs:18 | `texturePack.selectedTexturePack` | D3-TP | ok (TexturePackBase.java:28-30) | stays |
| 32 | textures/mod.rs:665 | `refreshTextures`' `textureSetupComplete = false` over the downloaded images | D2 | ok (RenderEngine.java:463-533, :475) | stays |

**mc-client README (7)**

| # | Line | Says | Kind | Disposition |
|---|---|---|---|---|
| 33 | 185 | texture packs and skin/cloak downloads deferred | D3-TP, D2 | stays |
| 34 | 517 | `RenderPlayer` skins and cloaks | D2 | stays |
| 35 | 564 | `/mob/char.png`, skin downloads deferred | D2 | stays |
| 36 | 785 | title: Multiplayer always disabled | D3-MP | stays (Q33) |
| 37 | 799 | world needing conversion: "Must be converted!", opening fails | D3-CONV | stays, Phase 6 |
| 38 | 833 | texture packs: default pack only | D3-TP | stays (Q33) |
| 39 | 958 | item renderer: skin download | D2 | stays |

**mc-sim `src` (28)**

| # | Site | What is skipped (Java) | Kind | Cite | Disposition |
|---|---|---|---|---|---|
| 40 | inventory/mod.rs:11 | `InventoryBasic` built only by `NetClientHandler` | D3-MP | ok (NetClientHandler.java:488 is the only `new InventoryBasic`) | stays (the class is transcribed anyway) |
| 41 | inventory/basic.rs:5 | same | D3-MP | ok | stays |
| 42 | stats/mod.rs:18 | `StatsSyncher`: "threads and the received copy deferred (D2)" | **SCHED** / dead | label wrong | send path: §2.3. Received copy: dead unless a `stats_<user>.dat` already exists; local file I/O, not D2 (§2.3) |
| 43 | stats/mod.rs:19 | `ThreadStatSyncherReceive`, `ThreadStatSyncherSend` "deferred (D2)" | **SCHED** / dead | label wrong | as #42 |
| 44 | stats/file_writer.rs:29 | the threads, `func_27418_a`, `func_27424_a`, the received copy, the per-tick `func_27178_d` | **SCHED** / dead | Java cites ok; label wrong | as #42 |
| 45 | stats/file_writer.rs:398 | `field_27189_c` read only by the deferred `func_27178_d` | **SCHED** | ok | §2.3 |
| 46 | stats/file_writer.rs:486 | the receive thread | dead unless the file exists | ok (StatsSyncher.java:26-49) | local file I/O, not D2: dead unless a `stats_<user>.dat` already exists; optional WP-4.7 item (6) or an accepted divergence (§2.3, Q34) |
| 47 | stats/file_writer.rs:534 | `syncStatsFileWithMap`'s busy-wait for the threads | n/a | ok (StatsSyncher.java:139-165) | nothing to do once the send runs synchronously |
| 48 | stats/file_writer.rs:588 | `field_27433_f` = `stats_<user>.dat` | dead | ok | as #46 |
| 49 | world/sky.rs:18 | `WorldProviderSky` overrides | D3-SKY | ok | stays; unreachable in singleplayer (§2.3) |
| 50 | world/sky.rs:79 | `getCloudHeight()` 8 in the sky dimension | D3-SKY | ok (WorldProvider.java:110-112; WorldProviderSky.java:45-47) | stays |
| 51 | world/sky.rs:86 | `func_28112_c()` false in the sky dimension | D3-SKY | ok (WorldProvider.java:114-116; WorldProviderSky.java:41-43) | stays |
| 52 | save/mod.rs:47 | pre-McRegion `ChunkLoader` files and `SaveConverterMcRegion.convertMapFormat` | D3-CONV | ok (SaveConverterMcRegion.java:62-171, file is 171 lines) | stays, Phase 6 |
| 53 | save/mod.rs:109 | `SaveError::Deferred` variant | — | — | — |
| 54 | save/mod.rs:121 | its `Display` | — | — | — |
| 55 | save/save_handler.rs:8 | `SaveHandler.getChunkLoader` (pre-McRegion) | D3-CONV | ok | stays |
| 56 | save/save_format_old.rs:7 | module: conversion and `SaveFormatOld.getSaveLoader` | D3-CONV | ok (SaveFormatOld.java:111-113) | stays |
| 57 | save/save_format_old.rs:11 | both return `SaveError::Deferred` | D3-CONV | ok | stays |
| 58 | save/save_format_old.rs:229 | `getSaveLoader` | D3-CONV | ok | stays |
| 59 | save/save_format_old.rs:232 | its error | D3-CONV | ok | stays |
| 60 | save/save_format_old.rs:356 | `convertMapFormat` | D3-CONV | ok (SaveConverterMcRegion.java:62-90) | stays |
| 61 | save/save_format_old.rs:359 | its error | D3-CONV | ok | stays |
| 62 | save/chunk_loader.rs:6 | the rest of `ChunkLoader` (per-chunk files) | D3-CONV | ok (ChunkLoader.java:108-200 is the ported part) | stays |
| 63 | save/region_cache.rs:22 | `SaveConverterMcRegion`'s `func_22193_a`/`func_22192_a` callers | D3-CONV | ok | stays |
| 64 | world/chunk.rs:35 | `setChunkData` (`Packet51MapChunk`) | D3-MP | ok (Chunk.java:552-596) | stays |
| 65 | worldgen/biome.rs:27 | `BiomeGenBase.sky` / `BiomeGenSky` | D3-SKY | ok (BiomeGenBase.java:21) | stays |
| 66 | worldgen/mod.rs:20 | `WorldProviderSky`, `ChunkProviderSky`, `BiomeGenSky` | D3-SKY | ok | stays |
| 67 | world/mod.rs:1915 | "deferred while the tile-entity pass runs" (`addTileEntity` queues into `field_30900_E`) | — | ok (World.java:1278-1285) | not a scope tag |

**mc-sim tests (1), PLAN_v4 (7)**

| # | Site | Says | Disposition |
|---|---|---|---|
| 68 | tests/golden_save.rs:1175 | asserts `SaveError::Deferred` for a pre-McRegion world | — (pins #52-61) |
| 69 | PLAN_v4.md:106 | D3 defines the `deferred` tag | — |
| 70 | PLAN_v4.md:142 | D5: pre-McRegion worlds deferred | D3-CONV, Phase 6 |
| 71 | PLAN_v4.md:400 | networking 72 files / 4.6k LOC deferred | D3-MP, Phase 6 |
| 72 | PLAN_v4.md:476 | WP-2.7: "`StatsSyncher` deferred per D2" | **wrong**, §2.3; fix in PLAN_v5, together with D2's own "stats sync" (PLAN_v4.md:100, a line without the word `deferred`) |
| 73 | PLAN_v4.md:505 | this WP | — |
| 74 | PLAN_v4.md:546 | brief template "Out of scope: deferred code it will meet" | — |
| 75 | PLAN_v4.md:563 | DoD 7 "deferred code skipped" | — |

Tally of the 64 tags, by each line's main subject:

| Kind | Lines | Rows |
|---|---|---|
| D2, all accurate, nothing to schedule | 12 | #2, 3, 12, 13, 21, 28, 29, 30, 32, 34, 35, 39 |
| D3-MP | 13 | #7, 15–20, 26, 27, 36, 40, 41, 64 |
| D3-TP | 8 | #1, 4, 22–25, 31, 38 |
| D3-CONV | 13 | #10, 11, 37, 52, 55–63 |
| D3-SKY | 5 | #49–51, 65, 66 |
| Stats, wrongly called D2 / D3 / "sync" / "upload" | 8 | #9, 42–48 |
| Two subjects on one line | 5 | #5 and #8 (has-paid D2 + stats), #33 (TP + D2), #6 and #14 (lists) |

The applet (PLAT) appears only inside #6.

### 2.2 D2/D3 tags that do not say "deferred" (36 lines)

These skip code the same way without the word. All 36 were read; all are
accurate.

| Area | Lines | What |
|---|---|---|
| Multiplayer only (`NetClientHandler`, packets, MP entity classes) | mc-sim entity/arrow.rs:5, fish.rs:9, fireball.rs:11 (the `(World, x, y, z, ax, ay, az)` constructor: only NetClientHandler.java:93 calls it), painting.rs:10, snowball.rs:11, data_watcher.rs:6, base.rs:69 (`outfitWithItem`), mod.rs:64, save/entity_list.rs:78, stats/mod.rs:38 (`joinMultiplayerStat`), world/map_storage.rs:40 (`updateMPMapData`), world/mod.rs:25 (`setChunkData`, `sendQuittingDisconnectingPacket`); mc-client session.rs:17 (`mpPassParameter`), settings.rs:256 (`lastServer`, kept for the file), game.rs:1071 (`GuiSleepMP` branch), gui/container/sleep.rs:3 | D3-MP, stays |
| Online skins and cloaks | mc-sim entity/base.rs:68, :200, dispatch.rs:253, player.rs:77, :81, mod.rs:194; mc-client game.rs:2044, gui/container/inventory.rs:15, :319, item_renderer/mod.rs:104, render/living.rs:265, render/mod.rs:42, render/player.rs:2, :9 | D2, stays |
| Sky dimension | mc-sim world/provider.rs:23; mc-client sky.rs:33 | D3-SKY, stays |
| Pre-McRegion | mc-sim save/mod.rs:107, save_format_old.rs:233, :360 | D3-CONV, stays |
| Texture packs | mc-client game.rs:91 | D3-TP, stays |

### 2.3 Findings on the tags

1. **Stats autosave is in scope (SCHED).** In vanilla, `runTick` calls
   `statFileWriter.func_27178_d()` every tick (Minecraft.java:949). It
   starts `ThreadStatSyncherSend` whenever two things hold:
   - the writer is dirty (`field_27189_c`, which is set by every `readStat`
     and never cleared: a vanilla bug);
   - `func_27420_b()` holds: the 100-tick countdown `field_27427_l` has run
     out and no thread is busy.

   That thread writes the unsent map to `stats/stats_<user>_unsent.dat`
   through `.tmp`/`.old` (StatsSyncher.java:99-117). No network is
   involved. So vanilla rewrites the stats file about every 5 seconds, from
   the first changed stat on, even on the title screen. The port has the
   same write as `StatFileWriter::sync_stats` and runs it only at world
   change and shutdown (game.rs:728, :854, :1518; vanilla's `syncStats` at
   Minecraft.java:420, :460, :1270).
   - **Effect:** a crash or a killed process loses every stat and
     achievement of the session. Vanilla loses at most about 5 seconds.
   - **Fix (small):** move `func_27178_d` / `func_27425_c` /
     `func_27420_b` (the countdown, and the busy flag as a no-op) into
     `StatFileWriter`, and run the send synchronously through `Storage`
     from the shell's tick, before `IngameGuiUpdateTick`.
   - **Proposed:** WP-4.7 (§3.6). Mc-sim's stats module is fixture-neutral:
     the oracle's `HarnessPlayer` counts no stats.
   - **The label comes from D2 itself.** PLAN_v4.md:100 names "stats sync"
     among the defunct online services that are out of scope. In b1.7.3
     there is no server side to it: every "sync" is a local file. So D2's
     list should drop "stats sync" (or narrow it to a server upload, which
     b1.7.3 never had). That is an amendment of an owner decision, which
     Q34 asks for.
   - **The receive half** (`ThreadStatSyncherReceive`, `stats_<user>.dat`,
     `func_27180_b`) is local file I/O too, not D2. It is dead unless a
     `stats_<user>.dat` already exists. Nothing in b1.7.3 singleplayer
     writes that file: the receive thread's write branch needs a map that
     only the receive thread itself sets, and it runs once, at
     construction. But a file can exist: `StatFileWriter`'s constructor
     relocates any `stats_*.dat` from the data directory's root
     (StatFileWriter.java:22-34), or the owner may copy one in from a
     `.minecraft`. Vanilla then reads it at start-up
     (ThreadStatSyncherReceive.java:15-16, StatsSyncher.java:26-49), and the
     per-tick `func_27425_c` folds it through `func_27180_b`
     (StatsSyncher.java:185-187, StatFileWriter.java:69-80) into the totals
     map that `hasAchievementUnlocked`, `writeStat` and the statistics
     screen read (StatFileWriter.java:174-185). The port relocates the file
     (stats/file_writer.rs:486-500) but does not merge it. Either add the
     merge as optional WP-4.7 item (6) or record it as an accepted
     divergence (Q34).
   - Relabel when that WP lands: game.rs:16, :1021-1022, gui/menu/mod.rs:42,
     stats/mod.rs:18-19, stats/file_writer.rs:29, :398, PLAN_v5's WP-2.7
     row, and PLAN_v5's D2 text (PLAN_v4.md:100).
2. **The sky dimension is unreachable in vanilla singleplayer.** The
   loading constructor picks `getProviderForDimension(-1)` for a saved
   dimension of −1 and the surface for anything else (World.java:184-190).
   No portal leads to dimension 1. Only `WorldClient` (multiplayer,
   WorldClient.java:16) asks for a provider by a server-sent id. Deferring
   it costs nothing in singleplayer parity.
3. **Pre-McRegion conversion is reachable** in vanilla singleplayer (a
   pre-Beta-1.3 save in `saves/`). The port lists such a world with "Must
   be converted!" and refuses to open it (game.rs:1470-1478). D5 import
   refuses it too. Phase 6, owner decision (Q32).
4. **Texture packs**: with no pack installed, vanilla's screen matches the
   port's except for the enabled "Open texture pack folder" button. The
   same holds for the always-enabled Multiplayer button on the title
   screen. Both are visible D3 deviations for WP-4.1's checklist (Q33).
5. **Every D2 item behaves today exactly as vanilla does now.** The
   services are gone:
   - `login.minecraft.net` never answers 400, so the has-paid lines never
     appear;
   - the skin, cloak and resource URLs on `s3.amazonaws.com` fail, so
     vanilla falls back to `/mob/char.png` and to the local `resources/`.

   So the D2 tags hide no visible difference.

### 2.4 Citation line nits (reported, not edited)

They are outside this WP's files; a cleanup pass can take them.

**WP-4.8:** the assets.rs, session.rs and sleep.rs nits are fixed as listed
(comments only; sleep.rs is `crates/mc-client/src/gui/container/sleep.rs`).
The README "Item renderer" sentence is left as it is: the README is edited
additively only in wave F, and WP-4.9 rewrites that paragraph (anaglyph's
second pass); its conclusion stands.

- assets.rs:9 `TexturePackBase.java:27-29` → 28-29. assets.rs:12
  `TexturePackDefault.java:7-13` → the constructor is 13-18.
- assets.rs:36 `TexturePackDefault.java:11` → 18 (line 11 is the field
  declaration; the `/pack.png` read is :18). assets.rs:38
  `LoadingScreenRenderer.java:145` → 80 (the file has 123 lines).
  assets.rs:41 `TexturePackDefault.java:28` → 40 (`/gui/unknown_pack.png`).
- session.rs:13/:16/:19 `Session.java:7/:8/:9` → 8/9/10; :25
  `Session.java:11-14` → 12-15.
- sleep.rs:3 `Minecraft.java:978-983` → 975-979.
- README "Item renderer": `cameraZoom` is "only changed by the unused
  high-resolution screenshot". In fact nothing in b1.7.3 assigns it
  (EntityRenderer.java:38 is the only write). The conclusion, that there is
  nothing to port, stands.

## 3. Recorded gaps that are not `deferred` tags

### 3.1 The `note_unimplemented` sites in `crates/mc-sim/src`, by Java class

"Reachable" means reachable in the shipping singleplayer game. The client
drains no `unimplemented_hits`, so a reached gap is silent there (§3.6,
WP-4.8 item 1). **WP-4.8:** `dump_debug` now writes the open world's
`skipped_systems` and `unimplemented_hits` (`gaps`, crates/mc-client/README.md),
and every tick or world-read fixture writes them to `gaps.json` next to its
`actual.jsonl` (tests/golden/README.md, "The gaps file"). The counts below
are WP-4.4's; WP-4.8 removed 20 sites (B: 14, D: 6), leaving 76.

**A. Parity gaps (10 sites).** All are reachable, and none changes a
current fixture.

| Java | Site | Effect in play | Owner |
|---|---|---|---|
| `BlockCake.blockActivated` → `EntityLiving.heal`'s `heartsLife = heartsHalvesLife / 2` (EntityLiving.java:303) | block/cake.rs:129 | After a hit, `heartsLife` is 11–20 for about 10 ticks. Vanilla's heal drops it to 10, so the next hit takes the full branch of `attackEntityFrom` (EntityLiving.java:317-331); the port keeps the reduced branch. The HUD's heart blink (`heartsLife / 3 % 2`, off below 10, GuiIngame.java:59-60) differs. Gate 2 §4.2 called this "probably unobservable"; it is observable | WP-4.6 (R2) |
| `EntityLiving.onDeath` → `EntityPlayer.addToPlayerScore` | entity/living.rs:772 | `mobKillsStat` / `playerKillsStat` never counted (EntityPlayer.java:239-247); `score` stays 0, as it would anyway (every `scoreValue` is 0) | WP-4.6 (R2) |
| `EntityLiving.onDeath` → `EntityPlayer.onKillEntity` | entity/living.rs:778 | **The `killEnemy` achievement ("Monster Hunter") can never be earned** (EntityPlayer.java:795-799). Visible since WP-2.7 counts stats. docs/mappings.md:345 still says "the gap changes no state"; that was true before WP-2.7 | WP-4.6 (R2) |
| `EntityPig.onStruckByLightning` | entity/pig.rs:102 | A struck pig dies; no zombie pigman joins, so the entity id stream shifts | WP-4.6 (R2) |
| `EntityPlayer.alertWolves`: the tamed branch | entity/player.rs:836, :853 | Tamed wolves never defend their owner (EntityPlayer.java:402-437) | WP-4.6 |
| `EntityFireball.getCollisionBorderSize` (1.0F, EntityFireball.java:198-200) | entity/dispatch.rs:795 | **New.** The client's `getMouseOver` calls the sim dispatch (camera.rs:567), which records the gap and returns the base 0.1. A ghast fireball is therefore picked through a box expanded by 0.1 instead of 1.0, so punching it back (`attackEntityFrom` sends it along the look) is much harder than in vanilla. Gate 2 gave this method to WP-3.1; it was not done there, and fireball.rs:10-12 still calls it "client". One match arm | **add to WP-4.6** |
| A projectile's shooter that has left `loadedEntityList`: `attackEntityFrom(this.owner, 4)` (EntityArrow.java:163; the fireball's `shootingEntity` likewise, EntityFireball.java:124) | entity/arrow.rs:312 (`attack_entity_by_id`, shared by all five projectiles) | **New; moved here from the defensive guards.** Java still holds the shooter object; the port passes no attacker. **Repro:** switch to Peaceful while a skeleton's arrow is in flight. Vanilla removes the skeleton (EntityMob.java:20-24), the arrow still passes it, `EntityPlayer.attackEntityFrom` sees an `EntityMob`, scales the damage to 0 and returns false (EntityPlayer.java:366-381), and the arrow bounces back (EntityArrow.java:166-173). The port skips the scaling (player.rs:767-778), deals 4 damage and kills the arrow with `random.drr`. On Easy the port deals 4 where vanilla deals 2; on Hard, 4 where vanilla deals 6 (Normal agrees). With no attacker, `EntityLiving.attackEntityFrom` also skips the knockback and draws `Math.random` once for `attackedAtYaw` instead of the `atan2` path (EntityLiving.java:337-349, `knockBack` at :346), which shifts the shared `Math.random` stream, and `alertWolves` is not called. The same happens when the skeleton is killed (removed at `deathTime > 20`, EntityLiving.java:157-159) or despawns while the arrow flies, and when a ghast is removed under its fireball (EntityGhast.java:32, damage 0: only knockback and the `Math.random` draw differ) | **proposed WP-4.7** item (5) (mc-sim behaviour; the snapshot of WP-4.8 item 2 is the general remedy) |
| `EntityLiving.updatePlayerActionState`: `faceEntity(currentTarget)` before the `isDead` check (EntityLiving.java:706-710) | entity/living.rs:324 (`EntityView::of`, via living.rs:1096) | **New; moved here from the defensive guards.** An idle mob looking at the player (`currentTarget`, set at EntityLiving.java:697-700) when the dead player leaves the list 20 ticks after death (EntityLiving.java:157-159): vanilla turns the mob toward the removed player's last position once more, then drops it as `isDead`; the port finds no view, skips the turn and drops it. One tick of the mob's yaw and pitch differs, and its walk direction follows from them. Rare and invisible behind the death screen, but a simulation divergence | **proposed WP-4.7** item (5), with the projectile shooter |
| `Container.slotClick` over a tile entity that has left the world (TileEntityChest/Furnace/Dispenser) | inventory/container/mod.rs:115 (`te_gone`, called from :144, :170, :211, :253, :366) | **New; moved here from the defensive guards.** **Repro:** an explosion destroys a chest, furnace or dispenser whose window is open. The port closes the window in tick N+1's screen hook, *after* `handle_input` (gui/mod.rs:344-351; the README's close-timing deviation, §3.2), so clicks queued since tick N still reach `slot_click`, which records `te_gone` and does nothing. Vanilla depends on the order within tick N's `updateEntities`: if the explosion came before the player's update, `isUsableByPlayer` failed and the window closed in tick N (EntityPlayer.java:80-84), so tick N+1's input goes to the game; if it came after, tick N+1's `handleInput` (Minecraft.java:987-991, before `updateEntities` at :1160) clicks the orphaned `TileEntityChest`, whose stacks `onBlockRemoval` left at `stackSize` 0 (BlockChest.java:155-182), before the player's update closes the window. The site's own comment (container/mod.rs:111-113, "unreachable from the client, which closes the window") is wrong | **proposed WP-4.7** item (2) (fixing the close timing removes the first case); the comment is WP-4.6's (§3.4) |

**B. Client-only, dead in the shipping game (14 sites).** The client
implements these itself and never calls the sim's versions.

| Java | Sites | Client implementation |
|---|---|---|
| `ColorizerGrass.getGrassColor`, `ColorizerFoliage.getFoliageColor` (via the sim's `colorMultiplier`) | block/grass.rs:88, leaves.rs:131 | `terrain/blocks.rs:327`, `particles/subclasses.rs:50` with `textures::Colorizers` |
| `isInRangeToRenderVec3D` (LightningBolt), `isInRangeToRenderDist` (Egg, Fireball, Fish, Snowball) | entity/dispatch.rs:664, :673-676 | `render/mod.rs:597` |
| `getShadowSize` (Arrow, Egg, FallingSand, Fireball, Fish, Snowball, TNTPrimed) | entity/dispatch.rs:702, :705-710 | `render/shadow.rs:22` |

They can go, or the sim arms can return the Java constants (WP-4.8 item 4).

**WP-4.8: closed, by deleting the sim's arms.** §3.6 proposed removing them,
and that is what fits the tree: the client already has every override,
transcribed with citations and unit tests (`render::shadow::get_shadow_size`,
`render::is_in_range_to_render_vec3d`, `render/tests.rs::in_range_overrides`),
and the simulation never calls these methods on a tick path. Making the
client call the sim instead would have moved render data into mc-sim and
changed client files WP-4.9 is editing, for no gain. So
`entity::dispatch::{get_shadow_size, is_in_range_to_render_vec3d,
is_in_range_to_render_dist}` are gone; the three methods are in
`dispatch::NOT_DISPATCHED` with the reason (the coverage test checks it),
`getShadowSize` left the Boat and Minecart rows of `TRANSCRIBED`, and
`base::is_in_range_to_render_vec3d` calls `Entity`'s own
`isInRangeToRenderDist` (the base bodies stay: the client uses
`base::is_in_range_to_render_dist`). The colorizer sites keep their arm:
`BlockGrass`/`BlockLeaves.colorMultiplier` return what the headless Java
returns before the client installs a colormap (0, also the oracle's value),
which is the whole Java body for a world without a client, so they no longer
record anything. The shipping client tints with its own copy and never calls
them.

**C. Multiplayer only (5 sites)**: `setVelocity` (Arrow, Egg, Fish,
Snowball; entity/dispatch.rs:825, :828, :829, :832) and
`setPositionAndRotation2` (Fish, :777). Only NetClientHandler.java:133,
:168, :212, :227 calls them. D3, stays.

**D. Dead code (6 sites)** (Gate 1 N2/N3, Gate 2 R4):
- tile_entity/mod.rs:200, :269, :297: `TileEntityKindData::Untranscribed`
  is never constructed.
- save/entity_list.rs:287: `construct_gap`. `missing_constructors()` is
  empty, as squid_spawning.rs:217 asserts.
- entity/nbt.rs:458: the `readEntityFromNBT` fallthrough. Gate 2 said
  "probably unreachable; not verified". This review checked it: each of the
  28 `EntityKindData` variants has its own arm, so it is **unreachable**.
- block/dispatch.rs:680: `untranscribed()`, reached only through the five
  all-`None` tails (§1).

**WP-4.8: all removed.** The five tails and `untranscribed()`;
`TileEntityKindData::Untranscribed` and its three arms; `EntityList`'s gap
path (`FromNbt::Gap`, `construct_gap`, the two gap records of
`EntityMapping`, `preserved()`) with the chunk loaders' preserved-compound
plumbing (`LoadedChunk::preserved`, `McRegionChunkLoader::preserved`,
`store_chunk_in_compound`'s parameter); `missing_constructors()` stays as the
invariant the tests assert, and `create_entity_from_nbt` skips an id with no
constructor row (none exists). The `readEntityFromNBT` fallthrough is an
exhaustive empty arm for the kinds read before the `match`.

**E. Defensive guards (62 sites)**: the take-out/put-back pattern and Java
exception paths. None is a work item on its own. (The first version of this
document counted 65; arrow.rs:312, living.rs:324 and
inventory/container/mod.rs:115 moved to A after its review.)

| File | Sites |
|---|---|
| world/entities.rs | 52, 282, 507, 629, 633, 724, 728, 753, 771, 818 |
| entity/wolf.rs | 338, 398, 552, 662, 686, 728, 757 |
| entity/base.rs | 1096, 1198, 1209, 1228, 1248 |
| entity/fish.rs | 260, 269, 295, 503 |
| entity/player.rs | 537, 798, 1108, 1298 |
| spawner.rs | 127, 383, 399, 424 |
| entity/boat.rs | 168, 322, 474 |
| entity/painting.rs | 306, 407, 497 |
| entity/arrow.rs, minecart.rs, pressure_plate.rs, tile_entity/mob_spawner.rs | 324; 303, 1065; 189, 193; 166, 216 |
| one each | creature.rs:110, ghast.rs:202, item.rs:222, lightning_bolt.rs:160, mob.rs:182, nbt.rs:129, pig.rs:122, skeleton.rs:87, spider.rs:76, item/behaviour/projectiles.rs:110, save/chunk_loader.rs:118, save/entity_list.rs:301, tick.rs:98, world/explosion.rs:345, world/mod.rs:2023 |

Three families:

1. **Java exceptions** (9): the "Java NullPointerException" and
   `ClassCastException` paths in item.rs, player.rs:537, nbt.rs:129,
   fish.rs:260, painting.rs and mob_spawner.rs. Vanilla would crash or log
   here; the port skips. This is not a parity concern.
2. **Taken-out entity** (Gate 2 R5), 46 sites: a query meets an entity
   that is out of its slot, so the port leaves it out where Java would test
   it.
   - 11 passing fixtures reach it, and none changes a value.
   - The risk is a case where the skipped entity would have matched.
   - Unscheduled: §3.6, WP-4.8 item 2.

   The one-each members were traced site by site (the first version of this
   document classified the family as a block, and
   inventory/container/mod.rs:115 turned out to be reachable; it is now in
   A):
   - **creature.rs:110**: unreachable. The target is the player
     `findPlayerToAttack` has just returned from `playerEntities`, and no
     mob updates nested inside a player's update: the port puts a mount back
     in its slot before it updates the rider (world/entities.rs:470-471).
   - **ghast.rs:202**: unreachable. A removed target is `isDead`, and the
     ghast drops a dead target before reading it (EntityGhast.java:61-63,
     ghast.rs:176-180); its target is always a player (EntityGhast.java:66).
   - **mob.rs:182, skeleton.rs:87, spider.rs:76**: unreachable.
     `attackEntity` runs only after `EntityView::of` found the target in its
     slot (creature.rs:142-148).
   - **pressure_plate.rs:189**: believed unreachable. During a plate's
     query the only entities out of their slots are the colliding entity
     (passed in as `in_hand`) and an attack target inside
     `attackEntityFrom`, which neither moves nor ticks a plate. The first
     message (a chunk list holding an entity not in `loadedEntityList`)
     cannot happen: dead and unloaded entities leave both lists together
     (World.java:1189-1231).
   - **save/chunk_loader.rs:118**: unreachable. Singleplayer's
     `ChunkProvider` (World.java:212) saves only from `saveChunks` and
     `unload100OldestChunks` (ChunkProvider.java:130-167), which run from
     `World.tick` and `saveWorld` (World.java:1762, :1774), never inside an
     entity's update.
   - **world/entities.rs:771**: reachable, with no effect. On the death
     screen, more than 20 ticks after death, the dead player has left the
     list (EntityLiving.java:157-159), and `joinEntityInSurroundings` runs
     every 30 ticks (Minecraft.java:1134-1139). Vanilla appends the dead
     player again (World.java:2305-2307), and the same tick's
     `updateEntities` drops it, since it is `isDead`, after every other
     entity has updated and before `World.tick`; the 5 × 5 chunks it loads
     are already loaded. The port records the gap and returns.
   - **tick.rs:98, world/mod.rs:2023, lightning_bolt.rs:160, pig.rs:122**:
     unreachable. The sleep check (World.java:1748) and the level save run
     outside entity updates; the bolt strikes entities its own query found
     in their slots; a pig's `fall` runs in the pig's own update, with its
     rider in its slot.
   - **world/explosion.rs:345**: believed unreachable, as the plate. The
     entity whose update set off the explosion is passed in (`hid`), and
     no other entity is out of its slot when an explosion runs.
   - **save/entity_list.rs:301**: not a taken-out entity. A saved entity
     whose id is outside `EntityList`'s table; vanilla skips it too
     (`createEntityFromNBT` returns null), so nothing differs but the
     record.

   The multi-site files (world/entities.rs, wolf.rs, base.rs, fish.rs,
   player.rs, spawner.rs, boat.rs, painting.rs, minecart.rs,
   mob_spawner.rs, arrow.rs:324, pressure_plate.rs:193) were not traced
   site by site. WP-4.8 item 2 should trace them before it picks a remedy.
3. **Removed-entity references**, 7 sites. Java keeps an object after it
   leaves `loadedEntityList`; the port keeps an id. The sites:
   - player.rs:798 and wolf.rs:662: the owner of an arrow passed *as* the
     attacker. **Unreachable in b1.7.3:** no caller passes an arrow entity
     to `attackEntityFrom`. All five projectiles pass their shooter
     (EntityArrow.java:163, EntitySnowball.java:153, EntityEgg.java:153,
     EntityFireball.java:124, EntityFish.java:225), so the
     `instanceof EntityArrow` branches of EntityPlayer.java:384 and
     EntityWolf.java:269 never run. The reachable form of this gap, the
     shooter passed directly, is arrow.rs:312 in A;
   - fish.rs:503 and projectiles.rs:110: the bobber;
   - world/entities.rs:52/282/507: the dead-or-gone check and a rider's
     mount.

   The projectile shooter (arrow.rs:312) and the look target
   (living.rs:324) were in this family in the first version of this
   document; both are parity gaps reachable in singleplayer and are now in
   A. Gate 2 R5 suggests one remedy for the rest of this family and for
   family 2: keep a snapshot (class, position, box) of entities that are
   taken out or removed while something still refers to them. WP-4.8 item
   2.

**Update (WP-4.8): the multi-site files traced site by site.** Line numbers
are at the WP-4.8 tip. "Out" means taken out of its slot further up the
stack; "removed" means no longer in `loadedEntityList` (WP-4.6 keeps a removed
entity in `World::removed_entities` while a loaded entity's field names it,
`World::entity_object`). "Snapshot?" answers the brief's question: does
parity need a copy of the out or removed entity at this site?

Method: every site was read against its Java; every caller chain that can
hold an entity out of its slot was listed (an entity's own update from
`updateEntities`; its rider's update after the mount is put back; the
client's player during a click, `use_item`, `attack`, `interact`; the
target of a `with_entity_mut` inside those); and `gaps.json` was collected
for all 230 default and 5 soak fixtures, with a temporary backtrace at the
two R5 records to name each caller. The reachable cases were then run in
the oracle as throwaway scenarios before any fix.

| File:line (WP-4.8) | Java | What is out / removed | Reachable in SP? | Snapshot? | WP-4.8 |
|---|---|---|---|---|---|
| world/entities.rs:56 `is_entity_dead_or_gone` | `x.isDead` on a field | an id neither loaded nor kept | no: every entity-typed field keeps its referent (`referenced_entity_ids`) | already (WP-4.6) | — |
| world/entities.rs:334, :563 (`updateEntities`, `updateEntityList` mount check) | World.java:1210-1217, :2339-2346 | the rider's mount, removed earlier in the pass (it died: `Entity.setEntityDead` does not dismount; a pig killed under its rider, a cart broken by an arrow) | **yes**; same outcome as Java (`mount.isDead`, dismount), but recorded | the removed object, kept already | **neutral fix**: read through `entity_object` (no record; the removed mount's `riddenByEntity` cleared as in Java). The one case whose outcome changes, a mount unloaded while its rider stays loaded (Java keeps the rider on it), now matches Java; rider and mount share a chunk column, so it is not reached |
| world/entities.rs:713, :811 "Chunk entity list holds …" | — | invariant | no: dead and unloaded entities leave both lists in one pass (World.java:1189-1231) | no | — |
| world/entities.rs:717 `getEntitiesWithinAABBExcludingEntity` over an out entity (R5) | World.java:2013-2029 | whoever the caller's chain holds | **yes** (3 default fixtures, the 5 soaks): only the placement check `World.canBlockBePlacedAt` → `checkIfAABBIsClear` meeting the `use_item` player; the placing item tests the player's box first (`item::behaviour::can_block_be_placed_at`), so nothing changes. The other callers run in an entity's own update with only that entity out (excluded): projectiles, boat, cart, `collideWithNearbyEntities`, the player's pickup scan, lightning, the piston push, `getCollidingBoundingBoxes`, `getCanSpawnHere`; the painting's `onValidSurface` with the player out keeps only `EntityPainting`s | no (box needed only if a match were possible; none found) | `pig_zombie` moved to the class query (below) |
| world/entities.rs:819 `getEntitiesWithinAABB(class)` over an out entity | World.java:2031-2047 | as above | before WP-4.8: 10 fixtures, all `alertWolves` (class `EntityWolf`) meeting the out player or arrow | no | **neutral fix**: `EntitySlot::class_chain` keeps each entity's Java class chain, so an out entity of another class is skipped as Java's `isAssignableFrom` skips it, without a record. Left recorded: an out entity of the queried class. Traced callers: the wolf scans (the scanning wolf is in hand; the attacker of a calm wild wolf cannot be a wolf), `alertWolves` (attacker in hand), plates and detector rails (collider in hand), the mob spawner (no entity out). None reachable |
| world/entities.rs:843 `countEntities` | World.java:2064-2075 | — | no: `performSpawning` runs from `World.tick` | no | — |
| world/entities.rs:861 `joinEntityInSurroundings` | World.java:2294-2309 | the dead player, removed 20 ticks after death | **yes**, every 30 ticks on the Game Over screen; no effect (vanilla re-adds the dead object and the same `updateEntities` drops it again: one `releaseEntitySkin`, which loads nothing, D2) | no | — (WP-4.7's P4 dead-player work is next to it) |
| world/entities.rs:908 `updateAllPlayersSleepingFlag` | World.java:2381-2393 | another player | no: one player in SP; the dying player has left `playerEntities` first | no | — |
| entity/wolf.rs:338 owner out | EntityWolf.java:96-123 | the owner | no: `getPlayerEntityByName` only returns a player in its slot (it skips an out one silently, as `getClosestPlayer` does), and a wolf never updates inside the player's update | no | — |
| entity/wolf.rs:398 look target out | EntityWolf.java:125-150 | `currentTarget`, a player | no | no | — (see "the reused id" below) |
| entity/wolf.rs:552 path target out | EntityWolf.java:229-248 | the owner from :338 | no | no | — |
| entity/wolf.rs:662 arrow's owner | EntityWolf.java:269 | — | no: no caller passes an arrow as the attacker (family 3) | no | — |
| entity/wolf.rs:686 a wolf in range out | EntityWolf.java:272-283 | — | no: the ids come from the query, which returns only entities in their slot or in hand | no | — |
| entity/wolf.rs:728, :757 attack target out | EntityWolf.java:304-324 | — | no: `EntityCreature` calls `attackEntity` only after finding the target in its slot | no | — |
| entity/base.rs:1096 `updateRidden` mount out | Entity.java:916-968 | — | no: the rider updates after its mount is put back | no | — |
| entity/base.rs:1198, :1209 `mountEntity` old mount / old rider out | Entity.java:996-1007 | — | no: the callers are the new mount's own `interact`, `attackEntityFrom`, `applyEntityCollision` (target in hand) and the spider jockey (not joined yet); the old mount or rider is in its slot | no | — |
| entity/base.rs:1228 `mount_entity_in_world` | — | — | no caller outside the tests | no | — |
| entity/base.rs:1248 `mountEntity(null)` mount out | Entity.java:983-991 | — | no: `World.setEntityDead` dismounts the rider with the mount in hand (`dismount_from`); `sleepInBedAt` has the mount in its slot | no | — |
| entity/fish.rs:260 | EntityFish.java:146 | — | Java NPE (family 1) | — | — |
| entity/fish.rs:269 angler out | EntityFish.java:145-150 | — | no: the bobber updates from `updateEntities`; a removed angler reads as dead, as in Java | no | — |
| entity/fish.rs:295 hooked entity out | EntityFish.java:152-161 | — | no | no | — |
| entity/fish.rs:503 `catchFish` hooked entity gone or out | EntityFish.java:348-360 | the hooked entity | in principle: the bobber may hook its own angler after 5 ticks in the air (EntityFish.java:205), and the angler is in the rod's hand; a straight-up cast in the oracle fell past the player, so not reproduced. The effect would be a small pull on the player | no (the angler is the `angler` parameter) | — (one `if` when reproduced) |
| item/behaviour/projectiles.rs:122 `catchFish` on a bobber not in the list | ItemFishingRod.java:19-22 | the bobber, removed | **yes**: a bobber in the ground dies after 1200 ticks without clearing `angler.fishEntity` (EntityFish.java:168-176); the next click reels in the dead object, 2 rod damage; the port recorded and did 0 (oracle: 2 vs 0) | yes: the removed object, kept already | **fixed**: `catchFish` on `World::removed_entities`' copy; `entity/fishing-rod-dead-bobber` |
| entity/boat.rs:173 `attackEntityFrom` rider out | EntityBoat.java:69-72 | the rider = the attacker, held by its own click | **yes**: punch the boat you ride until it breaks (3 fist hits); vanilla puts you on top of it, the port left you on the dead boat and dropped you in place next tick (oracle: position differs from t = 10) | no: the rider is in the caller's hand | **fixed**: the boat leaves it to `player_controller::attack_entity`, which runs `mountEntity` right after the attack (the order is not observable: nothing in between reads the player); `entity/boat-broken-by-rider` |
| entity/boat.rs:327 `onUpdate` rider out | EntityBoat.java:195-198 | — | no: the boat updates from `updateEntities` | no | — |
| entity/boat.rs:479 `interact` rider out | EntityBoat.java:336-346 | — | no: the other rider is in its slot | no | — |
| entity/minecart.rs:309 `attackEntityFrom` rider out | EntityMinecart.java:79-82 | as the boat | **yes**, as the boat (oracle: the player's y differs at the break) | no | **fixed** as the boat; `entity/minecart-broken-by-rider` |
| entity/minecart.rs:1071 `interact` rider out | EntityMinecart.java:763-771 | — | no | no | — |
| entity/painting.rs:306, :407, :497 | — | — | Java NPE (family 1) | — | — |
| tile_entity/mob_spawner.rs:163, :213 | TileEntityMobSpawner.java:51-80 | — | Java `ClassCastException` / an unjoined mob (family 1); a non-living spawner id needs an edited world | — | — |
| spawner.rs:127, :424 | SpawnerAnimals.java:27-28, :175-185 | the player | no: both run from `World.tick` | no | — |
| spawner.rs:383, :399 | Pathfinder over unloaded chunks | — | believed no (§3.1, below) | no | — |
| block/pressure_plate.rs:189, :193 | BlockPressurePlate.java:75-88 | — | no: `moveEntity` runs only in the moving entity's own update (passed in hand); knock-back, pulls and pushes change motion only; the scheduled tick runs from `World.tick` | no | — |
| entity/arrow.rs:322, :334 (WP-4.7's file) | EntityArrow.java:163 | the shooter or the target | no: the projectile updates from `updateEntities`; a removed shooter is kept (WP-4.6) | already | — |
| entity/player.rs:537, :947 | — | — | Java NPE (family 1) | — | — |
| entity/player.rs:641, :645 (WP-4.7's file) | EntityLiving.java:407-414 | the removed attacker player | yes, recorded by WP-4.6 (WP4.6_SIM_GAPS.md §3) | already | WP-4.7 |
| entity/player.rs:902 arrow's owner | EntityPlayer.java:384 | — | no (family 3) | no | — |
| entity/player.rs:978 a wolf out | EntityPlayer.java:402-437 | — | no: the attacker is in hand and skipped | no | — |
| entity/player.rs:1260 another player out | World.java:2381-2393 | — | no: one player | no | — |
| entity/player.rs:1450 `addMountedMovementStat` mount out | EntityPlayer.java:766-785 | — | no: the rider updates with its mount in its slot | no | — |
| world/explosion.rs:345 (one-each in the first version: "believed unreachable") | Explosion.java:77-105 | the player that clicked a nether bed | **yes**: the blast's scan skipped the clicking player, who took no damage; vanilla's player next to the bed dies (oracle: health −28 at t = 1, the port's 20) | no: the player is the click's `&mut` | **fixed**: `BlockBed.blockActivated` runs `newExplosion`'s body with the player in hand, as the TNT and the fireball pass themselves; `tick/bed-nether-player-near`. No other explosion has a second entity out |

What WP-4.8 changed (all in its files; the four fixes and their fixtures
are one commit, "step 7"):

- **Neutral**: `EntitySlot::class_chain` and its use in the class query;
  `EntityPigZombie.attackEntityFrom`'s anger scan on the class query (same
  ids in the same order, the pigman's own id dropped); the mount checks
  through `entity_object`. Unit tests: `crates/mc-sim/tests/gap_hygiene.rs`.
- **Behaviour (oracle-verified gaps)**: the bed explosion, the boat and
  minecart broken by their rider, the rod on a dead bobber. Each new fixture
  failed on the port before its fix (first difference as in the table) and
  passes after; every other `expected.jsonl` is unchanged.

Found and **not** changed (proposals):

1. **The reused player id (family 3).** On respawn the new player gets the
   dead player's `entityId` (Minecraft.java:1451, :1468), and the port's
   fields hold ids, so every field that named the dead player now names the
   respawned one; Java's fields still hold the dead object. Most fields drop
   a dead target long before a respawn (`EntityCreature` drops a target that
   is not `isEntityAlive`, EntityCreature.java:24-25; the ghast and the
   look target drop `isDead`; the arrow pickup tests
   `doesArrowBelongToPlayer`, not the owner, EntityArrow.java:257-266). What
   lasts is a projectile's owner: an arrow, snowball or egg the player threw
   before dying that hits after the respawn is attributed to the respawned
   player (the victim's knock-back direction, `EntityMob`'s revenge target,
   EntityMob.java:37, and the kill hooks, WP4.6_SIM_GAPS.md §3), where Java
   attributes it to the dead object (dropped at the victim's next AI tick).
   Rare (the projectile must outlive the Game Over screen) and not verified
   in the oracle: the harness has no respawn op. Proposed remedy, in
   world/entities.rs: when an entity joins with the id of a kept removed
   one, move the removed object to a fresh negative "ghost" id and rewrite
   the fields that named it (`referenced_entity_ids`' fields) to the ghost
   id, so they keep pointing at the dead object. It touches the respawn
   path WP-4.7 is changing (P4), so it is left to a follow-up after WP-4.7
   merges.
2. **The bobber hooking its own angler** (fish.rs:503, above): one `if`
   once a scenario reproduces it.

The two `SpawnerAnimals.performSleepSpawning` "Pathfinder read of an
unloaded chunk" records (spawner.rs:383, :399) are believed unreachable.
The random-tick pass keeps the 19 × 19 chunks around each player loaded
(World.java:1868-1885), and the pathfinder reads within about 64 blocks.

**Also outside `note_unimplemented`:**
- **`World::skip`** (world/entities.rs:185) is an invariant check, and its
  doc (:177-179) is stale (Gate 1 N4). **WP-4.8:** the doc now says so.
- **mc-golden** tick_driver.rs:621 is harness-only.

### 3.2 mc-client README "Known deviations" and the Gate 3 §4 gaps

| Gap | Source | Reachable | Status / owner |
|---|---|---|---|
| D1 third-person held items not drawn (`render::held_item::HeldItemHook` never inserted; `ItemRenderer::player_model_drawn` not called from the third-person draw) | Gate 3 §3; README "Integration status" | yes | scheduled **WP-3.9** |
| D2 achievement/statistics block icons drawn flat | Gate 3 §3; README "Item icons" | yes | scheduled **WP-3.9** |
| Container close timing: `isUsableByPlayer` close runs at the start of tick N+1 (EntityPlayer.java:80-84) | README "Known deviations" | yes. (a) Walk away from an open chest: the dropped cursor stack gets one `EntityItem` update fewer, and its id comes after tick N's spawns. (b) **A whole tick of input still reaches a container vanilla has already closed.** The port closes in `update_screen`, which runs after `handle_input` in tick N+1's screen hook (gui/mod.rs:344-351). Vanilla closed the window inside tick N's `updateEntities` and routes tick N+1's `handleInput` (Minecraft.java:987-991) to the game. So chest clicks in the tick after walking out of range still move items between the chest and the inventory, which vanilla never allows, and that tick's key and mouse events (a number key, Q, a click) miss the game. (c) A container whose tile entity an explosion removed (§3.1 A, container/mod.rs:115) | **unowned**. Needs `craftingInventory` in mc-sim's player or a shell hook after the player's update; proposed **WP-4.7** with WP-4.6 for the mc-sim side |
| A screen opened by a click opens at the next tick hook; the rest of that tick's input goes to the game (Minecraft.java:995-1126) | README | yes (a number key or Q in the same tick as the right click on a chest) | **unowned**; proposed **WP-4.7** (`Game::process_input` sets `current_screen` after each controller call that raises a GUI) |
| Portal with a workbench open: vanilla drops the 3 × 3 grid twice (EntityPlayerSP.java:45-47 skips `closeScreen`; the second `EntityItem`s share the first ones' `ItemStack`s); the port drops once | README | yes (rare) | **accepted divergence** of value-typed stacks unless the owner wants it emulated (Q31) |
| Chest minecart cannot be opened (`InventoryRef` has no entity variant) | README; Gate 3 §4 | yes | scheduled **WP-4.6** |
| Inventory preview: no hurt overlay, shadow or fire; duplicates `RenderPlayer` code that WP-3.5b has | README; Gate 3 §4 | yes (cosmetic) | **unowned**; candidate for WP-3.9 or wave F (WP-4.1 will list it) |
| No occlusion queries (F3 `O: 0`) | Gate 3 §4 | always | accepted (wgpu has none); a performance item at most, WP-4.3 |
| `f32` world-space transforms: jitter far from the origin, no floating origin | Gate 3 §4 | far from 0,0 (e.g. the soak site at 20008, −15000) | **unowned**. WP-4.1 should look at a far coordinate; a fix is a client rendering WP |
| Pass-1 (translucent) sections sorted by Bevy, not nearest first | Gate 3 §4 | fast graphics only | **unowned**, low |
| `isChunkLit` scoped to the mesher's snapshot | Gate 3 §4 | edge of the snapshot | **unowned**, low |
| Anaglyph 3D draws one eye | Gate 3 §4; README | anaglyph option | **unowned**, low |
| `cameraZoom` ignored | Gate 3 §4 | never: b1.7.3 never assigns it | nothing to do (§2.4) |
| Audio: `ClientEvent`, `ClientSound` and simulation sounds are three streams per frame, not interleaved in call order; `playSound`'s range check uses the view entity's end-of-frame position | README "Audio"; Gate 3 §4 | frames that run more than one tick | accepted (README explains; can change a clock-seeded variant pick or a sound at exactly 16 blocks) |
| Panning approximates OpenAL's | README "Audio" | always | accepted; WP-4.1 to listen |
| Audio output not checked at Gate 3 | Gate 3 §4 | — | **WP-4.1** |
| Import is a typed path field | Gate 3 §4, Q6 | — | owner Q22 |
| Walking into a real nether portal not exercised | Gate 3 §4, O2 | — | scheduled **WP-3.9** |
| `click_slot` dev action (O1), `give`/`give_item` merge (N3), README stale note (N1), `mappings.md` duplicates (N2) | Gate 3 §3 | — | scheduled **WP-3.9** |
| Import refusal log line names the staging path (O4) | Gate 3 §3 | — | **unowned**, cosmetic |

### 3.3 Gate 0–2 divergence and reviewer lists: current status

| Item | Status now |
|---|---|
| Gate 0 §6 oracle gaps (unseeded constructor draws; dispenser/explosion Randoms; left-click breaking, portal and `pushOutOfBlocks` need `Minecraft`) | Constructor draws: K11. Randoms: K4. Breaking: the harness `dig` op (Gate 2 §5.2). Portal/`pushOutOfBlocks`: K12 |
| K1, K2 spawner/mob-spawner constructors | closed (WP-2.3c). spawner.rs:24-32 "Recorded gaps / Known divergence" is stale (Gate 2 R3, still there) |
| K3 `randomDisplayUpdates`' Random is one stream | by design (D9) |
| K4 clock-seeded Randoms seeded instead | by design. The shipping game seeds them from the clock (WP-3.1b `seed_clock_randoms`, game.rs "Seeding"), which answers Gate 1 Q5 |
| K5 lightning bolt | closed (WP-2.4a); its draws are K11 |
| K6 unconstructible saved entities | unreachable; dead code, §3.1 D |
| K7 eager region decompression | accepted |
| K8 `use_item` | closed (WP-2.6) |
| K9 oracle `Math` == `StrictMath` | owner Q2 |
| K10 hostile spawning unseeded in the oracle | owner Q1; the soaks run at difficulty 0 with thunder held off for the same reason (WP-4.2) |
| K11 irreproducible constructor draws | owner Q1; unit tests only |
| K12 `EntityPlayerSP` client parts off in golden runs | owner Q16; unit tests only |
| K13 stats counted by the port, never by the oracle | open; unverified against vanilla except the registries. Add §2.3's autosave |
| K14 reload keeps the old provider | owner Q15 (harness only) |
| K15 Gate 2's real gaps | pickups closed (WP-2.H); `setInPortal` closed (WP-3.1b); the rest is §3.1 A → WP-4.6 |
| Gate 1 N1 / Gate 2 R6 "every fixture has a test" checks missing for `math`, `block`, `item`, `recipe`, `stat` | **closed (WP-4.8)**: `every_<area>_fixture_has_a_test` in mc-core `math_sweep_report.rs` and `golden_{block,item,recipe,stat}_table.rs` (`mc_golden::fixture_names`), and mc-golden `runner.rs::every_fixture_area_has_a_coverage_check` for a new area |
| Gate 1 N2 / R4 five unreachable `untranscribed` tails | **closed (WP-4.8)**, §3.1 D |
| Gate 1 N3 / R4 `TileEntityKindData::Untranscribed` | **closed (WP-4.8)**, §3.1 D (with `FromNbt::Gap`, `construct_gap` and the preserved compounds) |
| Gate 1 N4 stale `add_loaded_entities` doc | **closed (WP-4.8)** |
| Gate 1 N5 `save/testworld` compares one chunk | informational; Q11 |
| Gate 1 N6 cross-read tests pass silently without `build/oracle` | **closed (WP-4.8)**: `#[ignore = "needs the oracle build …"]`; run with `--ignored` they panic without `build/oracle` or `java` (both pass after a regen) |
| Gate 1 N7 / Gate 2 R7 per-fixture gap reporting | **closed (WP-4.8)**: `gaps.json` next to `actual.jsonl` (tests/golden/README.md) |
| Gate 2 R1 `testworld-6000` dumps | fixed (WP-2.H) |
| Gate 2 R2 wiring sites | `setInPortal` done (WP-3.1b); the rest → WP-4.6 |
| Gate 2 R3 stale docs | item.rs, arrow.rs and portal.rs fixed. **Still stale:** spawner.rs:24-32; living.rs:768-770 ("EntityPlayer's is WP-2.5"); pig.rs:12-13 and :95-98 ("is WP-2.3"); cake.rs:127 ("not in EntityBase yet (Phase 2)"); world/entities.rs:177-179; docs/mappings.md:345 (§3.1 A). **New:** player.rs:824-827 ("`EntityWolf` is not transcribed (WP-2.3) and cannot be constructed yet"). These are WP-4.6's files; listed for it |
| Gate 2 R5 taken-out guard | **WP-4.8**: traced (§3.1 E update). Class queries test a taken-out entity's class (no record for another class); every default fixture's remaining record is the placement check over the `use_item` player, which the item tests itself. A box snapshot of taken-out entities is not needed for any reachable case found; the bed explosion, the one reachable case with an effect, now passes the player in hand |
| Gate 0 skeleton Q8: `entity()` / `entity_index()` are linear scans | still linear (world/entities.rs:23-35). Relevant to WP-4.3's performance pass; listed for it |
| Gate 1 decision 7: whole-region rewrite per chunk save | scheduled **WP-4.3** (incremental region writes) |

### 3.4 For WP-4.6 (coordination; this WP changes none of it)

WP-4.6's brief has the R2 sites, the tamed branch of `alertWolves` and the
chest-minecart `InventoryRef`. This review adds:

1. `EntityFireball.getCollisionBorderSize` → 1.0 (entity/dispatch.rs:793-797;
   fix fireball.rs:10-12's "client" note).
2. The stale docs in §3.3 (R3 row), including mappings.md:345 and
   player.rs:824-827, and inventory/container/mod.rs:111-113: `te_gone`'s
   "unreachable from the client, which closes the window" is wrong (§3.1 A).
3. The cake fix is observable (§3.1 A). A unit test can pin it:
   `heartsLife` 15, eat cake, take 1 damage → vanilla takes the full branch
   and sets `heartsLife` 20.
4. Optionally the dead code of §3.1 D and the sim-side render arms of
   §3.1 B, if WP-4.6 wants them out of the way (otherwise WP-4.8).

### 3.5 Uncertain or unverified items carried in the gate documents

- **K11, K12, K13:** behaviour checked by unit tests only, not against the
  oracle (owner Q1, Q16).
- **K3:** the display draw order is unverified.
- **Gate 2 §4.2:** "the `readEntityFromNBT` fallthrough is probably
  unreachable". Now verified unreachable (§3.1 D).
- **Gate 2 §4.2:** the cake gap "probably unobservable". Wrong; it is
  observable (§3.1 A).
- **Gate 2 §4.2 and mappings.md:345:** the kill hooks "change no state".
  Wrong since WP-2.7 (§3.1 A).
- **Gate 3 O2:** the dev `use_portal` round trip re-enters the nether. This
  is a dev-tool path, equivalent to vanilla. The real walk-in is WP-3.9.
- **Audio output:** not checked at Gate 3 (WP-4.1).

### 3.6 Proposed follow-up WPs (not scheduled; for the orchestrator and owner)

| Proposed | Content | Crates | Size |
|---|---|---|---|
| **WP-4.7 Small parity remainder** | (1) Stats autosave: `func_27178_d` / `func_27425_c` / `func_27420_b` on `StatFileWriter`, the send run synchronously from the shell's tick, and the tags relabelled (§2.3). (2) Container close timing (README): close in tick N after the player's update, so tick N+1's input goes to the game as in vanilla (§3.2 b), and the te_gone case of §3.1 A shrinks to an explosion after the player's update. (3) The screen-opened-by-click input routing (README). (4) Q33's two buttons if the owner wants them enabled (and doing nothing). (5) Removed references with a parity effect (§3.1 A): the projectile's shooter (arrow.rs:312: Peaceful bounce, difficulty scaling, knockback, `alertWolves`, the extra `Math.random` draw) and the mob's look target (living.rs:324); a copy of the removed entity's class and position held by the referrer is enough. (6) Optional: merge an existing `stats_<user>.dat` at start-up (`func_27180_b`, §2.3) | mc-sim `stats/` (fixture-neutral), mc-sim `entity/` for (5) (with WP-4.6 or after it), mc-client `game.rs`, `gui/` | small to medium |
| **WP-4.8 Gap hygiene** | (1) Surface `unimplemented_hits` / `skipped_systems` (a `dump_debug` field in the client, and a per-fixture `gaps` file next to `actual.jsonl`: R7/N7). (2) R5 and the removed-entity family (§3.1 E): trace the multi-site files of family 2 site by site, then keep a snapshot of taken-out or removed entities for queries and references (the general form of WP-4.7 item 5). (3) The coverage checks (N1/R6) and the cross-read `#[ignore]` (N6). (4) Remove the dead code (§3.1 D) and the sim's render-only arms (§3.1 B); fix the stale docs not taken by WP-4.6. (5) The citation nits of §2.4. **Done (WP-4.8)**: (1), (3), (4), (5) in full; (2) traced, its neutral half done, four reachable gaps fixed with fixtures, one proposal left (§3.1 E update) | mc-sim (behaviour-neutral except (2)), mc-golden, mc-client | medium |
| Rendering follow-up (or wave F) | The `f32` far-origin jitter, the pass-1 sort, the inventory preview's overlay/shadow/fire, anaglyph's second eye | mc-client | by WP-4.1's findings |

## 4. PLAN_v4 §5 reconciled with what was delivered (for PLAN_v5)

Commit counts include "merge" and integration ("int") commits; ranges are
`first..last` on `phase-4`. "Added" means the orchestrator added or split
the WP; PLAN_v4 does not list it. Every per-WP branch still exists
(`git branch`); nothing is pushed.

| WP | Name | Origin | Commits | Range | Status |
|---|---|---|---|---|---|
| 0.1 | Adapt the template into the workspace | plan | 7 | `5a25331..de70816` | done (Gate 0) |
| 0.2 | Oracle build, `Math.random` rewrite, Tier A, `mc-golden` runner | plan | 5 | `dcaf8ac..07b4600` | done |
| 0.3 | `mc-core`, `mc-nbt`; §3.4 rule 5 decided | plan | 9 | `56c7648..1af5398` | done |
| 0.4 | Tier B/C/S scaffolding, `Math.random()` census | plan | 5 | `a07f239..77bf175` | done |
| 0.5 | `mc-sim` skeleton | plan | 5 | `6d18a34..a094a54` | done |
| 1.1 | Block data table | plan | 4 | `8dbe907..be11166` | done |
| 1.2 | Item data table (+ integration steps 1, 2, 4 of `wp-int-a`) | plan | 6 | `d7dd7cf..8a0a8bf` | done |
| 1.3 | Chunk storage and lighting | plan | 4 | `ba1d2b0..690b060` | done |
| 1.4 | Noise and terrain shape (+ `wp-int-a` step 3) | plan | 5 | `96e149a..87a6ca1` | done |
| 1.5 | World block access, ticks, minimal player, `mob_spawning` knob | plan | 4 | `cfe99c8..a38ebcd` | done |
| int-a | Integration clean-up between waves (module table, public ids, `block_texture` move) | **added** | (in 1.2, 1.4) | `4716b62..8a0a8bf` | done |
| 1.6 prep | Block dispatch surface for every overridden `Block` method, `TRANSCRIBED` registry, coverage test; `use_item` op | **added** | 3 | `8baf851..3d484aa` | done |
| 1.6a | Fluids | split of 1.6 | 6 | `ec8a2ab..bc2d45e` | done |
| 1.6b | Fire, plants, crops, leaves, snow, ice | split | 6 | `28bf60d..7aff249` | done |
| 1.6c | Redstone wire, torches, repeaters, buttons, levers, plates | split | 5 | `d710b78..d87dd84` | done |
| 1.6d | Pistons (+ `TileEntityPiston`) | split | 5 | `641e31f..c905133` | done |
| 1.6e | Rails (`RailLogic`, detector rail) | split | 4 | `740954d..b4f5195` | done |
| 1.6f | Doors, trapdoors, beds, ladders, cake, portal, fences, stairs, slabs, workbench, locked chest | split | 5 | `dad3691..2f12440` | done. PLAN's "containers … signs" moved to 1.8 |
| 1.6g | Simple remainder (TNT, soul sand, web, sponge, pumpkin, breakable) | split | 4 | `aaf7dd8..f4a6c11` | done |
| 1.7 | Population features | plan | 4 | `bdc0bdc..3ad8398` | done |
| 1.8 | Tile entities, explosions, **and the container blocks and signs** (from 1.6) | plan (widened) | 5 | `45b9438..74a1129` | done |
| 1.9a | McRegion container (`RegionFile`, cache) | split of 1.9 | 2 | `5e8b74f..b86a7b8` | done |
| 1.9b | Chunk NBT, `SaveOldDir`, `SaveFormatOld`, `WorldInfo`, D5 validation | split of 1.9 | 5 | `e88b46c..6ebd8c9` | done (Gate 1 `66c6d09`) |
| 2.1 | Base entity physics | plan | 6 | `ab3cacc..22284cc` | done |
| 2.2 | Living base and pathfinding | plan | 8 | `33ac436..31c1252` | done |
| 2.3a | Animals and wolf | split of 2.3 | 9 | `f33f7e4..f18f2da` | done |
| 2.3b | Monsters, slime, ghast | split | 8 | `098c909..fd9021e` | done |
| 2.3c | Squid, spawn rules (`SpawnerAnimals`) | split | 9 | `c45b11d..3b7ce47` | done |
| 2.4a | Item, falling sand, TNT, painting, projectiles, fishing hook, lightning | split of 2.4 | 6 | `85b3871..4d2bd61` | done |
| 2.4b | Minecart and boat | split | 6 | `7ec2bc5..6549971` | done |
| 2.5a | `EntityPlayer`, `EntityPlayerSP` (non-client), `PlayerControllerSP` | split of 2.5 | 7 | `f5bae6d..2c012d9` | done |
| 2.5b | Inventory, containers, crafting, furnace recipes | split | 8 | `e457764..f4383ab` | done |
| 2.6 | Item behaviour, `ItemMap`, `MapData` | plan | 9 | `8d811ff..f864d46` | done |
| 2.7 | Stats and achievements (local) | plan | 5 | `b0c0fed..febad38` | done. The row's "`StatsSyncher` deferred per D2" is wrong (§2.3) |
| 2.G | Gate 2 package: `reload` op, reload fixtures, `tick/gate2-session` | **added** | 5 | `ffc14d9..c415e67` | done (Gate 2 doc `e969e7b`) |
| 2.H | Item, arrow, snowball/egg pickups (`onCollideWithPlayer`); `testworld-6000` dumps the player | **added** | 6 | `3241e81..a8f3841` | done; Gate 2 passes |
| 3.1 | App shell (+ `Teleporter`, `usePortal`: Gate 2 Q7) | plan (widened) | 5 | `5f26aa1..2009a09` | done |
| 3.1b | Client requests (`ClientRequest`), `usePortal` split, `World::new_for_dimension`, `seed_clock_randoms` | **added** | 3 | `692fd79..355ab2d` | done |
| 3.2 | Texture atlas, animated textures | plan | 3 | `0a66798..6c8179d` | done |
| 3.3a | Terrain core: sectioning, cubes, AO, frustum, crack | split of 3.3 | 7 | `5b87950..6ce6f86` | done |
| 3.3b | Fluids | split | 2 | `1ca92b3..a0530d7` | done |
| 3.3c | Plants, torches, fire, crops, cactus | split | 2 | `69f0eca..1625647` | done |
| 3.3d | Redstone, rails, doors, stairs, pistons, bed and the rest | split | 3 | `fd5dd1b..82aaf2d` | done |
| 3.4 | Sky, clouds, weather, fog, selection box (+ mc-sim `world/sky.rs`) | plan | 4 | `5f3d92e..61793bd` | done |
| 3.5a | Models | split of 3.5 | 3 | `9bf0cae..f93f2aa` | done |
| 3.5b | Entity and tile-entity renderers | split | 6 | `19bdf6b..17b269b` | done; D1 open → 3.9 |
| 3.6a | GUI base: batcher, font, widgets | split of 3.6 | 3 | `b568554..6324be7` | done |
| 3.6b | HUD, chat, inventory and container screens | split | 4 | `96ed67b..4ed0119` | done |
| 3.6c | Menus, world select/create/import, options, achievements, stats | split | 3 | `f9ff6ff..49cd40d` | done; D2 open → 3.9 |
| 3.7a | Particles | split of 3.7 | 3 | `ad7322c..eac8e64` | done |
| 3.7b | Hand, item renderer, sRGB compositing | split | 6 | `cd8deb0..6a72090` | done |
| 3.8 | Audio | plan | 4 | `4f2963f..708d3a6` | done (Gate 3 `d52ac6b`) |
| 3.9 | Gate 3 defects D1/D2, nits N1–N3, `click_slot` (O1), a real nether-portal walk | **added** | 0 | branch `wp-3.9` | in progress (wave E) |
| 4.1 | Side-by-side parity checklist → `docs/PARITY_CHECKLIST.md` | plan | 0 | branch `wp-4.1` | in progress (wave E) |
| 4.2 | Long-run soak (5 fixtures, `SOAK` modes) | plan | 5 | `a62b460..70cdb29` | done, on `phase-4` |
| 4.3 | Performance and packaging → `docs/PERF.md`, `docs/RUNNING.md` | plan | 0 | branch `wp-4.3` | in progress (wave E) |
| 4.4 | Deferred-tag review → this document | plan (widened) | — | branch `wp-4.4` | this commit |
| 4.5 | World export → `docs/EXPORT.md` | plan | 0 | branch `wp-4.5` | in progress (wave E) |
| 4.6 | Sim parity gaps: R2 sites, `alertWolves` tamed branch, chest-minecart `InventoryRef` | **added** | 0 | branch `wp-4.6` | in progress (wave E) |
| 4.7 | Small parity remainder (§3.6) | **proposed here** | — | — | not scheduled |
| 4.8 | Gap hygiene (§3.6) | **proposed here** | — | — | not scheduled |
| wave F | Parity fixes from `PARITY_CHECKLIST.md` | planned by the orchestrator | — | — | after wave E |
| Gate 4 | Review → `docs/GATE4_REVIEW.md` | plan | — | — | after wave F |
| Phase 5 | Browser build: Trunk, browser `Storage`, the user's jar/`resources/` by file picker, audio on first gesture | plan | — | — | not started. The import path box (Q22) needs its browser counterpart there |
| Phase 6 | Multiplayer, texture packs, sky dimension, pre-McRegion conversion | plan | — | — | owner decisions (Q32) |

Gate documents: `docs/GATE0_REVIEW.md` (added in `a094a54`, WP-0.5's last
commit), `66c6d09` (Gate 1), `e969e7b` (Gate 2, updated by WP-2.H),
`d52ac6b` (Gate 3). Other commits outside a WP: `9b47aec` (template
initial commit) and `a165739` (PLAN_v4 §3.1 `worldgen`).

## 5. Owner-question backlog (deduplicated)

Sources: PLAN_v4 §8 items 1–4, Gate 0 §2 and §3, Gate 1 §8 Q1–11, Gate 2 §8
Q1–10, Gate 3 §8 Q1–11, and this review. No owner answer is recorded in
the tree. The owner's two instructions, "continue execution" (2026-10-06)
and "do phase 1 and 2" / "do phase 3 and 4", were taken as approval to
proceed (Gate 1 §5.1).

| # | Question | Sources | Status | Blocks |
|---|---|---|---|---|
| Q1 | **Second oracle rewrite** `Entity.java:96` `rand = new Random()` → `HarnessMath.entityRandom(entityId)` | PLAN §8.3; G0 §2.5; G1 Q11; G2 Q3; G3 Q2 | **open** | Hostile natural-spawning fixtures (K10). The constructor draws of K11. The soaks have to hold thunder off and run at difficulty 0 (WP-4.2). The port needs no change |
| Q2 | Confirm oracle `Math` == `StrictMath` (intrinsics off, fdlibm in Rust; 1 ulp from the real client on 1–4 % of sin/cos/pow inputs) and the clean-room `JavaHashMap` | PLAN §8.4; G0 §2.1-2; K9 | **open** | nothing technical; defines "parity" |
| Q3 | Gate sign-offs: Gate 1 (with exceptions, now moot: all 7 fixtures green since Phase 2), Gate 2 (passes since WP-2.H), Gate 3 (passes) | G1 Q1; G2 Q1; G3 Q1 | **open** (no written sign-off) | formally PLAN §6 |
| Q4 | Accept the workflow: integration branches, splits and added WPs (§4 table); Phase N started before the Gate N−1 questions were answered | G1 Q2; G2 Q8, Q10 | **open** | PLAN_v5 |
| Q5 | Gate 0 skeleton questions 1–8: (1) block statics as world state; (2) leaves `isOpaqueCube` follows the fancy option; (3) block reads take `&mut World`; (4) `ChunkGenerator` enum; (5) `TileEntityRef` handles; (6) the `HarnessPlayer` name; (7) flat-tuple entity data; (8) linear entity lookup | G0 §3 | **open**, built upon. (2) is de facto answered "as vanilla": the client sets it in `loadRenderers` (terrain/mod.rs:519, from RenderGlobal.java:187), so the simulation follows the option as in vanilla. (8) is still linear (WP-4.3) | nothing |
| Q6 | Approve `mc-sim` interface changes: G1 §5.10, G2 §5.8 (map files, reload, `&mut Entity` dispatch, `EntityList` table, `stat_file_writer`, `client_additions`), G3 §5 (Teleporter, `ClientRequest` and the public `World::client_requests`, `new_for_dimension`'s save-handler hand-over, `seed_clock_randoms`, `world/sky.rs`) | G1 §5.10; G2 Q6; G3 Q7 | **open** | nothing in flight |
| Q7 | Accept the oracle harness additions (Tier A kinds, ops, dumps, `world: load`, `reload`, and WP-4.2's `SOAK` modes); `Math.random()` remains the only vanilla rewrite | G1 Q3; G2 §5.2 | **open** | nothing |
| Q8 | Keep the `mob_spawning: "none"` knob | G1 Q4 | **open** | nothing |
| Q9 | Seeding the clock-seeded Randoms in the shipping game | G1 Q5 | **resolved by implementation** (WP-3.1/3.1b `seed_clock_randoms`); confirm | — |
| Q10 | **`testWorld` licence.** REFERENCES.md: "Mojang-generated save data; local only". `tests/golden/README.md`: "not Mojang content". Three committed files are byte copies of it (§6) | G1 Q9; G2 Q8; G3 Q4 | **open** | Whether `save/testworld`, `nbt/testworld-level-dat` and the fixtures that load them (`tick/testworld-6000`, `soak/testworld` on `phase-4`) may stay in git; Q11 |
| Q11 | Commit the full `testWorld` (784 chunks, four regions, 3.6 MB) as the `save/testworld` fixture | G1 Q6 | **open**, depends on Q10 | — |
| Q12 | Cross-read tests should not pass silently without `build/oracle` | G1 Q7 | **done (WP-4.8)** | `#[ignore]`d with the reason; fail when run without the oracle |
| Q13 | Per-fixture gap reporting | G1 Q8; G2 Q9 | **done (WP-4.8)** | `gaps.json` per fixture; `dump_debug`'s `gaps` in the game |
| Q14 | Tier S policy for unconstructible saved entities | G1 Q10 | **moot** (every constructor exists; dead code) | — |
| Q15 | Confirm the reload protocol, including the kept provider (K14) | G2 Q4 | **open** | nothing |
| Q16 | Confirm the `HarnessPlayer` model; add an oracle path for the `EntityPlayerSP` parts that move the player (`pushOutOfBlocks`, sneak `ySize`, portal timer)? | G2 Q5 | **open** | K12 stays unit-tested only |
| Q17 | `testworld-6000` dump change (a committed fixture changed) | G2 Q2 | **done** by WP-2.H with the orchestrator's approval; the owner may revert | — |
| Q18 | Wire the R2 sites and the tamed-wolf branch | G2 Q1; G3 Q8 | **scheduled** WP-4.6 | — |
| Q19 | Where nether travel lives | G2 Q7; G3 Q3 | **resolved** (WP-3.1/3.1b); confirm. The real walk-in is scheduled in WP-3.9 | — |
| Q20 | Gate 3 D1/D2: a short WP or WP-4.1? | G3 Q1 | **scheduled** WP-3.9 | — |
| Q21 | Data directory default `$XDG_DATA_HOME/mc_b173` (never `.minecraft`); the `--jar` / `--resources` defaults | G3 Q5 | **open** | WP-4.3's packaging and `docs/RUNNING.md` defaults |
| Q22 | Import by typed path, or add a native dialog (e.g. `rfd`)? | G3 Q6 | **open** | Phase 5 needs its own picker either way |
| Q23 | Default username: vanilla's `Player<millis % 1000>` makes stats and achievements per launch; persist a name? | G3 Q9 (O3) | **open** | stats persistence |
| Q24 | Clean-ups N1–N3, O1 | G3 Q10 | **scheduled** WP-3.9 | — |
| Q25 | Uncapped present mode and frame-time histogram for the baseline | G3 Q11 | **scheduled** WP-4.3 | — |
| Q26 | Install the `wasm32-unknown-unknown` std | PLAN §8.1 | **resolved** (Arch `rust-wasm` 1.99.0, 2026-10-05; every gate ran the check) | — |
| Q27 | Approve the WP-0.1 tree | PLAN §8.2 | **resolved** ("continue execution", 2026-10-06, read as approval) | — |
| Q28 | Gate 0 decisions 3, 4, 6: trace comparison per parsed JSON line; NBT written in sorted key order; `"world": "empty"` scenarios | G0 §2 | **open** (confirm) | nothing |
| Q29 | Orchestrator decisions of Phases 1–2 not covered above (G1 §5: explode seeding, minimal player, `u8` worldgen ids, in-memory `RegionFile`, save-path choices; G2 §5: statistics on the player, spawn kinds, item/map state as world fields, `getEyeHeight` landing in 2.3b, the `gate2-session` construction, sensitivity workarounds, `testworld-6000` with spawning off) | G1 §5; G2 §5 | **open** (confirm or overturn) | nothing |
| Q30 | Commit trailers: the briefs ask for `Claude Fable 5.1`; the commits carry `Claude Opus 5.5`, the model that made them | G1 §5.12; G2 §5.13 | **open**; pick one for PLAN_v5 | nothing |
| Q31 | **New.** The portal-with-workbench double drop (§3.2): accept the value-stack divergence, or emulate vanilla's shared `ItemStack`s? | this review | **open** | nothing |
| Q32 | **New, restating PLAN Phase 6.** Decide each D3 item: multiplayer; texture packs; the sky dimension (unreachable in vanilla singleplayer, §2.3); pre-McRegion conversion (reachable for old saves; D5 refuses them today) | PLAN §5 Phase 6; D3 | **open** | Phase 6 |
| Q33 | **New.** Vanilla enables the Multiplayer button and "Open texture pack folder"; the port disables both (D3). Keep them disabled, or enable them as no-ops / with a "not supported" screen? | this review | **open** | WP-4.1 checklist entries |
| Q34 | **New. Amend D2** (PLAN_v4.md:100): drop "stats sync" from D2's out-of-scope list, or narrow it to a server upload, which b1.7.3 never had, since every stats "sync" is local file I/O (§2.3). Then: schedule the stats autosave (proposed WP-4.7 item 1) or accept saving only at world change and shutdown; and merge an existing `stats_<user>.dat` (WP-4.7 item 6) or accept that divergence | this review | **open** | the D2 text in PLAN_v5; WP-4.7 items 1 and 6 |

Totals:
- resolved (4): Q9, Q19, Q26, Q27;
- scheduled (4): Q18, Q20, Q24, Q25;
- done or moot (2): Q17, Q14;
- open (24): the rest. Q12, Q13 and Q34 have a WP proposed in §3.6. Q5's
  item (2) is answered in practice.

Only Q1 and Q10 block work that would otherwise be done.

## 6. Licensing / D8 audit

`git ls-files` at `d52ac6b`: 1,004 files. Binary by git's own detection
(`git ls-files --eol`, `i/-text`): 17. `phase-4` adds only text (the five
soak fixtures, about 10 MB of JSON).

| File | Bytes | Added in | Origin | Mojang-derived? |
|---|---|---|---|---|
| `build/icon_1024x1024.png` | 101,624 | `9b47aec` (template) | `bevy_game_template`'s Bevy icon; MIT (`credits/licenses/Bevy_MIT_License.md`) | no |
| `build/macos/AppIcon.iconset/icon_{16,32,128,256,512}x…{,@2x}.png` (10) | 7,886–101,624 | `9b47aec` | same; `mc-app/src/main.rs:175` embeds `icon_256x256.png` as the window icon with `include_bytes!` | no |
| `build/macos/src/Game.app/Contents/Resources/AppIcon.icns` | 161,847 | `9b47aec` | same | no |
| `build/windows/icon.ico` | 156,431 | `9b47aec` | same | no |
| `tests/golden/math/sine-table/sine_table.f32le.bin` | 262,144 | `52d4749` (WP-0.2) | the oracle's dump of `MathHelper.SIN_TABLE`: `(float)sin(i·2π/65536)` for 65,536 `i` | output of running vanilla code; mathematical content only |
| `tests/golden/nbt/testworld-level-dat/level.dat` | 414 | `56c7648` (WP-0.3) | **byte-identical** to `.reference/mc_b1.7.3-world/testWorld/level.dat` (`cmp`) | **the Gate 1 Q9 question** (Q10). Gate 1 named only `save/testworld`; this third file is in the same position |
| `tests/golden/save/testworld/world/level.dat` | 414 | `6b8d703` (WP-0.4) | byte-identical to the same file | Q10 |
| `tests/golden/save/testworld/world/region/r.0.-1.mcr` | 1,249,280 | `6b8d703` | byte-identical to `testWorld/region/r.0.-1.mcr` | Q10 |

**Nothing from the jar or `resources/` is in git or in the binary.**
- No PNG, OGG, `.mus`, `.lang` or font file outside the template icons
  (`git ls-files` by extension).
- `assets/` holds only its README.
- `include_bytes!` / `include_str!` appear only for the template icon
  (mc-app), the model test data (`model/testdata/java_models.txt`) and the
  override lists (`mc-sim/tests/data/*.txt`).
- Every runtime asset is opened by path: the `lang/` files
  (`lang.rs:12-14`), `font.txt` (`gui/font.rs:33`), the splashes, the
  achievement map and the textures, from the user's `minecraft.jar` (D8).

**Text in git that is derived from Mojang code, which the owner should
know about.** All of it exists by design (D2); the repository is private
(PLAN_v4 §7):

- **The Rust sources:** a transcription of the decompiled client, with
  `// src:` citations. They include short UI strings from the Java, e.g.
  the title screen's copyright line (gui/menu/main_menu.rs).
- **`tests/golden/**/expected.jsonl`:**
  - 221 files, 45 MB, at `d52ac6b`, plus 5 soak files on `phase-4`;
  - output of running vanilla code (traces and digests);
  - includes the block, item, recipe and stat tables with their MCP names
    and translation keys.
- **`crates/mc-client/src/model/testdata/java_models.txt`:** 99,917 bytes of
  model geometry. `ModelDump.java` (project-owned) printed it by running the
  vanilla `Model*` classes.
- **`crates/mc-sim/tests/data/*_overrides.txt`:** lists of vanilla
  `Class.method` names, generated from the `.reference` tree by the
  `gen_*.sh` scripts next to them.
- **`tools/oracle/census.md`:** a census of `Math.random()` / `new Random()`
  sites with file:line.
- **`tools/oracle/java/harness/*.java`:** the project's harness. Two places
  mirror small vanilla fragments line for line (Gate 2 §5.2):
  - `HarnessWorld.Controller`, a copy of `PlayerControllerSP`'s dig logic;
  - the `slot_click` composition of `BlockChest.blockActivated`.

`.gitignore` covers `/.reference/`, `/build/oracle*`, `/build/golden-out/`
and `/build/screens/`. The `.reference` symlink in each worktree is
untracked. No screenshot is tracked.

## 7. Code changes made by this WP

One documentation fix, in a module doc comment. No behaviour change.

- `crates/mc-client/src/game.rs:88-90` ("Out of scope here"). Before:
  "Applet, multiplayer, the snooper and stats sync". After: "Applet,
  multiplayer, the has-paid check, resource downloads and stats sync". b1.7.3
  has no snooper (`grep -ri snoop` over the D2 tree finds nothing). The new
  text names the systems the same sentence's parenthesis lists
  (`ThreadCheckHasPaid`, `ThreadDownloadResources`). The stats item is still
  mislabelled; §2.3 leaves it to the WP that ports the autosave. The
  `deferred` word moved from line 88 to 89 (§2.1 row #6).
- docs/GATE3_REVIEW.md:295 still lists "snooper" among game.rs's deferred
  items. That line records what game.rs said at Gate 3; it is historical,
  and this fix supersedes it.

Not changed, deliberately: the stats labels (§2.3), the citation nits
(§2.4) and the stale docs (§3.3). They are in files other WPs own and are
editing now (WP-4.6 owns mc-sim; WP-3.9 and WP-4.3 edit the client). Each
is listed with its line for whoever takes it.
