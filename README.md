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
(`docs/PARITY_CHECKLIST.md`, `docs/GATE4_REVIEW.md`). See `docs/PLAN_v4.md` §5 for the phases and work
packages.

## Requirements

- Rust 1.99.0 via `rustup` (`rust-toolchain.toml` selects it; install rustup
  from https://rustup.rs or your distro's `rustup` package, then run
  `rustup show` in the checkout). `cargo` must be rustup's proxy: put
  `~/.cargo/bin` before the distro's `/usr/bin/cargo` on PATH.
- A JDK (21 or newer) to build and run the Java oracle that generates fixtures.
- A Prism Launcher `b1.7.3` instance, launched at least once: the game reads
  the client jar (textures and `lang/` files) and the `resources/` directory
  (sounds and music) from it. **This project ships no Mojang assets**; the
  instance is auto-detected or given with `--prism-instance`; see
  `docs/RUNNING.md`.
- The read-only reference tree under `.reference/` for development; see
  `docs/REFERENCES.md`. It is git-ignored and never committed.

Since Phase 3 the game is playable from the title screen (create, import,
play, save, reload; `docs/GATE3_REVIEW.md`). How to set up the Prism instance
and the command-line flags: [`docs/RUNNING.md`](docs/RUNNING.md).
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
`credits/`. Linux gets both a tarball and an x86_64 AppImage (built on Ubuntu
22.04, glibc 2.35 or newer).

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
  mc-app/                  the binary `mc_b173`: window, icon
tools/oracle/              Java harness sources; build.sh, regen.sh
tests/golden/<area>/<name>/{scenario.json, expected.jsonl}
docs/                      PLAN_v*.md, ADRs, mappings.md, REFERENCES.md
credits/                   CREDITS.md, licenses/
assets/                    project-owned files only
build/                     icons, macOS .app template, Windows icon and installer
.github/workflows/         ci.yml (+ golden-regen, soak), release.yaml
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
