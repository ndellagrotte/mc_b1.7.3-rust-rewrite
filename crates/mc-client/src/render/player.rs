//! `RenderPlayer` (RenderPlayer.java): the biped with `/mob/char.png`
//! (skins are downloads, D2), scaled 0.9375, the four armour passes
//! (`setArmorModel`: `/armor/<material>_<1|2>.png` on `ModelBiped(1.0F)`,
//! or `ModelBiped(0.5F)` for the leggings, with only that piece's parts
//! shown), the sleeping pose, the helmet block and the held item
//! (`renderSpecials`, through [`super::held_item::HeldItemHook`]).
//!
//! Skipped: `deadmau5`'s ears and the cloak (`loadDownloadableImageTexture`
//! of an online skin or cloak, D2: never bound, so never drawn, as when the
//! download fails); `renderName` (another player's name tag; in
//! singleplayer the only player is `livingPlayer`, which it never labels).

use mc_sim::entity::{Entity, EntityKindData as K, dispatch, player as eplayer};
use mc_sim::item::{self, ItemKind, ItemStack};

use super::Ctx;
use super::gl::EntityGl;
use super::living::{LivingClass, RenderLiving};
use crate::model::RenderMode;
use crate::model::biped;
use crate::terrain::{blocks, render_blocks};

/// `armorFilenamePrefix`, by `ItemArmor.renderIndex`.
// src: RenderPlayer.java:10
pub const ARMOR_FILENAME_PREFIX: [&str; 5] = ["cloth", "chain", "iron", "diamond", "gold"];

/// Index of `modelArmorChestplate` in [`RenderLiving::models`].
const CHESTPLATE: usize = 0;
/// Index of `modelArmor`.
const ARMOR: usize = 1;

/// The armour texture of pass `pass` (0 helmet … 3 boots) for render index
/// `ri`: `"/armor/" + prefix + "_" + (pass == 2 ? 2 : 1) + ".png"`.
// src: RenderPlayer.java:25
pub fn armor_texture(ri: i32, pass: i32) -> Option<&'static str> {
    const T: [[&str; 2]; 5] = [
        ["/armor/cloth_1.png", "/armor/cloth_2.png"],
        ["/armor/chain_1.png", "/armor/chain_2.png"],
        ["/armor/iron_1.png", "/armor/iron_2.png"],
        ["/armor/diamond_1.png", "/armor/diamond_2.png"],
        ["/armor/gold_1.png", "/armor/gold_2.png"],
    ];
    let row = T.get(usize::try_from(ri).ok()?)?;
    Some(row[if pass == 2 { 1 } else { 0 }])
}

/// `renderPlayer(player, x, y, z, yaw, partial)`: the held-item and sneak
/// flags on the three models, `y - yOffset` (another player sneaking 0.125
/// lower), `doRenderLiving`, the flags cleared.
// src: RenderPlayer.java:42-54
#[allow(clippy::too_many_arguments)]
pub fn render_player(
    r: &mut RenderLiving,
    cx: &mut Ctx,
    var1: &Entity,
    var2: f64,
    var4: f64,
    var6: f64,
    var8: f32,
    var9: f32,
) {
    let Some(p) = var1.kind.player() else {
        return;
    };
    let var10 = p.inventory.get_current_item().is_some();
    let sneaking = dispatch::is_sneaking(var1, cx.world);
    r.main_model.held_item_right = var10;
    r.main_model.is_sneak = sneaking;
    for m in &mut r.models {
        m.held_item_right = var10;
        m.is_sneak = sneaking;
    }
    let mut var11 = var4 - var1.base.y_offset as f64;
    // !(var1 instanceof EntityPlayerSP): the local player never is
    if sneaking && !matches!(var1.kind, K::PlayerSp(..)) {
        var11 -= 0.125;
    }
    r.do_render_living(cx, var1, var2, var11, var6, var8, var9);
    r.main_model.is_sneak = false;
    r.main_model.held_item_right = false;
    for m in &mut r.models {
        m.is_sneak = false;
        m.held_item_right = false;
    }
}

/// `setArmorModel(player, pass, partial)`: the armour in slot `3 - pass`
/// (helmet first) binds its texture and becomes the pass model with only
/// its pieces shown.
// src: RenderPlayer.java:19-40
pub fn set_armor_model(r: &mut RenderLiving, cx: &mut Ctx, var1: &Entity, var2: i32) -> bool {
    let Some(p) = var1.kind.player() else {
        return false;
    };
    let Some(var4) = p.inventory.armor_item_in_slot(3 - var2) else {
        return false;
    };
    let Some(var5) = item::item(var4.item_id) else {
        return false;
    };
    let ItemKind::Armor { render_index, .. } = var5.kind else {
        return false;
    };
    if let Some(t) = armor_texture(render_index, var2) {
        cx.gl.bind_texture(t);
    }
    let idx = if var2 == 2 { ARMOR } else { CHESTPLATE };
    let var7 = &mut r.models[idx];
    var7.parts[biped::HEAD].show_model = var2 == 0;
    var7.parts[biped::HEADWEAR].show_model = var2 == 0;
    var7.parts[biped::BODY].show_model = var2 == 1 || var2 == 2;
    var7.parts[biped::RIGHT_ARM].show_model = var2 == 1;
    var7.parts[biped::LEFT_ARM].show_model = var2 == 1;
    var7.parts[biped::RIGHT_LEG].show_model = var2 == 2 || var2 == 3;
    var7.parts[biped::LEFT_LEG].show_model = var2 == 2 || var2 == 3;
    r.set_render_pass_model(idx);
    true
}

/// `renderName(player, x, y, z)`: only for players other than
/// `livingPlayer` (module docs: none in singleplayer; the multiplayer name
/// tag is deferred, D3).
// src: RenderPlayer.java:56-104
pub fn render_name(cx: &mut Ctx, var1: &Entity, _var2: f64, _var4: f64, _var6: f64) {
    let is_living_player = cx
        .info
        .living_player
        .is_some_and(|(id, _)| id == var1.base.entity_id);
    if cx.info.gui_enabled && !is_living_player {
        // deferred (D3): another player's name tag
    }
}

/// `renderSpecials(player, partial)`: the helmet slot's block on the head,
/// the held item in the right hand (a stick while fishing).
// src: RenderPlayer.java:106-219
pub fn render_specials(r: &mut RenderLiving, cx: &mut Ctx, var1: &Entity, _var2: f32) {
    let Some(p) = var1.kind.player() else {
        return;
    };
    if let Some(var3) = p.inventory.armor_item_in_slot(3).copied()
        && item::item(var3.item_id).is_some_and(|d| d.shifted_index < 256)
    {
        cx.gl.push_matrix();
        if let Some(t) = r.main_model.post_render(biped::HEAD, 0.0625) {
            cx.gl.mult_transform(&t);
        }
        if render_blocks::render_item_in_3d(blocks::get_render_type(var3.item_id)) {
            let var4 = 0.625f32;
            cx.gl.translatef(0.0, -0.25, 0.0);
            cx.gl.rotatef(180.0, 0.0, 1.0, 0.0);
            cx.gl.scalef(var4, -var4, var4);
        }
        cx.render_item(var1, &var3);
        cx.gl.pop_matrix();
    }
    // deadmau5's ears and the cloak: online textures, deferred (D2; module docs).
    // src: RenderPlayer.java:123-177
    if let Some(mut var21) = p.inventory.get_current_item().copied() {
        cx.gl.push_matrix();
        if let Some(t) = r.main_model.post_render(biped::RIGHT_ARM, 0.0625) {
            cx.gl.mult_transform(&t);
        }
        cx.gl.translatef(-0.0625, 0.4375, 0.0625);
        if p.fish_entity.is_some() {
            // new ItemStack(Item.stick)
            var21 = ItemStack::new(STICK, 1, 0);
        }
        held_item_transform(cx, &var21, true);
        cx.render_item(var1, &var21);
        cx.gl.pop_matrix();
    }
}

/// `Item.stick`'s id.
// src: Item.java:298 (new Item(24))
const STICK: i32 = 256 + 24;

/// The held item's placement in the hand: a 3D block (scaled 0.375,
/// tilted), a full-3D tool (0.625, held upright; the fishing rod turned
/// round, only `RenderPlayer` checks it), or a flat item (0.375).
// src: RenderPlayer.java:188-213, RenderBiped.java:20-40
pub fn held_item_transform(cx: &mut Ctx, var21: &ItemStack, rotate_around: bool) {
    let g = &mut *cx.gl;
    if var21.item_id < 256
        && render_blocks::render_item_in_3d(blocks::get_render_type(var21.item_id))
    {
        let mut var5 = 0.5f32;
        g.translatef(0.0, 0.1875, -0.3125);
        var5 *= 0.75;
        g.rotatef(20.0, 1.0, 0.0, 0.0);
        g.rotatef(45.0, 0.0, 1.0, 0.0);
        g.scalef(var5, -var5, var5);
    } else if item::item(var21.item_id).is_some_and(|d| d.is_full_3d()) {
        let var5 = 0.625f32;
        if rotate_around
            && item::item(var21.item_id).is_some_and(|d| d.should_rotate_around_when_rendering())
        {
            g.rotatef(180.0, 0.0, 0.0, 1.0);
            g.translatef(0.0, -0.125, 0.0);
        }
        g.translatef(0.0, 0.1875, 0.0);
        g.scalef(var5, -var5, var5);
        g.rotatef(-100.0, 1.0, 0.0, 0.0);
        g.rotatef(45.0, 0.0, 1.0, 0.0);
    } else {
        let var5 = 0.375f32;
        g.translatef(0.25, 0.1875, -0.1875);
        g.scalef(var5, var5, var5);
        g.rotatef(60.0, 0.0, 0.0, 1.0);
        g.rotatef(-90.0, 1.0, 0.0, 0.0);
        g.rotatef(20.0, 0.0, 0.0, 1.0);
    }
}

/// `func_186_b(player, partial)`: `glScalef(0.9375F)`.
// src: RenderPlayer.java:221-224
pub fn func_186_b(cx: &mut Ctx) {
    let var3 = 0.9375f32;
    cx.gl.scalef(var3, var3, var3);
}

impl RenderLiving {
    /// `RenderPlayer.drawFirstPersonHand()`: the shared `modelBipedMain`
    /// with `onGround = 0` posed at rest, then only `bipedRightArm` drawn
    /// on `gl` under its current matrix and state. The model keeps the rest
    /// of its state (`isSneak`, `heldItemRight`, `isRiding`) from the last
    /// third-person draw, as vanilla's single `RenderPlayer` instance does.
    /// For `ItemRenderer.renderItemInFirstPerson` (WP-3.7b), which reaches
    /// it through `RenderManager.renderers.living_mut(RendererId::Player)`;
    /// on any other renderer it does nothing (only `RenderPlayer` has it).
    // src: RenderPlayer.java:226-230
    pub fn draw_first_person_hand(&mut self, gl: &mut EntityGl) {
        if self.class != LivingClass::Player {
            return;
        }
        self.main_model.on_ground = 0.0;
        self.main_model
            .set_rotation_angles(0.0, 0.0, 0.0, 0.0, 0.0, 0.0625);
        let draws = self
            .main_model
            .draws(&[(biped::RIGHT_ARM, RenderMode::Render)], 0.0625);
        gl.model(self.main_model.kind, draws);
    }
}

/// `func_22016_b(player, x, y, z)`: a sleeping, living player is drawn at
/// its `renderOffset`.
// src: RenderPlayer.java:232-239
pub fn func_22016_b(cx: &mut Ctx, var1: &Entity, var2: f64, var4: f64, var6: f64) {
    if let Some(p) = var1.kind.player()
        && dispatch::is_entity_alive(var1)
        && p.is_player_sleeping()
    {
        cx.gl.translatef(
            (var2 + p.render_offset_x as f64) as f32,
            (var4 + p.render_offset_y as f64) as f32,
            (var6 + p.render_offset_z as f64) as f32,
        );
    } else {
        cx.gl.translatef(var2 as f32, var4 as f32, var6 as f32);
    }
}

/// `func_22017_a(player, ticks, bodyYaw, partial)`: a sleeping player lies
/// along the bed (`getBedOrientationInDegrees`, then 90 and 270).
// src: RenderPlayer.java:241-250
pub fn func_22017_a(
    r: &RenderLiving,
    cx: &mut Ctx,
    var1: &Entity,
    _var2: f32,
    var3: f32,
    var4: f32,
) {
    if var1.kind.player().is_some_and(|p| p.is_player_sleeping()) && dispatch::is_entity_alive(var1)
    {
        let o = eplayer::get_bed_orientation_in_degrees(var1, cx.world);
        cx.gl.rotatef(o, 0.0, 1.0, 0.0);
        cx.gl.rotatef(r.get_death_max_rotation(), 0.0, 0.0, 1.0);
        cx.gl.rotatef(270.0, 0.0, 1.0, 0.0);
    } else {
        r.rotate_corpse_base(cx, var1, var3, var4);
    }
}
