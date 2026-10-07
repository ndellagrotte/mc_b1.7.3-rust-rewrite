//! `GuiChest` (GuiChest.java): a chest or double chest over the player's
//! inventory, `/gui/container.png` cut at the chest's rows.

use mc_sim::inventory::InventoryRef;
use mc_sim::inventory::container::chest::new_container_chest;
use mc_sim::world::World;

use super::{ContainerVariant, GuiContainer, LABEL_COLOR, inv_name};
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::font::FontRenderer;

/// `/gui/container.png`.
// src: GuiChest.java:27
pub const CONTAINER_PNG: &str = "/gui/container.png";

/// `GuiChest`'s fields.
#[derive(Clone, Debug)]
pub struct ChestState {
    /// `upperChestInventory`'s name (the player's inventory).
    // src: GuiChest.java:6
    pub upper_name: String,
    /// `lowerChestInventory`'s name (the chest).
    // src: GuiChest.java:7
    pub lower_name: String,
    /// `inventoryRows`.
    // src: GuiChest.java:8
    pub inventory_rows: i32,
}

/// `ySize`: `222 - 108 + rows * 18`.
// src: GuiChest.java:15-18
pub fn chest_y_size(rows: i32) -> i32 {
    let var3 = 222;
    let var4 = var3 - 108;
    var4 + rows * 18
}

/// `new GuiChest(player.inventory, chest)`: a `ContainerChest`, user input
/// off, the height from the chest's rows.
// src: GuiChest.java:10-19
pub fn new_gui_chest(world: &World, var2: InventoryRef) -> GuiContainer {
    let container = new_container_chest(world, var2.clone());
    let inventory_rows = container.inv_get_size_inventory(world, &var2) / 9;
    let mut g = GuiContainer::new(
        Some(container),
        ContainerVariant::Chest(ChestState {
            upper_name: inv_name(world, &InventoryRef::Player),
            lower_name: inv_name(world, &var2),
            inventory_rows,
        }),
    );
    g.base.allow_user_input = false;
    g.y_size = chest_y_size(inventory_rows);
    g
}

impl ChestState {
    /// `drawGuiContainerForegroundLayer()`: the chest's name and the
    /// inventory's.
    // src: GuiChest.java:21-24
    pub fn draw_foreground(&self, g: &mut GuiGl, f: &FontRenderer, y_size: i32) {
        f.draw_string(g, &self.lower_name, 8, 6, LABEL_COLOR);
        f.draw_string(g, &self.upper_name, 8, y_size - 96 + 2, LABEL_COLOR);
    }

    /// `drawGuiContainerBackgroundLayer(partial)`: the top of the sheet
    /// down to the chest's last row, then the player's part.
    // src: GuiChest.java:26-34
    pub fn draw_background(&self, g: &mut GuiGl, w: i32, h: i32, x_size: i32, y_size: i32) {
        g.color4f(1.0, 1.0, 1.0, 1.0);
        g.bind_texture(CONTAINER_PNG);
        let var3 = (w - x_size) / 2;
        let var4 = (h - y_size) / 2;
        draw::draw_textured_modal_rect(
            g,
            0.0,
            var3,
            var4,
            0,
            0,
            x_size,
            self.inventory_rows * 18 + 17,
        );
        draw::draw_textured_modal_rect(
            g,
            0.0,
            var3,
            var4 + self.inventory_rows * 18 + 17,
            0,
            126,
            x_size,
            96,
        );
    }
}
