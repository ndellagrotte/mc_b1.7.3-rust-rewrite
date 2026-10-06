# Credits

## Assets

* Bevy icon: [MIT License](licenses/Bevy_MIT_License.md);

## Code

> Parts of this project are adapted from
> [MinecraftOSS](https://github.com/chasmlol/2010-rust-rewrite-mashup) by
> **chasmlol**, from the `2010-rust-rewrite-mashup` repository (commit
> `4013a68`), used under the Apache License 2.0. Adapted files are marked in
> their headers; see `NOTICE` and `credits/licenses/MinecraftOSS_Apache-2.0.md`.

`crates/mc-core/src/strict_math.rs` transcribes fdlibm 5.3 (Sun Microsystems,
1993/2004; "permission to use, copy, modify, and distribute this software is
freely granted, provided that this notice is preserved"). The notice is kept at
the top of that file. It exists so that `mc_core::jmath` reproduces Java's
`StrictMath` bit for bit (PLAN_v4 §3.4 rule 5).
