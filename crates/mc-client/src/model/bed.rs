//! `ModelBed` (ModelBed.java): not a model in b1.7.3 but three static
//! tables about the bed block, used by `RenderBlocks.renderBlockBed`
//! (WP-3.3), `BlockBed` and `BlockRedstoneWire` (the last two already
//! transcribed in `mc-sim`).

/// `headInvisibleFace`: per bed direction (`meta & 3`), the face of the
/// head half that is not drawn (it touches the foot).
// src: ModelBed.java:4, RenderBlocks.java:208
pub const HEAD_INVISIBLE_FACE: [i32; 4] = [3, 4, 2, 5];

/// `footInvisibleFaceRemap`: the opposite direction, so
/// `HEAD_INVISIBLE_FACE[FOOT_INVISIBLE_FACE_REMAP[d]]` is the foot's hidden
/// face.
// src: ModelBed.java:5, RenderBlocks.java:210
pub const FOOT_INVISIBLE_FACE_REMAP: [i32; 4] = [2, 3, 0, 1];

/// `bedDirection`: per bed direction, the model side each world side
/// shows.
// src: ModelBed.java:6
pub const BED_DIRECTION: [[i32; 6]; 4] = [
    [1, 0, 3, 2, 5, 4],
    [1, 0, 5, 4, 2, 3],
    [1, 0, 2, 3, 4, 5],
    [1, 0, 4, 5, 3, 2],
];
