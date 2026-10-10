//! `GuiInventory` (GuiInventory.java): the player's inventory with the 2×2
//! crafting grid, `/gui/inventory.png`, and the player drawn in the box at
//! the left, turned toward the pointer.
//!
//! The player preview is `RenderManager.renderEntityWithPosYaw(thePlayer,
//! 0, 0, 0, 0, 1)` under the screen's matrix: since WP-4.9 it runs
//! WP-3.5b's renderers themselves (`RenderPlayer`: the model, the armour
//! passes, the held item and pumpkin, the hurt and death overlay;
//! `doRenderShadowAndFire`: the shadow with fancy graphics and the fire
//! while burning) and draws their output through the batcher
//! ([`crate::gui::entity_draws`]), instead of WP-3.6b's copy of
//! `RenderPlayer`, which had no overlay, shadow or fire. Not drawn: the
//! cloak (a download, D2).

use mc_core::math::jmath;
use mc_sim::entity::player as player_fns;
use mc_sim::inventory::container::Container;
use mc_sim::inventory::container::player::new_container_player;
use mc_sim::stats::{achievement_list::OPEN_INVENTORY, stat};

use super::LABEL_COLOR;
use crate::game::Game;
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::entity_draws;
use crate::gui::font::FontRenderer;
use crate::gui::ingame::with_player;
use crate::gui::render_item::ItemGl;
use crate::gui::screen::ScreenCtx;

/// `/gui/inventory.png`.
// src: GuiInventory.java:31
pub const INVENTORY_PNG: &str = "/gui/inventory.png";
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
        // The player's angles and entityBrightness are set for the draw
        // and put back (GuiInventory.java:44-60); the copy here is the drawn
        // one.
        let lights_eye = ig.lights;
        let entities = ctx.entities.as_mut();
        let drawn = with_player(ctx.game, |world, p| {
            let mut e = p.clone();
            if let Some(l) = e.kind.living_mut() {
                l.render_yaw_offset = yaw_offset;
            }
            e.base.rotation_yaw = yaw;
            e.base.rotation_pitch = pitch;
            e.base.entity_brightness = 1.0;
            ig.translatef(g, 0.0, e.base.y_offset, 0.0);
            let m = ig.matrix;
            let lights = entity_draws::lights_in_start_space(m, lights_eye);
            entities.map(|r| (m, lights, r.render(world, &e, lights, 1.0)))
        });
        if let Some(Some((m, lights, list))) = drawn {
            entity_draws::emit(g, m, &list, lights);
        }
        ig.pop_matrix(g);
        ig.disable_standard_item_lighting(g);
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
