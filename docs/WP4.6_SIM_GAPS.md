# WP-4.6: simulation gaps reachable in singleplayer

Written 2026-10-09 for the owner's Gate 4 review. WP-4.6 was added by the
orchestrator under Gate 4 ("vanilla parity"). Its job was to close the
simulation gaps that Gate 2 and Gate 3 left open and that singleplayer can
reach. Branch `wp-4.6`, forked from `phase-4` at `70cdb29`. Nothing is
pushed.

**Result.** Every gap that Gate 2 (§4.2, R2, owner question 1) and Gate 3
(§4: the chest minecart) recorded as reachable in singleplayer is now
transcribed. Each has an oracle fixture. The review of this WP found one
more reachable gap, a projectile whose shooter has already left the world
(§1, last row). It is closed too, with a fifth fixture.

The 221 default fixtures and the 5 soaks regenerate byte-identically. The
five new fixtures pass. In `World::unimplemented_hits`, 96 `note_unimplemented`
call sites remain (there were 97 at `70cdb29`; the brief's grep also counts
the definition). One of the 96 marks a vanilla path that singleplayer can
reach and that the simulation now runs, but the client cannot yet show
(§2, §3). None of the others is a known vanilla behaviour that singleplayer
can reach. **Since closed by WP-4.7** (`9f2708b`, `260e9a8`; §2).

## 1. What was closed

| Gap (Gate) | Java | Port | Fixture | Unit tests |
|---|---|---|---|---|
| Cake heal without `heartsLife` (G2 R2) | BlockCake.java:76-88 → EntityLiving.java:296-305 | `block::cake` calls `entity::living::heal`. The private copy is gone | `tick/cake-heal-window`: a hit right after a slice counts in full (health 13 at t = 5; the old code gave 17) | `singleplayer_gaps::cake_heals_through_entity_living_heal` |
| Pig → zombie pigman on lightning (G2 §4.1, R2) | EntityPig.java:62-69 | `entity::pig::on_struck_by_lightning`: a new `EntityPigZombie` (constructor, `setLocationAndAngles`, `entityJoinedWorld`), then the pig dies | `entity/pig-lightning-pigman` (harness `LightningBolt`, difficulty 1 so the pigman lives) | `a_struck_pig_becomes_a_zombie_pigman` |
| `onDeath`'s `addToPlayerScore` / `onKillEntity` (G2 R2) | EntityLiving.java:407-414, EntityPlayer.java:239-247, :795-800 | `entity::player::kill_hooks` (below) | none can observe it: `scoreValue` is always 0 and the oracle's `addStat` is empty. `entity/player-attack-pig` still passes | `stats::a_kill_counts`, `a_kill_reaches_the_killer_in_its_slot` |
| `alertWolves`' tamed-wolf branch (G2 question 1) | EntityPlayer.java:402-437 | `entity::player::alert_wolves`, now in full | `entity/wolf-defends-owner` | `alert_wolves_sends_the_players_tamed_wolves`, `…_without_a_username_is_recorded` |
| Chest minecart cannot be opened (G3 §4) | EntityMinecart.java:5, :716-761, :772-775, :811-817 | `InventoryRef::Entity(id)` with every `Container` `inv_*` arm. The client's `open_player_gui` opens `GuiChest` for `PlayerGui::MinecartChest` | `entity/minecart-chest-container` | `a_chest_cart_is_a_container_inventory`, `…_gone_from_the_world_is_recorded`, `a_gone_chest_cart_is_not_usable_and_not_recorded` |
| A projectile's shooter that has left the world (review finding) | EntityArrow.java:163, EntityFireball.java:124, EntitySnowball.java:153, EntityEgg.java:153; EntityPlayer.java:366-390; EntityLiving.java:154-158, :337-348 | `World::removed_entities` (below). The arrow, fireball, snowball and egg pass the removed shooter as the attacker. Before, they passed null | `entity/skeleton-arrow-after-death` | `a_removed_shooter_is_still_the_arrows_attacker`, `removed_entities_are_kept_only_while_named`, `stats::a_kill_by_a_removed_players_arrow` |

How the kill hooks work. Java calls the attacker object directly. The port
holds the attacker either as a copy (an arrow's owner) or as a shared borrow
(the player hitting with its own hand). The hook therefore goes to the
entity by id:

- **Attacker in its slot** (an arrow's owner, a harness direct `attack`): the
  hook runs at once, in Java's order.
- **Attacker out of its slot** (the player during its own melee hit): the
  hook waits in `World::pending_kill_hooks`. It runs right after the hit in
  `attack_target_entity_with_current_item`, before the item's `hitEntity`,
  as in Java. Otherwise it runs when the world puts the player back in its
  slot.

Nothing reads the score or the statistics in between. A player that has
left the world (dead for 20 ticks, with its arrow still in flight) gets the
hook on the removed object (below). That case is still recorded, because
the client does not see it (§3).

How a removed shooter works. Java's entity fields hold objects. A skeleton
is removed from `loadedEntityList` 21 ticks after its death
(`deathTime > 20`, EntityLiving.java:154-158), but its arrow's `owner` still holds the dead
skeleton. When the arrow lands, Java passes that object to
`attackEntityFrom` (EntityArrow.java:163). The port's fields hold ids, and
before this fix the arrow found no entity for the id and passed null. For a
player victim at difficulty 1 that gave 4 damage instead of 2 (the
`EntityMob` scaling, EntityPlayer.java:366-378). It also skipped
`alertWolves` and gave no knock-back at all (a null attacker only draws
`attackedAtYaw` from `Math.random()`, EntityLiving.java:346-348) instead
of pushing the player away from the skeleton. Now, when `updateEntities` or
`updateEntityList` removes an entity, the world keeps it in
`World::removed_entities`, exactly as it left. It stays there as long as a
loaded entity names it in an entity field. These fields are `riding`/`riddenBy`,
`currentTarget`, `playerToAttack`, the ghast's `targetedEntity`,
`fishEntity`, and the projectiles' owner fields. These are every
entity-typed field of the simulation's entity classes. Pruning runs after
each removal pass, so nothing builds up.
`World::entity_object(id)` returns the loaded entity or else the removed
one. Three places use it:

- the projectile attacker (`arrow::attack_entity_by_id`);
- the arrow-owner lookup in `EntityPlayer.attackEntityFrom`;
- `World::is_entity_dead_or_gone`, which now reads the removed object's
  `isDead` without a record. A wolf that targets the dead skeleton drops
  it on its next AI update, as in Java.

A new entity that joins with a removed entity's id replaces it. This
happens when the respawned player takes the dead player's id
(Minecraft.java:1451, :1468).

Two smaller fixes came with these:

- `alertWolves` now passes the attacker to the wolf search as the entity in
  hand, so an angry wolf biting the player is no longer recorded as
  "taken out".
- A null username is recorded as Java's `NullPointerException`.

**Harness change (outside the brief's file list, additive).** The
`slot_click` op had no way to put an item into a chest minecart. The new
container spec `"minecart <id>"` covers it: `new ContainerChest(inventory,
cart)` on the chest minecart with that id. It is added in
`tools/oracle/java/harness/TickScenario.java` and
`crates/mc-golden/src/tick_driver.rs`, and documented in
`tests/golden/README.md`. No other spec changed, and every committed fixture
is byte-identical.

## 2. Classification of the remaining `note_unimplemented` sites (95)

(a) means reachable in vanilla singleplayer and not transcribed. **One
site remains, in part:** `entity/player.rs:642`. A player's arrow kills a
mob after the player has died and the world has dropped it (on the
game-over screen, before respawning). The simulation applies Java's
`addToPlayerScore` / `onKillEntity` to the removed player object, but the
client shows and keeps the dead player through its own copy
(`Game::detached_player`), which misses the hook (§3). **Closed by WP-4.7**
(`9f2708b`: the hook on a removed player is no longer recorded; `260e9a8`:
`Game::player`, `render_view` and `stats_mut` read the dead player the world
keeps, so the hook reaches the player the client shows). The other 95 sites
fall into these groups:

| Class | Sites | Why it stays |
|---|---|---|
| (b) client only | `entity/dispatch.rs`: `isInRangeToRenderVec3D` (bolt), `isInRangeToRenderDist` ×4, `getShadowSize` ×7, `EntityFireball.getCollisionBorderSize`; `block/grass.rs:88` and `block/leaves.rs:131` (`ColorizerGrass`/`ColorizerFoliage`) | Render and picking data that `mc-client` owns (WP-3.3/3.5/3.1). The simulation never calls them on a tick path. The colorizer sites fall back to the colormap lookup |
| (c) deferred, D3 multiplayer | `entity/dispatch.rs`: `setVelocity` ×4 (Arrow, Egg, Fish, Snowball), `EntityFish.setPositionAndRotation2` | Only `NetClientHandler` calls them |
| Defensive: take-out/put-back guards | every `… out of its slot`, `… not in its slot`, `… taken-out …`, `… no longer in loadedEntityList …`, `… no longer kept`, `Chunk entity list holds …` record: spawner.rs (2), tick.rs, arrow.rs (2), boat.rs (3), base.rs (5), creature.rs, ghast.rs, mob.rs, skeleton.rs, spider.rs, wolf.rs (7), pig.rs:132, minecart.rs (2), fish.rs (3), lightning_bolt.rs, living.rs:324, player.rs (5), projectiles.rs, pressure_plate.rs (2), explosion.rs, world/entities.rs (10), world/mod.rs:2030, save/chunk_loader.rs:118, inventory/container (2) | They mark a state the port's slot model can produce but vanilla's object references cannot. An entity that a field still names is now kept after its removal (§1), so the "no longer in loadedEntityList" records only fire for an entity that no field names. The two container records fire only for slot reads on a cart that is gone. `canInteractWith` on a gone cart returns false without a record, because a cart leaves the list only by dying. None is hit by a fixture except the two AABB-query records (Gate 2 R5, below) |
| Defensive: Java exceptions | painting.rs ×3 (null art), entity/item.rs:222 and nbt.rs:129 (null item), fish.rs:260 (no angler), player.rs:538 and :948 (null username), mob_spawner.rs ×2 (`ClassCastException`, a mob that did not join) | Vanilla throws here. The port records the case and continues |
| Dead or unreachable (Gate 1 N2/N3, Gate 2 R4) | `block/dispatch.rs:680` (the five `untranscribed` tails), `tile_entity/mod.rs:200, :269, :297` (`Untranscribed`), `entity/nbt.rs:458`, `save/entity_list.rs:287, :301` | No path constructs them |
| Unreachable in singleplayer | `spawner.rs:383, :399` (the sleep-spawn pathfinder reads an unloaded chunk) | Chunks within 64 blocks of a player are always loaded |

## 3. Open items (not gaps WP-4.6 could close)

- **Gate 2 R5 is unchanged.** `World::get_entities_within_aabb[_excluding_entity]`
  still skips an entity that is out of its slot further up the stack. In
  Java that entity would be tested like any other. No known case changes a
  value, but nothing proves that.
- **Client-side deviations, out of this WP's scope.** Gate 3 recorded two
  deviations in the `mc-client` README: the container close timing and
  screens opened by a click. Neither needs a simulation accessor that is
  missing, so neither was changed here. WP-4.7 closed the close timing
  (`9f2708b`, `260e9a8`); the screen opened by a click is still open
  (GATE4_REVIEW §8 D1).
- **The kill hooks have no oracle fixture.** The oracle cannot observe them.
  Unit tests cover them.
- **A kill by a dead player's arrow does not reach the client (the one
  (a) site, §2).** The simulation runs the hooks on the removed player object
  and records the case. To show them, the client would need to read the
  dead player from `World::removed_entity(id)`, or run the hook on its own
  `Game::detached_player`. That is a change to `crates/mc-client/src/game.rs`,
  which is outside this WP. What the player loses: `mobKills` (and the
  `killEnemy` achievement) for that one kill, and the score on the game-over
  screen, which is always 0 in b1.7.3 anyway (`scoreValue` is never set).
  After a respawn the same arrow names the new player, because the new
  player takes the old player's id. Its kill hooks then go to the new
  player's statistics writer, which is the same `Minecraft.statFileWriter`
  Java uses. The score goes to the new player instead of the dead one, and
  is 0 either way. **Closed by WP-4.7** (`9f2708b`, `260e9a8`): the client
  reads the dead player the world keeps, so the kill reaches it. Since
  WP-4.7's review fix (`2f19710`) a respawn moves the dead player to a
  fresh id, so the arrow keeps naming the dead object, as in Java.

## 4. Checks

- `sh tools/oracle/regen.sh` and `SOAK=only sh tools/oracle/regen.sh`: every
  committed `expected.jsonl` is unchanged. The five new fixtures come from
  the oracle. The review fix changed no Java, and its default regen changed
  only the new `entity/skeleton-arrow-after-death`.
- `check-sensitivity.sh` (after the review fix): the 226 default fixtures
  are 209 seeded ones, which all change under a flipped seed bit (the 5 new
  ones included), and 17 unseeded ones, which stay identical.
  `SOAK=only check-sensitivity.sh`: the 5 soaks change. `check-reproducible.sh`:
  3 default runs byte-identical (228 files); with `SOAK=only`, 2 runs
  byte-identical.
- The new fixture pins the fix. With the attacker lookup reverted to the
  loaded entity only, the port first differs at t = 28 (the hit's
  `Math.random()` knock-back draws).
- `cargo test --workspace`: 1,130 passed, 5 ignored (the soaks). The soaks
  pass with `cargo test -p mc-golden --test golden_soak -- --ignored`
  (230 s, after the review fix).
- `cargo test --workspace`, `cargo clippy --workspace --all-targets
  --all-features -- -Dwarnings`, `cargo fmt --all --check` all pass.
- Smoke run (`build/screens/wp-4.6/scripts/cart_{fresh,testworld}.json`):
  a chest minecart was spawned on a fresh seed-123 world and on the
  `testWorld` import, then right-clicked. `GuiChest` opened titled
  "Minecart" with 3 rows. Cobblestone and one plank were clicked into cart
  slots 0 and 26. Pressing Escape dropped the 6 planks still on the cursor
  (vanilla `onCraftGuiClosed`). Reopening the cart showed the same contents.
  After the review fix both runs were repeated
  (`build/screens/wp-4.6/{fresh2,testworld2}`) with the same result. The
  fresh-world script now first builds a stone platform at y = 119 around
  (0, 0) and moves the player onto it (`set_block`, `set_pos`), so the cart
  does not depend on the spawn point. `World.rand` is unseeded in a real
  world, so the spawn point varies from run to run.
