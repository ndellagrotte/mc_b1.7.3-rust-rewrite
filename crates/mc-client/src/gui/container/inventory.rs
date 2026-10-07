//! `GuiInventory` (GuiInventory.java): the player's inventory with the 2×2
//! crafting grid, `/gui/inventory.png`, and the player drawn in the box at
//! the left, turned toward the pointer.
//!
//! The player preview is `RenderManager.renderEntityWithPosYaw(thePlayer,
//! 0, 0, 0, 0, 1)` under the screen's matrix. The renderers are WP-3.5b's;
//! here the player is drawn through the GUI's batcher with WP-3.5a's
//! model tables ([`render_player_preview`]): `RenderPlayer.renderPlayer` →
//! `RenderLiving.doRenderLiving`'s matrix, the `ModelBiped` and the armour
//! passes (`RenderPlayer.setArmorModel`) under the standard item lighting.
//! Not drawn (reported): the held item and the pumpkin helmet
//! (`renderEquippedItems` → `ItemRenderer.renderItem`, WP-3.7b), the hurt
//! and death red overlay (`glDepthFunc(GL_EQUAL)` passes), the cloak (a
//! download, D2), and `doRenderShadowAndFire`'s shadow and fire.

use mc_core::math::jmath;
use mc_sim::entity::{Entity, dispatch, player as player_fns};
use mc_sim::inventory::container::Container;
use mc_sim::inventory::container::player::new_container_player;
use mc_sim::item::ItemKind;
use mc_sim::stats::{achievement_list::OPEN_INVENTORY, stat};
use mc_sim::world::World;

use super::LABEL_COLOR;
use crate::game::Game;
use crate::gui::batch::{GuiGl, gl};
use crate::gui::draw;
use crate::gui::font::FontRenderer;
use crate::gui::ingame::with_player;
use crate::gui::render_item::ItemGl;
use crate::gui::screen::ScreenCtx;
use crate::model::{Model, ModelKind, RenderMode, biped};

/// `/gui/inventory.png`.
// src: GuiInventory.java:31
pub const INVENTORY_PNG: &str = "/gui/inventory.png";
/// The player's skin (`EntityPlayer.texture`; downloads deferred, D2).
// src: EntityPlayer.java:51
pub const CHAR_PNG: &str = "/mob/char.png";
/// `RenderPlayer.armorFilenamePrefix`.
// src: RenderPlayer.java:10
pub const ARMOR_FILENAME_PREFIX: [&str; 5] = ["cloth", "chain", "iron", "diamond", "gold"];

/// `GuiInventory`'s fields.
#[derive(Clone, Debug, Default)]
pub struct InventoryState {
    /// `xSize_lo`: the pointer's x at the last `drawScreen`.
    // src: GuiInventory.java:6
    pub x_size_lo: f32,
    /// `ySize_lo`.
    // src: GuiInventory.java:7
    pub y_size_lo: f32,
}

/// `new GuiInventory(thePlayer)`'s container and side effect:
/// `player.inventorySlots` (a new `ContainerPlayer`, module docs of
/// [`super`]) and `player.addStat(AchievementList.openInventory, 1)`.
// src: GuiInventory.java:9-13, EntityPlayer.java:42
pub fn new_inventory_container(game: &mut Game) -> Option<Container> {
    let multiplayer = game.is_multiplayer_world();
    with_player(game, |_w, p| {
        player_fns::add_stat(p, stat(OPEN_INVENTORY), 1);
    });
    Some(new_container_player(!multiplayer))
}

/// `drawGuiContainerForegroundLayer()`: "Crafting" over the grid.
// src: GuiInventory.java:19-21
pub fn draw_foreground(g: &mut GuiGl, f: &FontRenderer) {
    f.draw_string(g, "Crafting", 86, 16, LABEL_COLOR);
}

/// The preview's turn toward the pointer: `(renderYawOffset, rotationYaw,
/// rotationPitch, the matrix's x tilt)` from the pointer's offsets
/// `var9` (from the box's centre x) and `var10` (from 50 above its
/// bottom).
// src: GuiInventory.java:225-233
pub fn preview_angles(var9: f32, var10: f32) -> (f32, f32, f32, f32) {
    let a9 = jmath::atan((var9 / 40.0) as f64) as f32;
    let a10 = jmath::atan((var10 / 40.0) as f64) as f32;
    (a9 * 20.0, a9 * 40.0, -a10 * 20.0, -a10 * 20.0)
}

impl InventoryState {
    /// `drawGuiContainerBackgroundLayer(partial)`: the sheet, then the
    /// player (module docs).
    // src: GuiInventory.java:28-46
    #[allow(clippy::too_many_arguments)]
    pub fn draw_background(
        &self,
        ctx: &mut ScreenCtx,
        g: &mut GuiGl,
        w: i32,
        h: i32,
        x_size: i32,
        y_size: i32,
        _partial: f32,
    ) {
        g.color4f(1.0, 1.0, 1.0, 1.0);
        g.bind_texture(INVENTORY_PNG);
        let var3 = (w - x_size) / 2;
        let var4 = (h - y_size) / 2;
        draw::draw_textured_modal_rect(g, 0.0, var3, var4, 0, 0, x_size, y_size);
        let mut ig = ItemGl::new();
        ig.rescale_normal = true;
        ig.push_matrix(g);
        ig.translatef(g, (var3 + 51) as f32, (var4 + 75) as f32, 50.0);
        let var5 = 30.0f32;
        ig.scalef(g, -var5, var5, var5);
        ig.rotatef(g, 180.0, 0.0, 0.0, 1.0);
        let var9 = (var3 + 51) as f32 - self.x_size_lo;
        let var10 = (var4 + 75 - 50) as f32 - self.y_size_lo;
        ig.rotatef(g, 135.0, 0.0, 1.0, 0.0);
        ig.enable_standard_item_lighting(g);
        ig.rotatef(g, -135.0, 0.0, 1.0, 0.0);
        let (yaw_offset, yaw, pitch, tilt) = preview_angles(var9, var10);
        ig.rotatef(g, tilt, 1.0, 0.0, 0.0);
        with_player(ctx.game, |world, p| {
            // The player's angles and entityBrightness are set for the draw
            // and put back (GuiInventory.java:222-241); the copy here is the
            // drawn one.
            let mut e = p.clone();
            if let Some(l) = e.kind.living_mut() {
                l.render_yaw_offset = yaw_offset;
            }
            e.base.rotation_yaw = yaw;
            e.base.rotation_pitch = pitch;
            e.base.entity_brightness = 1.0;
            ig.translatef(g, 0.0, e.base.y_offset, 0.0);
            // RenderManager.instance.playerViewY = 180 (only the name tag
            // reads it, not drawn for the player itself).
            render_player_preview(&mut ig, g, world, &e, 1.0);
        });
        ig.pop_matrix(g);
        ig.disable_standard_item_lighting(g);
    }
}

/// `RenderPlayer.renderPlayer(player, 0, 0, 0, yaw, partial)` →
/// `RenderLiving.doRenderLiving` under the current matrix, through the
/// batcher (module docs: what is drawn and what is not).
// src: RenderPlayer.java:42-54, :221-249, RenderLiving.java:19-117
pub fn render_player_preview(
    ig: &mut ItemGl,
    g: &mut GuiGl,
    world: &mut World,
    p: &Entity,
    var9: f32,
) {
    let (Some(l), Some(pd)) = (p.kind.living(), p.kind.player()) else {
        return;
    };
    let mut main = Model::new(ModelKind::Biped);
    let held = pd.inventory.get_current_item().is_some();
    let sneak = dispatch::is_sneaking(p, world);
    main.held_item_right = held;
    main.is_sneak = sneak;
    // var11 = y - yOffset (the player is an EntityPlayerSP: no sneak drop).
    let var11 = -p.base.y_offset;
    ig.push_matrix(g);
    let cull = ig.cull_face;
    ig.cull_face = false;
    // getSwingProgress(partial)
    let mut d = l.swing_progress - l.prev_swing_progress;
    if d < 0.0 {
        d += 1.0;
    }
    main.on_ground = l.prev_swing_progress + d * var9;
    main.is_riding = p.base.riding_entity.is_some();
    let var10 = l.prev_render_yaw_offset + (l.render_yaw_offset - l.prev_render_yaw_offset) * var9;
    let var11y = p.base.prev_rotation_yaw + (p.base.rotation_yaw - p.base.prev_rotation_yaw) * var9;
    let var12 =
        p.base.prev_rotation_pitch + (p.base.rotation_pitch - p.base.prev_rotation_pitch) * var9;
    // func_22012_b → RenderPlayer.func_22016_b: a sleeping player is moved
    // by its render offset.
    let sleeping = dispatch::is_entity_alive(p) && pd.sleeping;
    if sleeping {
        ig.translatef(
            g,
            pd.render_offset_x,
            var11 + pd.render_offset_y,
            pd.render_offset_z,
        );
    } else {
        ig.translatef(g, 0.0, var11, 0.0);
    }
    let var13 = p.base.ticks_existed as f32 + var9;
    // rotateCorpse → RenderPlayer.func_22017_a
    // (getDeathMaxRotation: 90, RenderLiving.java:153-155).
    if sleeping {
        let o = player_fns::get_bed_orientation_in_degrees(p, world);
        ig.rotatef(g, o, 0.0, 1.0, 0.0);
        ig.rotatef(g, 90.0, 0.0, 0.0, 1.0);
        ig.rotatef(g, 270.0, 0.0, 1.0, 0.0);
    } else {
        ig.rotatef(g, 180.0 - var10, 0.0, 1.0, 0.0);
        if l.death_time > 0 {
            let mut var5 = (l.death_time as f32 + var9 - 1.0) / 20.0 * 1.6;
            var5 = mc_core::math::sqrt_float(var5);
            if var5 > 1.0 {
                var5 = 1.0;
            }
            ig.rotatef(g, var5 * 90.0, 0.0, 0.0, 1.0);
        }
    }
    let var14 = 0.0625f32;
    ig.scalef(g, -1.0, -1.0, 1.0);
    // preRenderCallback → RenderPlayer.func_186_b: scale 0.9375.
    // src: RenderPlayer.java:221-224
    let var3 = 0.9375f32;
    ig.scalef(g, var3, var3, var3);
    ig.translatef(g, 0.0, -24.0 * var14 - 0.0078125, 0.0);
    let mut var15 = l.prev_leg_yaw + (l.leg_yaw - l.prev_leg_yaw) * var9;
    let var16 = l.leg_swing - l.leg_yaw * (1.0 - var9);
    if var15 > 1.0 {
        var15 = 1.0;
    }
    g.bind_texture(CHAR_PNG);
    g.enable(gl::ALPHA_TEST);
    let draws = main.render(var16, var15, var13, var11y - var10, var12, var14);
    draw_model_parts(ig, g, &main, &draws, var14);
    for var17 in 0..4 {
        // shouldRenderPass → setArmorModel(player, pass, partial)
        let Some(stack) = pd.inventory.armor_item_in_slot(3 - var17) else {
            continue;
        };
        let Some(ItemKind::Armor { render_index, .. }) = stack.get_item().map(|i| i.kind) else {
            continue;
        };
        let tex = format!(
            "/armor/{}_{}.png",
            ARMOR_FILENAME_PREFIX[render_index as usize % 5],
            if var17 == 2 { 2 } else { 1 }
        );
        let kind = if var17 == 2 {
            ModelKind::BipedArmor
        } else {
            ModelKind::BipedArmorChestplate
        };
        let mut var7 = Model::new(kind);
        var7.parts[biped::HEAD].show_model = var17 == 0;
        var7.parts[biped::HEADWEAR].show_model = var17 == 0;
        var7.parts[biped::BODY].show_model = var17 == 1 || var17 == 2;
        var7.parts[biped::RIGHT_ARM].show_model = var17 == 1;
        var7.parts[biped::LEFT_ARM].show_model = var17 == 1;
        var7.parts[biped::RIGHT_LEG].show_model = var17 == 2 || var17 == 3;
        var7.parts[biped::LEFT_LEG].show_model = var17 == 2 || var17 == 3;
        var7.on_ground = main.on_ground;
        var7.is_riding = main.is_riding;
        var7.held_item_right = held;
        var7.is_sneak = sneak;
        g.bind_texture(crate::gui::batch::TexRef::Name(tex.into()));
        let draws = var7.render(var16, var15, var13, var11y - var10, var12, var14);
        draw_model_parts(ig, g, &var7, &draws, var14);
        g.disable(gl::BLEND);
        g.enable(gl::ALPHA_TEST);
    }
    ig.cull_face = cull;
    ig.pop_matrix(g);
}

/// `ModelRenderer.render(scale)` for each drawn part: translate to the
/// rotation point, rotate (Z, Y, X, or Y, X, Z for `renderWithRotation`)
/// in degrees (`angle * 57.295776F`), then the display list.
// src: ModelRenderer.java:85-148
fn draw_model_parts(
    ig: &mut ItemGl,
    g: &mut GuiGl,
    m: &Model,
    draws: &[crate::model::PartDraw],
    scale: f32,
) {
    const DEG: f32 = 57.295776;
    for d in draws {
        let part = &m.parts[d.part];
        let v = part.display_list(scale);
        ig.push_matrix(g);
        ig.translatef(
            g,
            part.rotation_point_x * scale,
            part.rotation_point_y * scale,
            part.rotation_point_z * scale,
        );
        let (x, y, z) = (
            part.rotate_angle_x * DEG,
            part.rotate_angle_y * DEG,
            part.rotate_angle_z * DEG,
        );
        match d.mode {
            RenderMode::Render => {
                if z != 0.0 {
                    ig.rotatef(g, z, 0.0, 0.0, 1.0);
                }
                if y != 0.0 {
                    ig.rotatef(g, y, 0.0, 1.0, 0.0);
                }
                if x != 0.0 {
                    ig.rotatef(g, x, 1.0, 0.0, 0.0);
                }
            }
            RenderMode::WithRotation => {
                if y != 0.0 {
                    ig.rotatef(g, y, 0.0, 1.0, 0.0);
                }
                if x != 0.0 {
                    ig.rotatef(g, x, 1.0, 0.0, 0.0);
                }
                if z != 0.0 {
                    ig.rotatef(g, z, 0.0, 0.0, 1.0);
                }
            }
        }
        ig.draw_quads(g, &v.positions, &v.uvs, None, &v.normals);
        ig.pop_matrix(g);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_turns_toward_the_pointer() {
        // Pointer at the box's centre: no turn.
        assert_eq!(preview_angles(0.0, 0.0), (0.0, 0.0, -0.0, -0.0));
        // 40 pixels to the left of the centre (var9 = +40): atan(1) = 45°
        // in radians × 20 for the body, × 40 for the head.
        let (body, head, _, _) = preview_angles(40.0, 0.0);
        assert!((body - std::f32::consts::FRAC_PI_4 * 20.0).abs() < 1e-5);
        assert!((head - std::f32::consts::FRAC_PI_4 * 40.0).abs() < 1e-5);
        let (_, _, pitch, tilt) = preview_angles(0.0, 40.0);
        assert_eq!(pitch, tilt);
        assert!(pitch < 0.0);
    }
}
