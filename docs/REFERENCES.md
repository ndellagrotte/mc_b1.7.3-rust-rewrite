# Reference material (`.reference/`)

`.reference/` holds the Mojang-derived and third-party inputs this project is
transcribed from. It is **git-ignored** (`/.reference/` in `.gitignore`, PLAN_v4
D8) and is **frozen**: nothing in it is edited, moved, or deleted, and nothing
from it is ever committed. Agents read it; they do not write it. It is restored
by hand from the sources below, never by the build.

| Directory | What it is | Origin | Revision | Licence (as stated in-tree) |
|---|---|---|---|---|
| `mc_b1.7.3_release/` | Vanilla Minecraft Beta 1.7.3 client and server source, MCP-deobfuscated. **The spec for the whole project** (PLAN_v4 D2); transcribers read only `1.7.3-LTS/src/minecraft`. | git clone of <https://github.com/jacobo-mc/mc_b1.7.3_release.git> | commit `740c583901e1ff1150e9ef37e37dab5bc0e4f807` ("Update LICENSE", 2021-12-05). The clone's `.git` is kept so the pin is checkable with `git -C .reference/mc_b1.7.3_release rev-parse HEAD`. | MIT, `LICENSE`, "Copyright (c) 2021 Modification Station and Mojang AB" |
| `mc_b1.7.3-web-port/` | A TeaVM browser port of Beta 1.7.3. Not a source for anything; kept only as a worked example of a browser build (Phase 5). | GitHub zip of <https://github.com/PeytonPlayz595/Beta-1.7.3> | zip, no VCS | MIT, `LICENSE`, "Copyright (c) 2024 PeytonPlayz595" |
| `2010-rust-rewrite-mashup-main/` | chasmlol's `2010-rust-rewrite-mashup`, containing the MinecraftOSS crates under `third_party/minecraftoss/`. Code may be adapted with attribution (PLAN_v4 D4); see `NOTICE` and `credits/`. | GitHub zip of <https://github.com/chasmlol/2010-rust-rewrite-mashup> | zip, no VCS. Its `third_party/minecraftoss/README.md` states the crates are copied from MinecraftOSS commit `4013a68`. | Apache-2.0, `LICENSE` (full licence text) plus `NOTICE` |
| `mc_b1.7.3-world/testWorld/` | A vanilla McRegion test world. | Generated locally with vanilla b1.7.3 | n/a | Mojang-generated save data; local only |

## Notes

- **Vanilla pin.** The vanilla release is frozen at commit `740c583…`. The
  owner has decided that this exact revision does not matter to anyone: the
  repository has not changed in substance since 2021, and any checkout of it
  is an acceptable spec. The commit is recorded once, here, and not re-verified.
- **Test world.** Its own `world_info.txt` reads:

  ```
  - world seed is '123'
  - chest at 9 65 -60 containing two wood planks and four sticks
  - crafting table at 9 65 -61
  ```

  PLAN_v4 (§1.2, §4.4 item 4) gives the chest as `9,64,-61`. The world's note
  is the primary source; the discrepancy is to be settled when the fixture
  that reads the chest is written (Phase 0/1), not by editing either file now.
- **Runtime assets** (textures, `lang/`, sounds) are never taken from here by
  the build. The game reads them from the user's own `minecraft.jar` and
  `resources/` directory at runtime (PLAN_v4 D8). The default `minecraft.jar`
  path points into this tree only as a convenience on the owner's machine.
- **Restoring.** Clone or unzip the sources above into `.reference/` with the
  directory names in the table. Nothing else is required.
