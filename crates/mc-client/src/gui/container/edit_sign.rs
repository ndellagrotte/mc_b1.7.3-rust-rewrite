//! `GuiEditSign` (GuiEditSign.java): editing the four lines of a sign just
//! placed, with the sign drawn large in the middle.
//!
//! The sign is drawn as `TileEntityRenderer.renderTileEntityAt(sign, -0.5,
//! -0.75, -0.5, 0)` draws it (`TileEntitySignRenderer.renderTileEntitySignAt`
//! with `SignModel`), through the GUI's batcher: the board and the stick
//! (`SignModel`'s two `ModelRenderer`s, built with WP-3.5a's
//! [`ModelPart`]) from `/item/sign.png`, and the four lines in black, the
//! edited one between `> <` while the cursor blinks on. The in-world sign
//! renderer is WP-3.5b's; this is the GUI's own copy of the same draw.
//! Lighting is off here, as the GUI leaves it.
//!
//! The text is the simulation's `TileEntitySign.signText`, edited in
//! place; Done calls `onInventoryChanged` (the world marks the block for
//! saving and redrawing). `lineBeingEdited` is not written into the
//! simulation: vanilla sets it only for the duration of this draw.

use mc_sim::tile_entity::get_block_type;
use mc_sim::world::tile_entity::{TileEntityKindData, TileEntityRef};

use crate::app::ScreenKind;
use crate::gui::batch::GuiGl;
use crate::gui::chat::{KEY_BACK, KEY_RETURN, java_len};
use crate::gui::draw;
use crate::gui::render_item::ItemGl;
use crate::gui::screen::{
    Screen, ScreenBase, ScreenCtx, default_draw_screen, draw_default_background,
};
use crate::gui::widgets::button::GuiButton;
use crate::model::ModelPart;

/// `/item/sign.png`.
// src: TileEntitySignRenderer.java:39
pub const SIGN_PNG: &str = "/item/sign.png";
/// `Block.signPost.blockID`.
// src: Block.java:655
pub const SIGN_POST: i32 = 63;
/// LWJGL `KEY_UP`.
pub const KEY_UP: i32 = 200;
/// LWJGL `KEY_DOWN`.
pub const KEY_DOWN: i32 = 208;

/// `GuiEditSign`.
#[derive(Clone, Debug)]
pub struct GuiEditSign {
    base: ScreenBase,
    /// `screenTitle`.
    // src: GuiEditSign.java:7
    pub screen_title: String,
    /// `entitySign`.
    // src: GuiEditSign.java:8
    pub entity_sign: TileEntityRef,
    /// `updateCounter`.
    // src: GuiEditSign.java:9
    pub update_counter: i32,
    /// `editLine`.
    // src: GuiEditSign.java:10
    pub edit_line: i32,
}

impl GuiEditSign {
    /// `new GuiEditSign(sign)`.
    // src: GuiEditSign.java:13-15
    pub fn new(var1: TileEntityRef) -> Self {
        GuiEditSign {
            base: ScreenBase::default(),
            screen_title: "Edit sign message:".into(),
            entity_sign: var1,
            update_counter: 0,
            edit_line: 0,
        }
    }

    /// The sign's lines, mutably (`None` when the tile entity is gone).
    fn lines<'a>(&self, ctx: &'a mut ScreenCtx) -> Option<&'a mut [String; 4]> {
        let w = ctx.game.the_world.as_mut()?;
        match &mut w.tile_entities.get_mut(&self.entity_sign)?.kind {
            TileEntityKindData::Sign(s) => Some(&mut s.sign_text),
            _ => None,
        }
    }
}

/// `keyTyped`'s editing of one sign (`edit_line`, the lines): up and down
/// (or Enter) move between the lines, Backspace deletes, an allowed
/// character is appended below 15 characters.
// src: GuiEditSign.java:45-65
pub fn edit_sign_key(
    allowed: &crate::gui::font::ChatAllowedCharacters,
    edit_line: &mut i32,
    lines: &mut [String; 4],
    var1: char,
    var2: i32,
) {
    if var2 == KEY_UP {
        *edit_line = (*edit_line - 1) & 3;
    }
    if var2 == KEY_DOWN || var2 == KEY_RETURN {
        *edit_line = (*edit_line + 1) & 3;
    }
    let line = &mut lines[*edit_line as usize];
    if var2 == KEY_BACK && !line.is_empty() {
        line.pop();
    }
    if allowed.is_allowed(var1) && java_len(line) < 15 {
        line.push(var1);
    }
}

/// `SignModel`: the board (24×12×2 at (-12, -14, -1), texture (0, 0)) and
/// the stick (2×14×2 at (-1, -2, -1), texture (0, 14)).
// src: SignModel.java:7-11
pub fn sign_model() -> (ModelPart, ModelPart) {
    let mut board = ModelPart::new("signBoard", 0, 0);
    board.add_box(-12.0, -14.0, -1.0, 24, 12, 2);
    let mut stick = ModelPart::new("signStick", 0, 14);
    stick.add_box(-1.0, -2.0, -1.0, 2, 14, 2);
    (board, stick)
}

impl Screen for GuiEditSign {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::Other("GuiEditSign".into())
    }
    // src: GuiEditSign.java:17-21
    fn init_gui(&mut self, ctx: &mut ScreenCtx) {
        self.base.control_list.clear();
        ctx.enable_repeat_events(true);
        let (w, h) = (self.base.width, self.base.height);
        self.base
            .control_list
            .push(GuiButton::new(0, w / 2 - 100, h / 4 + 120, "Done"));
    }
    // src: GuiEditSign.java:23-29 (the update packet is multiplayer only)
    fn on_gui_closed(&mut self, ctx: &mut ScreenCtx) {
        ctx.enable_repeat_events(false);
    }
    // src: GuiEditSign.java:31-33
    fn update_screen(&mut self, _ctx: &mut ScreenCtx) {
        self.update_counter = self.update_counter.wrapping_add(1);
    }
    // src: GuiEditSign.java:35-43
    fn action_performed(&mut self, ctx: &mut ScreenCtx, button: usize) {
        let b = &self.base.control_list[button];
        if b.enabled && b.id == 0 {
            if let Some(w) = ctx.game.the_world.as_mut() {
                mc_sim::tile_entity::on_inventory_changed(w, self.entity_sign);
            }
            ctx.display_gui_screen(None);
        }
    }
    // src: GuiEditSign.java:45-65
    fn key_typed(&mut self, ctx: &mut ScreenCtx, var1: char, var2: i32) {
        let allowed = ctx.font.allowed.clone();
        let mut line = self.edit_line;
        if let Some(lines) = self.lines(ctx) {
            edit_sign_key(&allowed, &mut line, lines, var1, var2);
        }
        self.edit_line = line;
    }
    // src: GuiEditSign.java:67-107, TileEntitySignRenderer.java:8-65
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, _p: f32) {
        draw_default_background(&self.base, ctx, g);
        let w = self.base.width;
        draw::draw_centered_string(g, ctx.font, &self.screen_title, w / 2, 40, 16777215);
        let mut ig = ItemGl::new();
        ig.push_matrix(g);
        ig.translatef(g, (w / 2) as f32, 0.0, 50.0);
        let var4 = 93.75f32;
        ig.scalef(g, -var4, -var4, -var4);
        ig.rotatef(g, 180.0, 0.0, 1.0, 0.0);
        let (block, meta) = match ctx.game.the_world.as_mut() {
            Some(wd) => {
                let b = get_block_type(wd, self.entity_sign).map(|b| b.block_id);
                (
                    b,
                    mc_sim::tile_entity::get_block_metadata(wd, self.entity_sign),
                )
            }
            None => (None, 0),
        };
        let post = block == Some(SIGN_POST);
        if post {
            let var6 = (meta * 360) as f32 / 16.0;
            ig.rotatef(g, var6, 0.0, 1.0, 0.0);
            ig.translatef(g, 0.0, -1.0625, 0.0);
        } else {
            let mut var7 = 0.0f32;
            if meta == 2 {
                var7 = 180.0;
            }
            if meta == 4 {
                var7 = 90.0;
            }
            if meta == 5 {
                var7 = -90.0;
            }
            ig.rotatef(g, var7, 0.0, 1.0, 0.0);
            ig.translatef(g, 0.0, -1.0625, 0.0);
        }
        let line_being_edited = if self.update_counter / 6 % 2 == 0 {
            self.edit_line
        } else {
            -1
        };
        let text = self.lines(ctx).cloned().unwrap_or_default();
        render_tile_entity_sign_at(
            &mut ig,
            g,
            ctx.font,
            post,
            meta,
            &text,
            line_being_edited,
            [-0.5, -0.75, -0.5],
        );
        ig.pop_matrix(g);
        default_draw_screen(&mut self.base, ctx, g, mx, my);
    }
}

/// `TileEntitySignRenderer.renderTileEntitySignAt(sign, x, y, z, partial)`
/// through the batcher (module docs).
// src: TileEntitySignRenderer.java:8-65
#[allow(clippy::too_many_arguments)]
pub fn render_tile_entity_sign_at(
    ig: &mut ItemGl,
    g: &mut GuiGl,
    font: &crate::gui::font::FontRenderer,
    post: bool,
    meta: i32,
    sign_text: &[String; 4],
    line_being_edited: i32,
    pos: [f64; 3],
) {
    let [var2, var4, var6] = pos;
    let (board, stick) = sign_model();
    ig.push_matrix(g);
    let var10 = 0.6666667f32;
    let show_stick = if post {
        ig.translatef(
            g,
            var2 as f32 + 0.5,
            var4 as f32 + 0.75 * var10,
            var6 as f32 + 0.5,
        );
        let var11 = (meta * 360) as f32 / 16.0;
        ig.rotatef(g, -var11, 0.0, 1.0, 0.0);
        true
    } else {
        let mut var12 = 0.0f32;
        if meta == 2 {
            var12 = 180.0;
        }
        if meta == 4 {
            var12 = 90.0;
        }
        if meta == 5 {
            var12 = -90.0;
        }
        ig.translatef(
            g,
            var2 as f32 + 0.5,
            var4 as f32 + 0.75 * var10,
            var6 as f32 + 0.5,
        );
        ig.rotatef(g, -var12, 0.0, 1.0, 0.0);
        ig.translatef(g, 0.0, -0.3125, -0.4375);
        false
    };
    g.bind_texture(SIGN_PNG);
    ig.push_matrix(g);
    ig.scalef(g, var10, -var10, -var10);
    for (part, shown) in [(&board, true), (&stick, show_stick)] {
        if shown {
            let v = part.display_list(0.0625);
            ig.draw_quads(g, &v.positions, &v.uvs, None, &v.normals);
        }
    }
    ig.pop_matrix(g);
    let var12 = 0.016666668f32 * var10;
    ig.translatef(g, 0.0, 0.5 * var10, 0.07 * var10);
    ig.scalef(g, var12, -var12, var12);
    g.depth_mask(false);
    let var13 = 0;
    for (var14, var15) in sign_text.iter().enumerate() {
        let var14 = var14 as i32;
        let s = if var14 == line_being_edited {
            format!("> {var15} <")
        } else {
            var15.clone()
        };
        font.draw_string(
            g,
            &s,
            -font.get_string_width(&s) / 2,
            var14 * 10 - sign_text.len() as i32 * 5,
            var13,
        );
    }
    g.depth_mask(true);
    g.color4f(1.0, 1.0, 1.0, 1.0);
    ig.pop_matrix(g);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::font::ChatAllowedCharacters;

    #[test]
    fn sign_line_editing() {
        let allowed = ChatAllowedCharacters::from_chars("abc ".chars().collect());
        let mut line = 0;
        let mut lines: [String; 4] = Default::default();
        for _ in 0..20 {
            edit_sign_key(&allowed, &mut line, &mut lines, 'a', 30);
        }
        // 15 characters at most.
        assert_eq!(lines[0].len(), 15);
        // Not allowed: nothing.
        edit_sign_key(&allowed, &mut line, &mut lines, '%', 0);
        edit_sign_key(&allowed, &mut line, &mut lines, '\u{8}', KEY_BACK);
        assert_eq!(lines[0].len(), 14);
        // Enter and down move on, up moves back, wrapping.
        edit_sign_key(&allowed, &mut line, &mut lines, '\r', KEY_RETURN);
        assert_eq!(line, 1);
        edit_sign_key(&allowed, &mut line, &mut lines, '\0', KEY_UP);
        edit_sign_key(&allowed, &mut line, &mut lines, '\0', KEY_UP);
        assert_eq!(line, 3);
        edit_sign_key(&allowed, &mut line, &mut lines, 'b', 48);
        assert_eq!(lines[3], "b");
    }

    #[test]
    fn sign_model_boxes() {
        let (board, stick) = sign_model();
        let b = board.display_list(0.0625);
        assert_eq!(b.positions.len(), 24);
        let xs: Vec<f32> = b.positions.iter().map(|p| p[0]).collect();
        assert_eq!(xs.iter().cloned().fold(f32::MAX, f32::min), -0.75);
        assert_eq!(xs.iter().cloned().fold(f32::MIN, f32::max), 0.75);
        assert_eq!(stick.display_list(0.0625).positions.len(), 24);
    }
}
