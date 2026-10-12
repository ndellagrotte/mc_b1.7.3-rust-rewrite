export const meta = {
  name: 'phase3-wave',
  description: 'PLAN_v4 Phase 3/4 wave (WPs in args): transcribe in a worktree, review against the Java, fix, integrator merge; optional gate review',
  phases: [
    { title: 'Transcribe', detail: 'one opus agent per WP in its own worktree' },
    { title: 'Review', detail: 'reviewer sees only the diff, the Java and a smoke run' },
    { title: 'Fix', detail: 'apply confirmed findings, re-run DoD' },
    { title: 'Merge', detail: 'sequential rebase onto the integration branch' },
    { title: 'Gate', detail: 'optional gate review from args.gate' },
  ],
}

const ROOT = '/home/nick/IdeaProjects/mc_b1.7.3-rust-rewrite'
const JAVA = ROOT + '/.reference/mc_b1.7.3_release/1.7.3-LTS/src/minecraft/net/minecraft/src'
const BASE = args.base
const STATE = args.state || ''
const BRANCH = args.branch
const WPS = args.wps || []
const ORDER = args.mergeOrder || WPS.map(w => w.id)
const MERGE_ONLY = args.mergeOnly || []

const COMMON = [
'You are a *transcriber* agent on the Rust/Bevy rewrite of Minecraft Beta 1.7.3 (master plan: ' + ROOT + '/docs/PLAN_v4.md; read §2 (esp. D1, D4, D7, D8, D9), §3 (esp. §3.2 side-effects, §3.3 tick order, §3.5 client notes), §6, and your WP row in §5 before anything else). You work ONLY inside the git worktree named below (a checkout of branch ' + BRANCH + ' at ' + BASE + '). Orchestrator state notes: ' + STATE + ' Treat every other path as read-only, and never edit anything under .reference/ (Mojang-derived, frozen; the worktree has a symlink to it).',
'',
'Spec tree (D2): ' + JAVA + ' (and ' + ROOT + '/.reference/mc_b1.7.3_release/1.7.3-LTS/src/minecraft/net/minecraft/client/Minecraft.java). Read every spec file for your task IN FULL before writing Rust. Citations are "// src: File.java:line" and resolve against that tree. Transcribe vanilla bugs, never fix them; mark them "// vanilla bug:". No other Java tree (not the web port under .reference/mc_b1.7.3-web-port, not the server tree) is a source. MinecraftOSS code (.reference/2010-rust-rewrite-mashup-main) may be adapted only from the D4 table rows, with the D4 origin header on each adapted file; its generator/entities/world/level/player code is forbidden.',
'',
'Rendering rule (D7): rendering parity is visual, not structural. Game-data parts are transcribed exactly with citations (faces, texture indices, colours, brightness/light formulas, model boxes and rotation points, GUI pixel coordinates, timings, option names and defaults); the drawing machinery (Bevy meshes, materials, shaders, cameras, UI) is written for Bevy 0.19 (the version in Cargo.toml; read the Bevy sources in ~/.cargo/registry when unsure about an API, do not guess). D8: nothing of Mojang\'s is embedded or committed: textures and lang/ files are read at runtime from the user\'s minecraft.jar (default ' + ROOT + '/.reference/mc_b1.7.3_release/1.7.3-LTS/jars/bin/minecraft.jar), sounds/music from a resources/ directory (default /home/nick/.var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher/instances/b1.7.3/minecraft/resources). Never commit a PNG/OGG derived from them; screenshots go under build/screens/ (untracked; add a .gitignore entry if needed) or the scratchpad.',
'',
'Existing code you must build on (read it first, do not restructure it): crates/mc-client (the Bevy shell; module tree created by WP-3.1), crates/mc-app (binary), crates/mc-sim (world/ with events.rs = the IWorldAccess event queue the client drains, block/ with the const BLOCKS table and block_texture.rs, item/, entity/, tile_entity/, save/ incl. storage.rs = the Storage trait and save_format_old.rs/save_handler.rs, player_controller.rs, tick.rs), crates/mc-golden/src/tick_driver.rs (shows how a world and player are built, loaded, saved and ticked), crates/mc-core (JavaRandom, MathHelper sine table, jmath), docs/mappings.md. The simulation crates stay free of std::fs/threads/clocks/env/rand (D9); the client passes time and bytes in. Follow the conventions in the module headers of the files you edit. Interface changes to mc-sim outside your Rust target: make them only if your brief grants the file, otherwise report them.',
'',
'Determinism rules (§3.4) apply to anything that reaches the simulation (player input → EntityPlayer, block picks, item use, dimension change). Client-only state (camera, GUI, particles, meshes) is f32 where the Java is float and keeps the Java arithmetic order so visuals match.',
'',
'Definition of Done for client WPs: (1) cargo build -p mc-app; cargo clippy --workspace --all-targets --all-features -- -Dwarnings; cargo fmt --all --check; cargo test --workspace (or at least -p mc-client plus every crate you touched) — all pass. (2) A smoke run: the binary started with the scripted-input mode (WP-3.1 defines it: a JSON script of timed key/mouse actions plus "screenshot" and "quit" actions; read crates/mc-client/README.md) on a fresh seed world and on the import of ' + ROOT + '/.reference/mc_b1.7.3-world/testWorld, producing screenshots you LOOK AT (Read the PNG) and describe in the report; a Wayland display is available (WAYLAND_DISPLAY=wayland-0, DISPLAY=:0). (3) Unit tests for every transcribed formula/table that can be tested without a GPU (e.g. option parsing, brightness tables, texture-index tables, model box tables, timer math, colorizers). (4) Every non-obvious constant and ordering has a src: citation. (5) Every field_NNN/func_NNN you touch gets a row in docs/mappings.md. (6) No edits outside your Rust target. (7) Report vanilla bugs transcribed, deferred code skipped (D3: multiplayer, texture packs, stats sync, skins/cloaks, resource downloads, sky dimension), anything uncertain, anything unverified visually.',
'',
'Git: commit in your worktree on its branch, one commit per logical step, message "WP-<id> step N: <what>", ending with a blank line and "Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>". Do NOT push, do not touch other branches, no git commands outside your worktree, never use git stash. Stage only files you meant to change (never build/, target/, screenshots; .reference is an untracked symlink, leave it). Finish with a clean working tree.',
'',
'Delegation: you may spawn your own subagents (model "opus" for transcription/review, "sonnet" only for narrow mechanical work); never "fable". Use the rust-analyzer LSP tool when it helps (ToolSearch "select:LSP"). A cold cargo build in a fresh worktree takes ~4 min; later builds are incremental. Work until the task is complete; do not stop at a partial result. Your final text is a machine-read report, not a chat message.',
].join('\n')

const REPORT = {
  type: 'object',
  properties: {
    summary: { type: 'string' },
    branch: { type: 'string' },
    worktree: { type: 'string' },
    commits: { type: 'array', items: { type: 'string' } },
    files_touched: { type: 'array', items: { type: 'string' } },
    interfaces_added: { type: 'array', items: { type: 'string' }, description: 'public Rust items other WPs will call (type/fn path + one line)' },
    smoke_run: { type: 'object', properties: { command: { type: 'string' }, screenshots: { type: 'array', items: { type: 'string' } }, observed: { type: 'string' } }, required: ['command', 'screenshots', 'observed'] },
    dod: { type: 'object', properties: { build: { type: 'boolean' }, clippy: { type: 'boolean' }, fmt: { type: 'boolean' }, tests: { type: 'boolean' }, notes: { type: 'string' } }, required: ['build', 'clippy', 'fmt', 'tests'] },
    vanilla_bugs: { type: 'array', items: { type: 'string' } },
    deferred: { type: 'array', items: { type: 'string' } },
    uncertain: { type: 'array', items: { type: 'string' } },
    unverified_visually: { type: 'array', items: { type: 'string' } },
    interface_changes_needed: { type: 'array', items: { type: 'string' } },
  },
  required: ['summary', 'branch', 'worktree', 'commits', 'files_touched', 'interfaces_added', 'smoke_run', 'dod', 'vanilla_bugs', 'deferred', 'uncertain', 'unverified_visually', 'interface_changes_needed'],
}

const REVIEW = {
  type: 'object',
  properties: {
    verdict: { type: 'string', enum: ['clean', 'fix-needed'] },
    findings: { type: 'array', items: { type: 'object', properties: {
      severity: { type: 'string', enum: ['behavioural', 'visual', 'citation', 'nit'] },
      file: { type: 'string' }, line: { type: 'integer' }, java: { type: 'string' }, problem: { type: 'string' }, fix: { type: 'string' } },
      required: ['severity', 'file', 'line', 'java', 'problem', 'fix'] } },
    coverage_gaps: { type: 'array', items: { type: 'string' } },
    smoke_observed: { type: 'string' },
    notes: { type: 'string' },
  },
  required: ['verdict', 'findings', 'coverage_gaps', 'smoke_observed', 'notes'],
}

const MERGE = {
  type: 'object',
  properties: {
    merged: { type: 'boolean' },
    tip: { type: 'string' },
    conflicts_resolved: { type: 'array', items: { type: 'string' } },
    tests: { type: 'string' },
    smoke: { type: 'string' },
    notes: { type: 'string' },
  },
  required: ['merged', 'tip', 'conflicts_resolved', 'tests', 'smoke', 'notes'],
}

const GATE = {
  type: 'object',
  properties: {
    gate_pass: { type: 'boolean' },
    criteria: { type: 'array', items: { type: 'object', properties: { criterion: { type: 'string' }, pass: { type: 'boolean' }, evidence: { type: 'string' } }, required: ['criterion', 'pass', 'evidence'] } },
    doc: { type: 'string' },
    bugs_found: { type: 'array', items: { type: 'string' } },
    owner_questions: { type: 'array', items: { type: 'string' } },
    commits: { type: 'array', items: { type: 'string' } },
    notes: { type: 'string' },
  },
  required: ['gate_pass', 'criteria', 'doc', 'bugs_found', 'owner_questions', 'commits', 'notes'],
}

function wt(wp) { return ROOT + '/.worktrees/wp-' + wp.id }

function transcribePrompt(wp) {
  return COMMON + '\nYour worktree: ' + wt(wp) + ' (branch wp-' + wp.id + ', from ' + BRANCH + ' at ' + BASE + '). cd there for every command; cargo runs from that directory. The .reference symlink and a warm target/ are already in place.\n\n' + wp.brief + '\n\nDeliver the complete task. When done, return the structured report.'
}

function reviewPrompt(wp, report) {
  return [
'You are the *reviewer* for WP-' + wp.id + ' (' + wp.name + ') on the Rust/Bevy rewrite of Minecraft Beta 1.7.3 (PLAN_v4 §6 reviewer role: you see the vanilla Java and the diff, not the transcriber\'s reasoning). You may read any file and run cargo and the binary, but must NOT edit tracked files or run git write commands.',
'',
'Worktree: ' + wt(wp) + ' (branch wp-' + wp.id + '). The diff under review: cd ' + wt(wp) + ' && git diff ' + BASE + '...HEAD (and git log ' + BASE + '..HEAD). Spec tree: ' + JAVA + ' (+ net/minecraft/client/Minecraft.java). The transcriber\'s brief was:',
'---', wp.brief, '---',
'The transcriber\'s report (claims to verify, not to trust): ' + JSON.stringify(report),
'',
'Review method: for every Rust function in the diff that cites a Java file, open that Java at the cited lines and compare statement by statement: constants, texture indices, colours, brightness/fog/sky formulas, model box dimensions and rotation points, GUI coordinates and draw order, option names/defaults/serialisation, key defaults, timing (ticks per second, partial ticks, animation frame counts), evaluation order where it matters (RNG draw order incl. which Random instance; input → simulation ordering per §3.3), float vs double, casts, off-by-one, else branches omitted, vanilla bugs silently "fixed". Check D8: `git -C ' + wt(wp) + ' diff --stat ' + BASE + '...HEAD` and `git ls-files` show no PNG/OGG/lang file or Mojang text committed; no Mojang constants beyond game data (no copied LWJGL/Java rendering code where a Bevy construct exists). Check D9 if mc-sim was touched (no fs/time/thread/rand). Check D4 headers on any adapted MinecraftOSS code and that nothing was taken from its forbidden directories. Run the Definition of Done yourself: cargo build -p mc-app; cargo clippy --workspace --all-targets --all-features -- -Dwarnings; cargo fmt --all --check; cargo test --workspace; and the smoke run the report names (re-run its command, Read the screenshots, describe what you actually see and whether it is plausible for Beta 1.7.3). Report any failure.',
'',
'Be adversarial and concrete: a finding must name the Rust file:line, the Java file:line, what differs, and the exact fix. Severity: behavioural (reaches the simulation or saved data), visual (changes what is drawn/heard/read), citation, nit. Coverage gaps: Java members in the spec files that the brief asked for and the diff does not transcribe (list each). Return the structured review.',
  ].join('\n')
}

function fixPrompt(wp, review) {
  return COMMON + '\nYour worktree: ' + wt(wp) + ' (branch wp-' + wp.id + '). You are the *fix* agent for WP-' + wp.id + ' (' + wp.name + '): a transcriber already landed the work on this branch (read git log ' + BASE + '..HEAD and the diff) and a reviewer, comparing the diff against the Java, produced these findings:\n' + JSON.stringify(review, null, 1) + '\n\nOriginal brief (for scope):\n---\n' + wp.brief + '\n---\nFor each finding: open the Java at the cited lines yourself and verify it; apply the exact fix if the reviewer is right, skip it (and say why) if the Java proves the reviewer wrong. Close every behavioural and visual finding and every coverage gap in scope. Re-run the full Definition of Done including the smoke run. Commit as "WP-' + wp.id + ' step N: review fixes (...)". Return the structured report, with summary describing which findings were applied and which were rejected with the Java evidence.'
}

function mergePrompt(wp, prior) {
  return [
'You are the *integrator* for WP-' + wp.id + ' (' + wp.name + ') on the Rust/Bevy rewrite of Minecraft Beta 1.7.3. Main checkout: ' + ROOT + ' on branch ' + BRANCH + ' (verify with git -C ' + ROOT + ' branch --show-current; if it is not ' + BRANCH + ', stop and report). Worktree of the branch to merge: ' + wt(wp) + ' (branch wp-' + wp.id + '). Branches merged before yours in this wave: ' + JSON.stringify(prior) + '. Never use git stash.',
'',
'Steps: (1) cd ' + wt(wp) + ' && git rebase ' + BRANCH + '. Resolve every conflict preserving BOTH sides\' intent (crates/mc-client/src/lib.rs plugin lists and module tables: keep every module and plugin from both sides; Cargo.toml dependency lists: keep all; docs/mappings.md, README tables: keep all rows; Cargo.lock: take either then run cargo build to regenerate). Never resolve by dropping code. After each resolved commit: git add and GIT_EDITOR=true git rebase --continue. (2) In the worktree run cargo build -p mc-app; cargo test --workspace; cargo clippy --workspace --all-targets --all-features -- -Dwarnings; cargo fmt --all --check; fix anything the merge broke in a commit "WP-' + wp.id + ' merge: <what>" with the Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com> trailer. If the diff touches crates/mc-sim, crates/mc-golden, tests/golden or tools/oracle, also run sh tools/oracle/regen.sh once in the worktree (it must change no committed expected.jsonl; if it does, STOP and report the drift instead of committing it). (3) Smoke: run the binary once with the scripted-input mode described in crates/mc-client/README.md (fresh seed world, a few seconds, one screenshot), Read the screenshot, and describe it in one sentence; if the binary panics, fix the merge, not the feature, and report. (4) cd ' + ROOT + ' && git merge --ff-only wp-' + wp.id + ' (must fast-forward; else rebase again). (5) Report the new branch tip, the test summary, conflicts resolved, and the smoke result. Do not push. Do not touch .reference/. Your final text is a machine-read report.',
  ].join('\n')
}

const results = await pipeline(
  WPS,
  wp => MERGE_ONLY.includes(wp.id) ? Promise.resolve({ skipped: 'merge-only' }) : agent(transcribePrompt(wp), { label: 'transcribe:WP-' + wp.id, phase: 'Transcribe', model: 'opus', schema: REPORT }),
  (report, wp) => {
    if (!report) return null
    if (report.skipped) return { report: null, review: null, fix: null, mergeOnly: true }
    return agent(reviewPrompt(wp, report), { label: 'review:WP-' + wp.id, phase: 'Review', model: 'opus', schema: REVIEW }).then(review => ({ report, review }))
  },
  (r, wp) => {
    if (!r) return null
    if (r.mergeOnly) return r
    if (!r.review || (r.review.verdict === 'clean' && r.review.coverage_gaps.length === 0)) return { report: r.report, review: r.review, fix: null }
    return agent(fixPrompt(wp, r.review), { label: 'fix:WP-' + wp.id, phase: 'Fix', model: 'opus', schema: REPORT }).then(fix => ({ report: r.report, review: r.review, fix: fix }))
  },
)

const merges = []
const prior = []
for (const id of ORDER) {
  const i = WPS.findIndex(w => w.id === id)
  if (i < 0 || !results[i]) { merges.push({ wp: id, skipped: 'no transcription result' }); continue }
  log('merging WP-' + id + ' into ' + BRANCH)
  const m = await agent(mergePrompt(WPS[i], prior.slice()), { label: 'merge:WP-' + id, phase: 'Merge', model: 'opus', schema: MERGE })
  merges.push({ wp: id, merge: m })
  if (m && m.merged) prior.push(id)
}

let gate = null
if (args.gate) {
  phase('Gate')
  gate = await agent(args.gate.prompt.replace('{MERGES}', JSON.stringify(merges)), { label: args.gate.label || 'gate', phase: 'Gate', model: 'opus', schema: GATE })
}

return { tasks: WPS.map((wp, i) => ({ wp: wp.id, name: wp.name, result: results[i] })), merges, gate }