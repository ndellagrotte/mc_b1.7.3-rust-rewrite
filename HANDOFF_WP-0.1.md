# Handoff — WP-0.1: adapt the template repository into the project workspace

You are a fresh agent. You have no prior context on this project; everything you
need is in this file and in the plan it points to. Read both fully before
changing anything.

## 1. Situation

- **Project:** a Rust + Bevy rewrite of Minecraft Beta 1.7.3 with vanilla
  singleplayer parity, proven by fixtures generated from the vanilla Java
  source. Master plan: `/home/nick/Downloads/Beta-1.7.3-main/PLAN_v4.md`
  (you will move it into this repository in step 1; read it first, at least
  §2 Decisions, §3.1 Repository layout and §5 Phase 0 / WP-0.1).
- **Repository:** `/home/nick/IdeaProjects/mc_b1.7.3-rust-rewrite`, branch
  `main`, one commit ("Initial commit"), remote
  `github.com/ndellagrotte/mc_b1.7.3-rust-rewrite`. It was generated from
  Niklas Eicker's `bevy_game_template` (Bevy 0.19) and is untouched since.
- **Toolchain:** `cargo 1.99.0` / `rustc 1.99.0` from the Arch `rust` package.
  There is **no `rustup`**, and the `wasm32-unknown-unknown` standard library
  is **not installed**. The owner has been asked to install it
  (`sudo pacman -S rust-wasm`). Do not try to install it yourself; if it is
  still absent when you reach step 7, report that the wasm check could not run.
- **Reference material** (read-only, Mojang-derived, never committed) is
  currently at `/home/nick/Downloads/Beta-1.7.3-main/.reference/` and moves
  into this repository in step 1. Contents:
  - `mc_b1.7.3_release/` — vanilla b1.7.3 source, MCP-deobfuscated; git clone of
    `https://github.com/jacobo-mc/mc_b1.7.3_release.git`, commit
    `740c583901e1ff1150e9ef37e37dab5bc0e4f807`. The spec for the whole project.
  - `mc_b1.7.3-web-port/` — a TeaVM browser port (not a git repo, from a
    GitHub zip of `PeytonPlayz595/Beta-1.7.3`). Not a source for anything;
    kept as a worked example of a browser build.
  - `2010-rust-rewrite-mashup-main/` — contains MinecraftOSS crates (not a git
    repo; from a zip of `chasmlol/2010-rust-rewrite-mashup`, MinecraftOSS
    commit `4013a68` per its README). Apache-2.0; code may be adapted with
    attribution (plan D4).
  - `mc_b1.7.3-world/testWorld/` — a vanilla McRegion test world, seed 123.
- **Plan documents** at `/home/nick/Downloads/Beta-1.7.3-main/`:
  `PLAN_v1.md` … `PLAN_v4.md`, `RUST_REWRITE_BLUEPRINT.md`,
  `docs/ADR-007-golden-harness.md`. `PLAN_v4.md` is current; the others are
  history and are kept.

## 2. Goal of WP-0.1

Turn the template into the empty-but-complete workspace described in
`PLAN_v4.md` §3.1, with reference material and documents in place, licences and
credits settled, lint rules and CI wired, and every crate compiling as a stub.
No game code. No oracle. No fixtures. Those are WP-0.2 onward.

Work on a branch `wp-0.1`. Commit after each numbered step below with a short
message naming the step. Do not push; the owner reviews the tree locally
(Gate 0 requires their approval of this tree).

## 3. Steps

### Step 1 — Move references and documents; ignore them properly

1. `git mv` nothing here; these come from outside the repo. Use `mv`, not
   `cp` (the reference tree is ~260 MB):
   - `/home/nick/Downloads/Beta-1.7.3-main/.reference` → `<repo>/.reference`
   - `/home/nick/Downloads/Beta-1.7.3-main/PLAN_v*.md`,
     `RUST_REWRITE_BLUEPRINT.md`, `docs/ADR-007-golden-harness.md` →
     `<repo>/docs/` (flat; create `docs/`).
   - Leave `/home/nick/Downloads/Beta-1.7.3-main/.classpath` and the now-empty
     directories alone; do not delete the Downloads folder.
2. Append to `.gitignore` (keep the template's existing entries):
   ```
   # Mojang-derived reference material: never committed (PLAN_v4 D8)
   /.reference/
   # oracle build products (PLAN_v4 §4.1)
   /build/oracle/
   /build/oracle-src/
   /build/golden-out/
   ```
   Note `build/` itself stays tracked: the template keeps icons and web CSS
   there.
3. Write `docs/REFERENCES.md` listing each `.reference/` entry: what it is,
   where it came from (URLs above), the commit or "zip, no VCS", its licence as
   stated in-tree (vanilla release: MIT per its `LICENSE`, attributed to
   "Modification Station and Mojang AB"; web port: MIT; mashup: Apache-2.0),
   and the rule that the directory is frozen and never edited. Record that the
   vanilla release is frozen at commit `740c583…` and that this does not
   matter to anyone (owner decision).
4. Verify `git status` shows no file under `.reference/` as untracked.

### Step 2 — Virtual workspace with six crates

Replace the root `Cargo.toml` with a virtual workspace:

```toml
[workspace]
resolver = "3"          # or "2" if 3 is rejected by this cargo
members = ["crates/*"]

[workspace.package]
version = "0.1.0"
edition = "2024"
publish = false
license = "CC0-1.0"     # the template's LICENSE; the owner may change it later

[workspace.dependencies]
bevy = { version = "0.19.0", default-features = false, features = ["2d", "3d", "ui"] }
bevy_kira_audio = "0.26"
bevy_asset_loader = "0.27.0"
flate2 = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
zip = { version = "2", default-features = false, features = ["deflate"] }
# keep in sync with Bevy's dependencies (template note)
winit = { version = "0.30", default-features = false }
image = { version = "0.25", default-features = false }
log = { version = "0.4", features = ["max_level_debug", "release_max_level_warn"] }

[workspace.lints.clippy]
excessive_precision = "allow"   # transcribed Java float literals keep their digits (PLAN_v4 D4)
approx_constant = "allow"
disallowed_types = "deny"
disallowed_methods = "deny"
```

Move the template's `[profile.*]` sections (dev opt-level 1 with
`[profile.dev.package."*"] opt-level = 3`, release, dist) to the workspace
root unchanged. Keep the template's `getrandom` wasm note only if a crate still
needs `getrandom`; the simulation crates must not.

Create `crates/` with these packages, each inheriting `version`, `edition`,
`publish`, `license` and `lints` from the workspace (`*.workspace = true`):

| Crate | Kind | Dependencies | Stub contents |
|---|---|---|---|
| `mc-core` | lib | none | `lib.rs` with a doc comment stating purpose (RNG, math, AABB, Java-order collections) |
| `mc-nbt` | lib | `flate2` | doc comment |
| `mc-sim` | lib | `mc-core`, `mc-nbt` | doc comment; empty modules `block`, `item`, `world`, `gen`, `entity`, `inventory`, `save` as files, each with a one-line doc comment |
| `mc-golden` | lib | `mc-sim`, `serde`, `serde_json` | doc comment |
| `mc-client` | lib | `mc-sim`, `bevy`, `bevy_kira_audio`, `bevy_asset_loader`, `image`, `log`, `zip` | `pub struct ClientPlugin; impl Plugin for ClientPlugin { fn build(&self, _app: &mut App) {} }` |
| `mc-app` | bin, name `mc_b173` | `mc-client`, `bevy`, `winit`, `image`, `log`; build-dep `embed-resource = "1"` | the template's `src/main.rs` and `build.rs`, adapted: window title `Minecraft Beta 1.7.3 (Rust)`, `ClearColor` left as is, `GamePlugin` → `mc_client::ClientPlugin`, icon path still `build/macos/icon_1024x1024.png` relative to the repo root (fix the `include_bytes!` path — the binary now lives in `crates/mc-app/src/`) |

Give `mc-app` a `[features] dev = ["bevy/dynamic_linking"]` like the template.
`mc-core`, `mc-nbt` and `mc-sim` must have **no** dependency on `bevy`,
`rand`, `getrandom`, or anything that pulls in `std::fs`/threads (PLAN_v4 D9).

### Step 3 — Remove template leftovers

Delete: `src/` (entire template example game), `mobile/`,
`.github/workflows/release-android-google-play.yaml`,
`.github/workflows/release-ios-testflight.yaml`, `assets/audio/flying.ogg`,
`assets/textures/bevy.png`, `assets/textures/github.png`, the `webbrowser` and
`rand` dependencies (not re-added anywhere). Keep `assets/` as an empty tracked
directory with a `README.md` saying it holds project-owned files only, never
Mojang assets (PLAN_v4 D8). Remove the Bevy-icon entry from
`credits/CREDITS.md` and `credits/licenses/Bevy_MIT_License.md` if the Bevy
icon PNGs are gone; keep them if you keep the icon files for the window icon
(decide one way and be consistent — keeping them is fine).

Rename every `bevy_game` / `Bevy game` / `BevyGame` / `ToDo` site. Known
locations: `index.html` (title), `.github/workflows/release.yaml`
(`GAME_EXECUTABLE_NAME` → `mc_b173`, and the macOS app name variable),
`build.rs`, `Trunk.toml` (check), `nix/flake.nix` (check). The nix flake may
be deleted entirely unless `nix` is in use on this machine (`which nix`); if
deleted, also delete `flake.lock` and `nix/`.

Grep for `ToDo`, `bevy_game`, `nikl` afterwards; the only remaining hits may
be in `docs/` history files.

### Step 4 — Lint configuration

Create `clippy.toml` at the root:

```toml
# PLAN_v4 D9: the simulation crates must compile for the browser and stay deterministic.
disallowed-types = [
    { path = "std::fs::File", reason = "simulation crates use the Storage trait (PLAN_v4 D9)" },
    { path = "std::thread::JoinHandle", reason = "simulation is single-threaded (PLAN_v4 D9)" },
    { path = "std::time::Instant", reason = "time is passed in by the client (PLAN_v4 D9)" },
    { path = "std::time::SystemTime", reason = "time is passed in by the client (PLAN_v4 D9)" },
    { path = "std::collections::HashMap", reason = "use BTreeMap, Vec, or mc_core::JavaHashMap where Java iterates (PLAN_v4 §3.4)" },
    { path = "std::collections::HashSet", reason = "use BTreeSet, Vec, or mc_core::JavaHashSet where Java iterates (PLAN_v4 §3.4)" },
]
disallowed-methods = [
    { path = "std::fs::read", reason = "Storage trait (PLAN_v4 D9)" },
    { path = "std::fs::write", reason = "Storage trait (PLAN_v4 D9)" },
    { path = "std::fs::read_dir", reason = "Storage trait (PLAN_v4 D9)" },
    { path = "std::thread::spawn", reason = "single-threaded simulation (PLAN_v4 D9)" },
    { path = "std::env::var", reason = "no environment access in simulation crates (PLAN_v4 D9)" },
]
```

`clippy.toml` applies to the whole workspace, which is intentional for now:
`mc-client` and `mc-app` will need `#[allow(clippy::disallowed_types)]` /
`disallowed_methods` at the few sites that legitimately touch files or clocks
(the native `Storage` implementation, the frame clock). Add a comment in
`clippy.toml` saying so. The `HashMap`/`HashSet` entries exist because PLAN_v4
§3.4 rule 4 wants every hash-iteration site to be a deliberate choice.

Create `rust-toolchain.toml` recording the current stable (`channel = "1.99.0"`,
components `rustfmt`, `clippy`) with a comment that the Arch system toolchain
ignores it and that it exists for `rustup` users and CI. Delete
`nix/rust-toolchain.toml` with the nix directory, or update it to match if
nix is kept.

### Step 5 — CI

Edit `.github/workflows/ci.yml`:

- Keep the three existing jobs (`test`, `all-doc-tests`, `lint`). They already
  run `cargo clippy --workspace --all-targets --all-features -- -Dwarnings` and
  `cargo fmt --all -- --check`.
- Add a job `wasm-check` on `ubuntu-latest`: `dtolnay/rust-toolchain@stable`
  with `targets: wasm32-unknown-unknown`, then
  `cargo check -p mc-core -p mc-nbt -p mc-sim --target wasm32-unknown-unknown`.
- Add a job `golden-regen` triggered only by `workflow_dispatch`, that checks
  for `.reference/mc_b1.7.3_release` and, if absent, prints
  "reference tree not available on this runner; run tools/oracle/regen.sh
  locally" and exits 0; if present, installs a JDK (`actions/setup-java`,
  Temurin 21) and runs `tools/oracle/regen.sh`. Create
  `tools/oracle/regen.sh` as a stub that prints "WP-0.2 not yet implemented"
  and exits 1, with the executable bit set, so the job is honest.

Keep `release.yaml` and `deploy-page.yaml`; fix their names per step 3 and
remove mobile references inside them if any remain.

### Step 6 — Licences, credits, README

1. `NOTICE` at the root:
   ```
   mc_b1.7.3-rust-rewrite

   This project contains code adapted from MinecraftOSS by chasmlol, from the
   2010-rust-rewrite-mashup repository
   (https://github.com/chasmlol/2010-rust-rewrite-mashup), MinecraftOSS commit
   4013a68, licensed under the Apache License, Version 2.0. Adapted files carry
   a header identifying their origin. A copy of the licence is in
   credits/licenses/MinecraftOSS_Apache-2.0.md.
   ```
2. `credits/licenses/MinecraftOSS_Apache-2.0.md`: the Apache License 2.0 text.
   Copy it from `.reference/2010-rust-rewrite-mashup-main/LICENSE` (that file
   is the licence text) and prefix a line naming MinecraftOSS / chasmlol.
3. `credits/CREDITS.md`: add a "Code" section with exactly this text:

   > Parts of this project are adapted from
   > [MinecraftOSS](https://github.com/chasmlol/2010-rust-rewrite-mashup) by
   > **chasmlol**, from the `2010-rust-rewrite-mashup` repository (commit
   > `4013a68`), used under the Apache License 2.0. Adapted files are marked in
   > their headers; see `NOTICE` and `credits/licenses/MinecraftOSS_Apache-2.0.md`.

4. `README.md`: replace the template body. Sections: one-paragraph description
   (Rust + Bevy rewrite of Minecraft Beta 1.7.3 aiming at vanilla singleplayer
   parity); **Status** (Phase 0, nothing playable); **Requirements** (Rust
   stable, a JDK for the oracle, your own `minecraft.jar` for b1.7.3 and a
   `resources/` directory for sounds — this project ships no Mojang assets);
   **Build and run** (`cargo run -p mc-app`, `cargo test --workspace`,
   `cargo clippy --workspace -- -Dwarnings`); **Layout** (the tree from
   PLAN_v4 §3.1); **Plan** (link to `docs/PLAN_v4.md`); **Credits** (the same
   text as in `credits/CREDITS.md`); **Licence** (CC0 for this project's own
   code, per `LICENSE`; adapted code under Apache-2.0 as noted). Keep the
   template's web-build instructions in a short "Browser build (Phase 5)"
   subsection since Trunk config is retained.

### Step 7 — Verify

Run from the repo root and record the output in your report:

```
cargo build --workspace
cargo clippy --workspace --all-targets -- -Dwarnings
cargo fmt --all -- --check
cargo test --workspace
cargo check -p mc-core -p mc-nbt -p mc-sim --target wasm32-unknown-unknown   # may fail if target not installed
cargo run -p mc-app   # should open an empty window with the icon and title; close it
grep -rn "ToDo\|bevy_game\|nikl" --exclude-dir=target --exclude-dir=.reference --exclude-dir=docs . || echo clean
git status --short | grep -c "^?? .reference" ; # must print 0
```

Expect the first `cargo build` to take several minutes (Bevy). If
`cargo run -p mc-app` cannot open a window in this environment, say so; a
build success is enough for this step.

## 4. Rules

- Never edit, move within, or delete anything under `.reference/` after step 1.
  Never commit anything from it.
- Do not write game logic, oracle code, or fixtures. Stubs only.
- Do not add dependencies beyond those listed without noting it in the report.
- Do not push. Do not change `LICENSE`.
- If a step is ambiguous, pick the option that leaves less in the tree and
  note the choice in the report.

## 5. Report

End with a report containing: the commit list on `wp-0.1`; the exact output of
each step-7 command (trimmed to the last lines where long); the final `tree -L 2
-a -I 'target|.git|.reference'` of the repo; decisions you made where this
handoff left a choice; anything that failed or could not be verified (expected:
the wasm check if the target is still missing).
