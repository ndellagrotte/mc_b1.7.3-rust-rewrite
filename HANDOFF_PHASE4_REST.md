# Handoff: the rest of Phase 4 (PLAN_v4 §5)

Written 2026-10-10 by the orchestrating session after the Phase 3 and Phase 4
waves. Untracked, like the other `HANDOFF_*` files. Companion files at the
repo root, also untracked:

- `HANDOFF_PHASE4_wave.js` — the Workflow script every wave since Phase 3 ran
  (transcribe → review against the Java → fix → integrator merge → optional
  gate).
- `HANDOFF_PHASE4_wave-args.json` — the ready-to-run arguments for the
  remaining wave (WP-4.7, 4.8, 4.9) and the Gate 4 review prompt.
- `HANDOFF_PHASE4_mkwt.sh` — the worktree helper (`mkwt.sh <id> <base-ref>`).

## 1. Where things stand

| Item | State |
|---|---|
| Integration branch | `phase-4`, tip `d222a15` (36 commits over `phase-3`; 301 over `main`). Nothing is pushed. Linear history, one commit per WP step. |
| Phase 3 | Complete. Gate 3 passed on all eight criteria (`docs/GATE3_REVIEW.md`, commit `d52ac6b`). |
| Phase 4 wave E | Merged: WP-4.2 soak, WP-3.9 (Gate 3 defects), WP-4.6 (sim parity gaps), WP-4.4 (deferred review), WP-4.5 (export), WP-4.3 (perf + packaging), WP-4.1 (parity checklist). |
| Tests | 1165 workspace tests pass, 5 soak tests ignored by default (`cargo test -p mc-golden --test golden_soak -- --ignored`, ~5 min, all pass). |
| Fixtures | 231 scenario dirs under `tests/golden` (226 default + 5 soak); `sh tools/oracle/regen.sh` and `SOAK=only sh tools/oracle/regen.sh` change nothing. |
| Gaps | 47 `deferred` tags in `crates/mc-client/src` + `crates/mc-sim/src`; 97 `note_unimplemented(` sites in `mc-sim`, all classified in `docs/DEFERRED_REVIEW.md` §3. |
| Speed | Simulation 2,000–3,000 ticks/s on the soaks (about the oracle's 2,300–2,400); client at the vsync cap by default (`docs/PERF.md`). |
| Worktrees | `.worktrees/wp-4.7`, `wp-4.8`, `wp-4.9` exist on branches `wp-4.7/4.8/4.9` at `d222a15`, each with the `.reference` symlink and a pre-warmed `target/` (a background build was running when this was written; if `cargo build -p mc-app` in one takes ~4 min, the warm-up had not reached it). |

Documents Phase 4 produced, all on `phase-4`:

| Doc | Content |
|---|---|
| `docs/PARITY_CHECKLIST.md` | WP-4.1. The original b1.7.3 jar was driven side by side (Prism's Java 8, `-Duser.home` fake home, nested Xwayland, xdotool). 137 screenshot pairs, 33 sound stations. Defects P1–P10 in §2; P1/P2 are closed by WP-3.9. §9 lists differences that are not bugs. §11 is the repro recipe. |
| `docs/PERF.md` | WP-4.3. Before/after tables, methodology, write-behind autosave, `--vsync`, dump_debug histogram; §6 owner questions (unsafe prefetch, dev opt-level, write-behind acceptance, zlib-rs); §7 known issues incl. the quick-quit segfault. |
| `docs/RUNNING.md` | How to run the game: data dir, `--jar`/`--resources` defaults and the D8 error, flags. |
| `docs/DEFERRED_REVIEW.md` | WP-4.4. Every `deferred` tag audited (one mislabel fixed), the 97 gap sites by family, §3.6 proposed WP-4.7/4.8, §4 the PLAN_v5 WP table, §5 the owner-question backlog, §6 the binary-file/licence audit. |
| `docs/EXPORT.md` | WP-4.5. Export action, validation, verification in the real client. |
| `docs/WP4.6_SIM_GAPS.md` | WP-4.6. The sim gaps closed (cake heal, pig→pigman, alertWolves tamed branch, kill hooks, chest-minecart inventory) and what stays open. |

## 2. What is left in Phase 4

1. **Wave F** (three WPs, disjoint files, run in parallel; merge order 4.8, 4.7, 4.9):
   - **WP-4.7 Simulation parity remainder**: P4 (a dead player must be saved in
     `level.dat` and the world reopen on Game Over), stats autosave
     (`StatFileWriter.func_27178_d` family, wrongly tagged deferred), removed
     references with a parity effect (arrow/fireball shooter, mob look target,
     fireball collision border), container close timing, stale docs in 4.6's
     files. Oracle fixtures where the harness reaches them.
   - **WP-4.8 Gap hygiene**: surface `unimplemented_hits`/`skipped_systems` in
     `dump_debug` and a per-fixture gaps file, trace the removed-entity family,
     the coverage/cross-read test honesty items, dead code and render-only arms,
     citation nits. Behaviour-neutral.
   - **WP-4.9 Client parity fixes**: P3 (dead player's view draws nothing but
     terrain), P5 (F3 memory lines), P6 (frame cap: vanilla is uncapped except
     Power saver), P7 (`portal.travel` timing), P8 (one untextured frame after
     loading), P9/P10 to confirm (loading bar; renderer rebuild policy vs
     `RenderGlobal.updateRenderers`), the quick-quit segfault, the rendering
     follow-ups (inventory preview overlay, pass-1 sort, anaglyph second eye,
     floating origin or its design).
   The full briefs are in `HANDOFF_PHASE4_wave-args.json`; each names its spec
   files, Rust target, fixtures/tests, smoke and out-of-scope.
2. **Gate 4 review** → `docs/GATE4_REVIEW.md` (prompt in the same JSON under
   `gate`). It re-exercises the Gate 3 path, the dead-player path, a walked
   portal round trip, export → import, the chest minecart, F3 memory, frame
   rate per performance option, 20 quick quits, and a 10-station spot check
   against the real client. `gate_pass` only if parity holds on every exercised
   criterion.
3. **Owner decisions pending** (none were taken by the orchestrator; the docs
   record the defaults chosen):
   - `docs/PERF.md` §6: `unsafe` prefetch in `mc-sim` (policy: `forbid(unsafe_code)`),
     the dev profile's opt-level, write-behind autosave acceptance, `zlib-rs`
     as the flate2 backend.
   - P6: whether Balanced/Max should run uncapped like vanilla (WP-4.9
     implements the vanilla default with `--vsync` as the override).
   - `docs/DEFERRED_REVIEW.md` §5: the deduplicated owner-question backlog from
     Gates 0–3 (Entity.rand rewrite / PLAN_v4 §8 item 3; testWorld licence; the
     project-owned data dir; the import/export screens being path fields; the
     mc-sim interface changes of Phase 3; nether travel assigned to 3.1/3.1b).
   - `docs/DEFERRED_REVIEW.md` §4: the PLAN_v5 WP table (what was split/added).

## 3. How to run the wave

In the orchestrating session (ultracode on), with the worktrees already in
place:

```text
Workflow({ scriptPath: "<repo>/HANDOFF_PHASE4_wave.js",
           args: <contents of HANDOFF_PHASE4_wave-args.json> })
```

The script reads `args.base`, `args.branch`, `args.state`, `args.wps[]`
(`id`, `name`, `brief`), `args.mergeOrder`, optional `args.mergeOnly[]` (skip
transcription, integrate an existing `wp-<id>` branch) and optional
`args.gate {label, prompt}` (`{MERGES}` in the prompt is replaced by the merge
reports). Every agent is `model: 'opus'`; never `fable`. Per WP it runs
transcribe → review (sees only the diff and the Java, runs the DoD and the
smoke) → fix (if findings) → integrator (rebase onto the branch, DoD, ff-merge
in the main checkout). Expect ~5–6 h wall clock for three WPs plus the gate.

If a worktree is missing: `sh HANDOFF_PHASE4_mkwt.sh 4.7 phase-4` (creates
`.worktrees/wp-4.7` on branch `wp-4.7`, the `.reference` symlink, and a
reflinked `target/`; `/home` is btrfs). Pre-warm with
`cd .worktrees/wp-4.7 && cargo build -p mc-app` (~4 min cold).

After the wave: `git worktree remove --force .worktrees/wp-4.x` for each, keep
the `wp-*` branches (the owner reviews them), do not push.

## 4. Rules and lessons that are not in the docs

- **Shared files are additive-only** across parallel WPs:
  `crates/mc-client/src/script.rs`, `lib.rs`, `README.md`, `docs/mappings.md`.
  Every other file has one owner per wave; the integrator resolves the rest.
- **Never `SendMessage` a running workflow agent.** It resumes the transcript
  as a second instance that edits the same worktree (happened to WP-4.6).
  Let the agent act on its stated default, or put the answer in the next brief.
- **Usage limits kill the whole wave.** A wave that dies on the weekly limit
  leaves partial edits in the worktrees; `git checkout -- .` / `git clean` them
  (only untracked `.reference` should remain) and relaunch with the same args.
- The harness attribution reminder makes subagents write
  `Co-Authored-By: Claude Opus 5.5`; the integrators' own commits say
  `Claude Fable 5.1`. Harmless; the owner was told at every hand-off.
- The vanilla client recipe (`docs/PARITY_CHECKLIST.md` §1.1, §11) uses a
  local, never-committed copy of `minecraft.jar` with its signature files
  removed and a tiny launcher class; `build/screens/` is git-ignored, so the
  helper scripts may need to be rebuilt from the doc's description.
- Oracle: `tools/oracle/regen.sh` (~45 s), `SOAK=only` for the five soaks
  (~2 min), `check-reproducible.sh`, `check-sensitivity.sh`. Never hand-edit an
  `expected.jsonl`.
- The reviewer prompt in the script checks D7/D8: no Mojang PNG/OGG/lang ever
  committed; screenshots live under `build/screens/<wp>/` or the scratchpad.

## 5. After Phase 4

PLAN_v4 Phase 5 (browser build) starts from the D9 guarantees: `mc-core`,
`mc-nbt`, `mc-sim` already pass `cargo check --target wasm32-unknown-unknown`
in CI; the client needs a browser `Storage`, the file-picker for
`minecraft.jar`/`resources/`, audio start on gesture and a single-threaded
meshing budget (`crates/mc-client/src/terrain/` already runs on Bevy's task
pool, which degrades to one thread). Phase 6 items stay owner-gated (D3).
