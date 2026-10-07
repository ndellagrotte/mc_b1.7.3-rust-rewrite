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
//! `renderEquippedItems` (`RenderPlayer.renderSpecials`: the pumpkin
//! helmet and the held item) is drawn with a GUI-batcher transcription of
//! `ItemRenderer.renderItem` ([`render_item`]). Not drawn (reported): the
//! hurt and death red overlay (`glDepthFunc(GL_EQUAL)` passes, which
//! [`GuiGl`] does not offer), the cloak (a download, D2), and
//! `doRenderShadowAndFire`'s shadow and fire (`Render`, WP-3.5b).

use mc_core::math::jmath;
use mc_sim::crafting::ids;
use mc_sim::entity::{Entity, base, dispatch, player as player_fns};
use mc_sim::inventory::container::Container;
use mc_sim::inventory::container::player::new_container_player;
use mc_sim::item::{ItemKind, ItemStack, item};
use mc_sim::stats::{achievement_list::OPEN_INVENTORY, stat};
use mc_sim::world::World;

use super::LABEL_COLOR;
use crate::game::Game;
use crate::gui::batch::{GuiGl, gl};
use crate::gui::draw;
use crate::gui::font::FontRenderer;
use crate::gui::ingame::with_player;
use crate::gui::render_item::{ITEMS_PNG, ItemGl, TERRAIN_PNG, empty_colorizers};
use crate::gui::screen::ScreenCtx;
use crate::model::part::ModelPart;
use crate::model::{Model, ModelKind, RenderMode, biped};
use crate::terrain::blocks;
use crate::terrain::render_blocks::{render_block_as_item, render_item_in_3d};
use crate::terrain::tessellator::Tessellator;

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
        // Vanilla indexes the array directly and would throw on a bad
        // renderIndex (none exists in b1.7.3: 0..4); the pass is skipped
        // instead of masking it with a modulo.
        let Some(prefix) = usize::try_from(render_index)
            .ok()
            .and_then(|i| ARMOR_FILENAME_PREFIX.get(i))
        else {
            continue;
        };
        let tex = format!("/armor/{}_{}.png", prefix, if var17 == 2 { 2 } else { 1 });
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
    // renderEquippedItems(player, partial) → RenderPlayer.renderSpecials.
    render_specials(ig, g, world, p, &main);
    ig.cull_face = cull;
    ig.pop_matrix(g);
}

/// `ModelRenderer.postRender(scale)`: the part's rotation point and angles
/// (Z, Y, X) applied to the current matrix; nothing for a hidden part.
// src: ModelRenderer.java:150-178
fn post_render(ig: &mut ItemGl, g: &mut GuiGl, part: &ModelPart, var1: f32) {
    const DEG: f32 = 57.295776;
    if part.is_hidden || !part.show_model {
        return;
    }
    if part.rotate_angle_x == 0.0 && part.rotate_angle_y == 0.0 && part.rotate_angle_z == 0.0 {
        if part.rotation_point_x != 0.0
            || part.rotation_point_y != 0.0
            || part.rotation_point_z != 0.0
        {
            ig.translatef(
                g,
                part.rotation_point_x * var1,
                part.rotation_point_y * var1,
                part.rotation_point_z * var1,
            );
        }
    } else {
        ig.translatef(
            g,
            part.rotation_point_x * var1,
            part.rotation_point_y * var1,
            part.rotation_point_z * var1,
        );
        if part.rotate_angle_z != 0.0 {
            ig.rotatef(g, part.rotate_angle_z * DEG, 0.0, 0.0, 1.0);
        }
        if part.rotate_angle_y != 0.0 {
            ig.rotatef(g, part.rotate_angle_y * DEG, 0.0, 1.0, 0.0);
        }
        if part.rotate_angle_x != 0.0 {
            ig.rotatef(g, part.rotate_angle_x * DEG, 1.0, 0.0, 0.0);
        }
    }
}

/// `RenderPlayer.renderSpecials(player, partial)` (`renderEquippedItems`):
/// the block helmet on the head (only the pumpkin fits the helmet slot,
/// `SlotArmor`) and the held item in the right hand, each through
/// `ItemRenderer.renderItem` ([`render_item`]). Not drawn: deadmau5's
/// ears (a downloaded skin) and the cloak (a download, D2).
// src: RenderPlayer.java:97-217
fn render_specials(ig: &mut ItemGl, g: &mut GuiGl, world: &mut World, p: &Entity, main: &Model) {
    let Some(pd) = p.kind.player() else {
        return;
    };
    // src: RenderPlayer.java:98-110
    if let Some(var3) = pd.inventory.armor_item_in_slot(3)
        && var3.get_item().is_some_and(|i| i.shifted_index < 256)
    {
        ig.push_matrix(g);
        post_render(ig, g, &main.parts[biped::HEAD], 0.0625);
        if render_item_in_3d(blocks::get_render_type(var3.item_id)) {
            let var4 = 0.625f32;
            ig.translatef(g, 0.0, -0.25, 0.0);
            ig.rotatef(g, 180.0, 0.0, 1.0, 0.0);
            ig.scalef(g, var4, -var4, var4);
        }
        render_item(ig, g, world, p, var3);
        ig.pop_matrix(g);
    }
    // src: RenderPlayer.java:176-213
    let Some(held) = pd.inventory.get_current_item() else {
        return;
    };
    let mut var21 = *held;
    ig.push_matrix(g);
    post_render(ig, g, &main.parts[biped::RIGHT_ARM], 0.0625);
    ig.translatef(g, -0.0625, 0.4375, 0.0625);
    if pd.fish_entity.is_some() {
        var21 = ItemStack::new(ids::STICK, 1, 0);
    }
    let def = item(var21.item_id);
    if var21.item_id < 256 && render_item_in_3d(blocks::get_render_type(var21.item_id)) {
        let mut var5 = 0.5f32;
        ig.translatef(g, 0.0, 0.1875, -0.3125);
        var5 *= 0.75;
        ig.rotatef(g, 20.0, 1.0, 0.0, 0.0);
        ig.rotatef(g, 45.0, 0.0, 1.0, 0.0);
        ig.scalef(g, var5, -var5, var5);
    } else if def.is_some_and(|d| d.is_full_3d()) {
        let var5 = 0.625f32;
        if def.is_some_and(|d| d.should_rotate_around_when_rendering()) {
            ig.rotatef(g, 180.0, 0.0, 0.0, 1.0);
            ig.translatef(g, 0.0, -0.125, 0.0);
        }
        ig.translatef(g, 0.0, 0.1875, 0.0);
        ig.scalef(g, var5, -var5, var5);
        ig.rotatef(g, -100.0, 1.0, 0.0, 0.0);
        ig.rotatef(g, 45.0, 0.0, 1.0, 0.0);
    } else {
        let var5 = 0.375f32;
        ig.translatef(g, 0.25, 0.1875, -0.1875);
        ig.scalef(g, var5, var5, var5);
        ig.rotatef(g, 60.0, 0.0, 0.0, 1.0);
        ig.rotatef(g, -90.0, 1.0, 0.0, 0.0);
        ig.rotatef(g, 20.0, 0.0, 0.0, 1.0);
    }
    render_item(ig, g, world, p, &var21);
    ig.pop_matrix(g);
}

/// `EntityPlayer.getItemIcon(stack)`: the stack's icon, one row down for
/// the fishing rod while its line is out.
// src: EntityPlayer.java:802-809, EntityLiving.java:872-874
pub fn get_item_icon(p: &Entity, var1: &ItemStack) -> i32 {
    let mut var2 = var1.get_icon_index();
    if var1.item_id == ids::FISHING_ROD && p.kind.player().is_some_and(|d| d.fish_entity.is_some())
    {
        var2 = var1.get_icon_index() + 16;
    }
    var2
}

/// The 16 one-pixel slices of `ItemRenderer.renderItem`'s flat item
/// (`var13 = 0.0625` thick, in `[0, 1]²`): the front and back faces, then
/// the −x, +x, +y and −y slice sets, each `(normal, quads)` with the
/// vertices as `(x, y, z, u, v)`, the icon's texture box being
/// `var5..var6` (u, flipped) and `var7..var8` (v).
// src: ItemRenderer.java:32-122
pub fn flat_item_faces(var4: i32) -> Vec<([f32; 3], Vec<[f64; 5]>)> {
    let var5 = ((var4 % 16 * 16) as f32 + 0.0) / 256.0;
    let var6 = ((var4 % 16 * 16) as f32 + 15.99) / 256.0;
    let var7 = ((var4 / 16 * 16) as f32 + 0.0) / 256.0;
    let var8 = ((var4 / 16 * 16) as f32 + 15.99) / 256.0;
    let var9 = 1.0f32;
    let var13 = 0.0625f32;
    let (u5, u6, v7, v8) = (var5 as f64, var6 as f64, var7 as f64, var8 as f64);
    let (w, d) = (var9 as f64, (0.0 - var13) as f64);
    let mut out = vec![
        (
            [0.0, 0.0, 1.0],
            vec![
                [0.0, 0.0, 0.0, u6, v8],
                [w, 0.0, 0.0, u5, v8],
                [w, 1.0, 0.0, u5, v7],
                [0.0, 1.0, 0.0, u6, v7],
            ],
        ),
        (
            [0.0, 0.0, -1.0],
            vec![
                [0.0, 1.0, d, u6, v7],
                [w, 1.0, d, u5, v7],
                [w, 0.0, d, u5, v8],
                [0.0, 0.0, d, u6, v8],
            ],
        ),
    ];
    let mut nx = Vec::new();
    let mut px = Vec::new();
    let mut py = Vec::new();
    let mut ny = Vec::new();
    for var14 in 0..16 {
        let var15 = var14 as f32 / 16.0;
        // -x
        let var16 = (var6 + (var5 - var6) * var15 - 0.001953125) as f64;
        let var17 = (var9 * var15) as f64;
        nx.extend([
            [var17, 0.0, d, var16, v8],
            [var17, 0.0, 0.0, var16, v8],
            [var17, 1.0, 0.0, var16, v7],
            [var17, 1.0, d, var16, v7],
        ]);
        // +x
        let var17 = (var9 * var15 + 0.0625) as f64;
        px.extend([
            [var17, 1.0, d, var16, v7],
            [var17, 1.0, 0.0, var16, v7],
            [var17, 0.0, 0.0, var16, v8],
            [var17, 0.0, d, var16, v8],
        ]);
        // +y
        let var16 = (var8 + (var7 - var8) * var15 - 0.001953125) as f64;
        py.extend([
            [0.0, var17, 0.0, u6, var16],
            [w, var17, 0.0, u5, var16],
            [w, var17, d, u5, var16],
            [0.0, var17, d, u6, var16],
        ]);
        // -y
        let var17 = (var9 * var15) as f64;
        ny.extend([
            [w, var17, 0.0, u5, var16],
            [0.0, var17, 0.0, u6, var16],
            [0.0, var17, d, u6, var16],
            [w, var17, d, u5, var16],
        ]);
    }
    out.push(([-1.0, 0.0, 0.0], nx));
    out.push(([1.0, 0.0, 0.0], px));
    out.push(([0.0, 1.0, 0.0], py));
    out.push(([0.0, -1.0, 0.0], ny));
    out
}

/// `ItemRenderer.renderItem(entity, stack)` through the batcher: a block
/// drawn in 3D (`RenderBlocks.renderItemIn3d`) as
/// `renderBlockOnInventory(block, damage, entity.getEntityBrightness(1))`
/// from `/terrain.png`, any other item as its icon extruded one pixel deep
/// ([`flat_item_faces`]) from `/terrain.png` or `/gui/items.png`, lit under
/// the current lights. The world's item renderer (first person, dropped
/// items, the held item of other entities) is WP-3.7b's.
// src: ItemRenderer.java:20-128
fn render_item(ig: &mut ItemGl, g: &mut GuiGl, world: &mut World, e: &Entity, var2: &ItemStack) {
    ig.push_matrix(g);
    if var2.item_id < 256 && render_item_in_3d(blocks::get_render_type(var2.item_id)) {
        g.bind_texture(TERRAIN_PNG);
        let b = base::get_entity_brightness(e, world, 1.0);
        let statics = world.block_statics.clone();
        if let Some(d) = render_block_as_item(
            &statics,
            empty_colorizers(),
            var2.item_id,
            var2.get_item_damage(),
            b,
        ) {
            if let Some(c) = d.gl_color {
                g.color4f(c[0], c[1], c[2], c[3]);
            }
            ig.draw_tessellator(g, &d.tess);
        }
    } else {
        if var2.item_id < 256 {
            g.bind_texture(TERRAIN_PNG);
        } else {
            g.bind_texture(ITEMS_PNG);
        }
        let var4 = get_item_icon(e, var2);
        let var10 = 0.0f32;
        let var11 = 0.3f32;
        ig.rescale_normal = true;
        ig.translatef(g, -var10, -var11, 0.0);
        let var12 = 1.5f32;
        ig.scalef(g, var12, var12, var12);
        ig.rotatef(g, 50.0, 0.0, 1.0, 0.0);
        ig.rotatef(g, 335.0, 0.0, 0.0, 1.0);
        ig.translatef(g, -0.9375, -0.0625, 0.0);
        let mut t = Tessellator::default();
        for (n, quads) in flat_item_faces(var4) {
            t.start_drawing_quads();
            t.set_normal(n[0], n[1], n[2]);
            for v in quads {
                t.add_vertex_with_uv(v[0], v[1], v[2], v[3], v[4]);
            }
            ig.draw_tessellator(g, &t);
        }
        // glDisable(GL_RESCALE_NORMAL) (it was on: doRenderLiving's).
        ig.rescale_normal = false;
    }
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

    #[test]
    fn flat_item_slices() {
        // Icon 17: column 1, row 1 of the sheet.
        let f = flat_item_faces(17);
        let normals: Vec<[f32; 3]> = f.iter().map(|(n, _)| *n).collect();
        assert_eq!(
            normals,
            vec![
                [0.0, 0.0, 1.0],
                [0.0, 0.0, -1.0],
                [-1.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, -1.0, 0.0],
            ]
        );
        assert_eq!(f[0].1.len(), 4);
        assert_eq!(f[1].1.len(), 4);
        for (_, q) in &f[2..] {
            assert_eq!(q.len(), 64);
        }
        let u5 = (16.0f32 / 256.0) as f64;
        let u6 = (31.99f32 / 256.0) as f64;
        let v7 = (16.0f32 / 256.0) as f64;
        let v8 = (31.99f32 / 256.0) as f64;
        // The front face: (0,0) samples (var6, var8), the u axis flipped.
        assert_eq!(f[0].1[0], [0.0, 0.0, 0.0, u6, v8]);
        assert_eq!(f[0].1[2], [1.0, 1.0, 0.0, u5, v7]);
        // The back face sits one pixel (0.0625) behind.
        assert_eq!(f[1].1[0][2], -0.0625);
        // The last +x slice is at 15/16 + 1/16 = 1, its u just inside
        // the 16th texel column: var6 + (var5 - var6) * 15/16 - 1/512.
        let last = f[3].1[60];
        assert_eq!(last[0], 1.0);
        let var16 =
            31.99f32 / 256.0 + (16.0f32 / 256.0 - 31.99f32 / 256.0) * (15.0 / 16.0) - 0.001953125;
        assert_eq!(last[3], var16 as f64);
        // The first -y slice is at y = 0, the first +y one at y = 1/16.
        assert_eq!(f[5].1[0][1], 0.0);
        assert_eq!(f[4].1[0][1], 0.0625);
    }
}
