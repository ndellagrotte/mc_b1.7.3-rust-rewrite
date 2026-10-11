# mc_b1.7.3-rust-rewrite

A Rust + [Bevy](https://bevyengine.org) rewrite of Minecraft Beta 1.7.3 that
aims at vanilla singleplayer parity. The simulation (world, blocks, entities,
generation, saves) is plain Rust transcribed from the deobfuscated vanilla
source; Bevy is only the shell (window, input, rendering, UI, audio). Parity is
proven by golden fixtures generated from the vanilla Java code running under a
JVM oracle. Worlds use the vanilla McRegion format and open unchanged.

## Status

**Phases 0-4 complete; Gate 4 ("vanilla parity on native") passed on
2026-10-10 and is tagged v0.1.0.** Singleplayer is playable from the title
screen: create, play, save, reload, import and export worlds, death and
respawn, Nether portals. Every block, screen, mob and sound trigger has been
compared side by side against the original client
(`docs/PARITY_CHECKLIST.md`, `docs/GATE4_REVIEW.md`). Phase 5 (the browser
client) has not started. See `docs/PLAN_v4.md` §5 for the phases and work
packages.

## Requirements

- Rust stable (see `rust-toolchain.toml`; the pinned version is informative
  for system toolchains and binding for `rustup`).
- A JDK (21 or newer) to build and run the Java oracle that generates fixtures.
- Your own `minecraft.jar` for Beta 1.7.3 (textures and `lang/` files) and a
  `resources/` directory (sounds and music). **This project ships no Mojang
  assets**; they are read at runtime from paths you configure.
- The read-only reference tree under `.reference/` for development; see
  `docs/REFERENCES.md`. It is git-ignored and never committed.

Since Phase 3 the game is playable from the title screen (create, import,
play, save, reload; `docs/GATE3_REVIEW.md`). How to supply `minecraft.jar` and
`resources/` and the command-line flags: [`docs/RUNNING.md`](docs/RUNNING.md).
Performance measurements and their method: [`docs/PERF.md`](docs/PERF.md).

## Build and run

```
cargo run -p mc-app
cargo test --workspace
cargo clippy --workspace -- -Dwarnings
```

Add `--features dev` to `cargo run` for Bevy dynamic linking during development.

The release build is `cargo build --profile dist -p mc-app` (opt-level 3, LTO;
`target/dist/mc_b173`), which `.github/workflows/release.yaml` packages for
Linux, Windows and macOS with `docs/RUNNING.md`, `NOTICE`, `LICENSE` and
`credits/`.

### Browser build (Phase 5)

The Trunk configuration from the template is kept for the later browser
build. It is not exercised yet.

- Install [trunk](https://github.com/trunk-rs/trunk): `cargo install --locked trunk`
- Install the target: `rustup target add wasm32-unknown-unknown`
  (Arch: `sudo pacman -S rust-wasm`)
- Serve locally: `trunk serve`
- Deploy: run the `deploy-github-page` workflow, then enable GitHub Pages from
  the `gh-pages` branch. Re-run the workflow to deploy a newer version.

## Layout

```
Cargo.toml                 workspace: members = ["crates/*"], shared [workspace.dependencies], lints
rust-toolchain.toml  clippy.toml
crates/
  mc-core/                 Java RNG, math, AABB, Java-order collections (no deps)
  mc-nbt/                  NBT tags 0-10 (flate2)
  mc-sim/                  the simulation: block, item, world, worldgen, entity, inventory, save
  mc-golden/               golden-fixture harness (serde, serde_json)
  mc-client/               Bevy shell: ClientPlugin
  mc-app/                  the binary `mc_b173`: window, icon, canvas setup
tools/oracle/              Java harness sources; build.sh, regen.sh
tests/golden/<area>/<name>/{scenario.json, expected.jsonl}
docs/                      PLAN_v*.md, ADRs, mappings.md, REFERENCES.md
credits/                   CREDITS.md, licenses/
assets/                    project-owned files only
build/  index.html  Trunk.toml   web and packaging (template)
.github/workflows/         ci.yml (+ wasm check, golden-regen), release.yaml, deploy-page.yaml
.reference/                git-ignored inputs (docs/REFERENCES.md)
```

## Plan

The master plan is [`docs/PLAN_v4.md`](docs/PLAN_v4.md). Earlier versions and
the blueprint are kept in `docs/` as history.

## Credits

> Parts of this project are adapted from
> [MinecraftOSS](https://github.com/chasmlol/2010-rust-rewrite-mashup) by
> **chasmlol**, from the `2010-rust-rewrite-mashup` repository (commit
> `4013a68`), used under the Apache License 2.0. Adapted files are marked in
> their headers; see `NOTICE` and `credits/licenses/MinecraftOSS_Apache-2.0.md`.

`crates/mc-core/src/strict_math.rs` transcribes fdlibm 5.3 (Sun Microsystems; freely
redistributable provided its notice is preserved, which the file does) so that
`mc_core::jmath` reproduces Java's `StrictMath` bit for bit.

The window and application icons are the Bevy icon, used under its
[MIT licence](credits/licenses/Bevy_MIT_License.md).

## Licence

This project's own code is released under [CC0 1.0 Universal](LICENSE), as
inherited from the Bevy game template it was generated from. Code adapted from
MinecraftOSS is under the Apache License 2.0, as noted above and in `NOTICE`.
Nothing of Mojang's is included.
