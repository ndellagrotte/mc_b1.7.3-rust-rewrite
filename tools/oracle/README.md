# Java oracle (PLAN_v4 §4, ADR-007)

Compiles the vanilla b1.7.3 client source from the git-ignored reference tree
(`.reference/mc_b1.7.3_release/1.7.3-LTS`) together with a small harness, and
writes the golden fixtures under `tests/golden/` (format: `tests/golden/README.md`).
Needs a JDK ≥ 17 (`javac`/`java` on `PATH`); the harness sources are Java 17.

| File | Purpose |
|---|---|
| `build.sh` | `rewrite-math-random.sh`, then `javac` of `build/oracle-src` + `java/` into `build/oracle` (§4.1) |
| `rewrite-math-random.sh` | Copies `src/minecraft` to `build/oracle-src`, rewrites `Math.random()` |
| `regen.sh` | `build.sh`, runs the harness into `build/golden-out`, copies the results into `tests/golden`, prints `git status --short tests/golden` |
| `check-reproducible.sh` | Three harness runs, byte-compared (§4.4 item 1) |
| `check-sensitivity.sh` | One run with every seed XOR 1; every seeded fixture must change, every unseeded one must not; only `sine_table`, `nbt_write`, `nbt_read`, `java_hash_order` may lack a seed (§4.4 item 2) |
| `env.sh` | Shared classpath (`minecraft.jar` last) and `oracle_run` helper, sourced by the scripts |
| `java/harness/` | `Main` (driver), `TierA` (kind dispatch; `java_random`, `sine_table`, `math_sweep`, `terrain`), `Nbt` (`nbt_write`, `nbt_read`), `HashOrder` (`java_hash_order`), `Blake3`, `Json`, `Hex`, `Trace`, `HarnessMath` |

Tier A kinds (specified in `tests/golden/README.md`): `java_random`, `sine_table`,
`math_sweep` (optional `"strict": true` for `StrictMath`), `terrain` (WP-0.2);
`nbt_write`, `nbt_read`, `java_hash_order` (WP-0.3).

## Use

```sh
tools/oracle/regen.sh               # build + regenerate tests/golden (about 3 s)
tools/oracle/check-reproducible.sh  # needs a built oracle
tools/oracle/check-sensitivity.sh   # compares against the committed fixtures
```

Scripts are POSIX `sh` and work from any directory. Direct use:

```sh
java -cp "build/oracle:$CP" harness.Main tests/golden build/golden-out [area/name ...]
java -cp "build/oracle:$CP" harness.Main --selftest       # BLAKE3 test vectors
java -Dharness.perturbSeed=1 -cp ... harness.Main ...      # XOR every scenario seed
```

Every run checks BLAKE3 against the official test vectors first. Never edit an
`expected.jsonl` by hand. Drift after a regen without a harness change is a
determinism bug: investigate it, do not commit it.

## The one rewrite of vanilla code

`rewrite-math-random.sh` replaces every `Math.random()` with
`harness.HarnessMath.random()` and nothing else (58 occurrences on 49 lines;
the script fails if the count is 0 or the rewrite is incomplete).
`HarnessMath` is one `java.util.Random` stream, reseeded from the scenario seed
(0 when there is none) at the start of every scenario. No other vanilla file is
edited; the copy is regenerated on every build.

## Determinism overrides

Applied by `TierA.makeDeterministic` right after `new World(new SaveHandlerMP(),
"harness", provider, seed)`, all derived from the scenario seed:

- `world.rand.setSeed(seed)` (World.java:82, `new Random()`).
- `field_9437_g = new Random(seed).nextInt()` (World.java:75, clock-seeded
  random-tick LCG state), set by reflection.
- `soundCounter = <that same Random>.nextInt(12000)` (World.java:90, drawn from
  the unseeded `rand`), set by reflection.

`lockTimestamp` (World.java:80, wall clock) is left alone: it only guards
session locks, which `SaveHandlerMP` does not have. Entities spawned by
population keep their unseeded `Entity.rand` (Entity.java:96); Tier A traces
contain no entity state. Block and Item statics initialise normally.

`java.lang.Math` results are those of the oracle JVM (HotSpot intrinsics, 1-ulp
contract); the fixtures in this repository were generated on OpenJDK 25,
x86-64. If a regen on another JDK changes only `math/math-sweep`, that is the
cause. Measured on that JVM (WP-0.3, PLAN_v4 §3.4 rule 5): `-Xint`, C1-only and
C2-only (`-Xcomp -XX:-TieredCompilation`) give the same `Math` bits as the default
(the interpreter and both compilers use the same stubs, so warm-up does not change
results); with `-XX:+UnlockDiagnosticVMOptions
-XX:DisableIntrinsic=_dsin,_dcos,_dtan,_datan2,_dpow,_dexp,_dlog,_dlog10`
`Math` equals `StrictMath` (`math/math-sweep-strict`) on every input. `regen.sh`
uses the default flags.

`java_hash_order` fixtures record the oracle JDK's `HashSet` order. `HashOrder`
cross-checks it against a model of the JDK 8+ table and refuses scenarios whose
order would depend on `System.identityHashCode` (equal-hash keys in a tree bin);
`regen.sh` prints the model's final capacity and tree-bin count per scenario.

`nbt_read` input files (e.g. `tests/golden/nbt/testworld-level-dat/level.dat`)
are re-emitted unchanged by the oracle, so `regen.sh`'s replace-all-generated-files
step keeps them.

## CI

The `golden-regen` job in `.github/workflows/ci.yml` (manual, Temurin 21) calls
`regen.sh`. It only does real work on a runner that has the git-ignored
reference tree; elsewhere it reports and exits 0. Ordinary CI runs only the
Rust side against the committed fixtures.
