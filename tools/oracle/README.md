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
| `check-sensitivity.sh` | One run with every seed XOR 1; every seeded fixture must change, `sine-table` must not (§4.4 item 2) |
| `env.sh` | Shared classpath (`minecraft.jar` last) and `oracle_run` helper, sourced by the scripts |
| `java/harness/` | `Main` (driver), `TierA` (the four kinds), `Blake3`, `Json`, `Hex`, `Trace`, `HarnessMath` |

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
cause.

## CI

The `golden-regen` job in `.github/workflows/ci.yml` (manual, Temurin 21) calls
`regen.sh`. It only does real work on a runner that has the git-ignored
reference tree; elsewhere it reports and exits 0. Ordinary CI runs only the
Rust side against the committed fixtures.
