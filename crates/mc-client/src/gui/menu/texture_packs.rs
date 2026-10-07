//! `GuiTexturePacks` and `GuiTexturePackSlot` (GuiTexturePacks.java,
//! GuiTexturePackSlot.java) with texture packs deferred (PLAN_v4 D3): the
//! list holds only `TexturePackDefault` ("Default", "The default look of
//! Minecraft", its `/pack.png` thumbnail), always selected; "Open texture
//! pack folder" is disabled (`Sys.openURL` of `texturepacks/`); Done
//! returns to the parent. `TexturePackList.updateAvaliableTexturePacks`
//! (scanning `texturepacks/`) and Done's `renderEngine.refreshTextures()`
//! are skipped: no other pack can be chosen, so nothing changes.

use super::display_parent;
use crate::app::ScreenKind;
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::screen::{
    Screen, ScreenBase, ScreenCtx, default_draw_screen, draw_default_background,
};
use crate::gui::widgets::button::GuiButton;
use crate::gui::widgets::slot::{GuiSlot, SlotList};

/// `TexturePackDefault`'s fields: `texturePackFileName`,
/// `firstDescriptionLine`, `secondDescriptionLine` (unset: Java's null,
/// which `drawString` does not draw).
// src: TexturePackDefault.java:13-16
pub const DEFAULT_PACK: (&str, &str) = ("Default", "The default look of Minecraft");
/// The default pack's thumbnail.
// src: TexturePackDefault.java:18
pub const PACK_PNG: &str = "/pack.png";

/// `GuiTexturePackSlot` over the one default pack.
#[derive(Clone, Copy, Debug)]
pub struct TexturePackSlot {
    width: i32,
    height: i32,
}

impl TexturePackSlot {
    /// `new GuiSlot(mc, width, height, 32, height - 55 + 4, 36)`.
    // src: GuiTexturePackSlot.java:9-12
    pub fn new_slot(width: i32, height: i32) -> GuiSlot {
        GuiSlot::new(width, height, 32, height - 55 + 4, 36)
    }
}

impl SlotList for TexturePackSlot {
    /// `availableTexturePacks().size()`: the default pack only.
    // src: GuiTexturePackSlot.java:14-17
    fn get_size(&self) -> i32 {
        1
    }
    /// `setTexturePack(pack)` + `refreshTextures()`: selecting the default
    /// pack again changes nothing.
    // src: GuiTexturePackSlot.java:19-23
    fn element_clicked(&mut self, _ctx: &mut ScreenCtx, _index: i32, _double: bool) {}
    /// `selectedTexturePack == pack`: the default is always selected.
    // src: GuiTexturePackSlot.java:25-28
    fn is_selected(&self, index: i32) -> bool {
        index == 0
    }
    // src: GuiTexturePackSlot.java:30-32
    fn get_content_height(&self, _slot: &GuiSlot) -> i32 {
        self.get_size() * 36
    }
    // src: GuiTexturePackSlot.java:34-36
    fn draw_background(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl) {
        let b = ScreenBase {
            width: self.width,
            height: self.height,
            ..ScreenBase::default()
        };
        draw_default_background(&b, ctx, g);
    }
    // src: GuiTexturePackSlot.java:38-52
    fn draw_slot(
        &mut self,
        ctx: &mut ScreenCtx,
        g: &mut GuiGl,
        _i: i32,
        var2: i32,
        var3: i32,
        var4: i32,
    ) {
        // bindThumbnailTexture: the thumbnail uploaded once
        // (allocateAndSetupTexture), here the texture cache's /pack.png.
        g.bind_texture(PACK_PNG);
        g.color4f(1.0, 1.0, 1.0, 1.0);
        g.start_drawing_quads();
        g.set_color_opaque_i(16777215);
        g.add_vertex_with_uv(var2 as f64, (var3 + var4) as f64, 0.0, 0.0, 1.0);
        g.add_vertex_with_uv((var2 + 32) as f64, (var3 + var4) as f64, 0.0, 1.0, 1.0);
        g.add_vertex_with_uv((var2 + 32) as f64, var3 as f64, 0.0, 1.0, 0.0);
        g.add_vertex_with_uv(var2 as f64, var3 as f64, 0.0, 0.0, 0.0);
        g.draw();
        let f = ctx.font;
        draw::draw_string(g, f, DEFAULT_PACK.0, var2 + 32 + 2, var3 + 1, 16777215);
        draw::draw_string(g, f, DEFAULT_PACK.1, var2 + 32 + 2, var3 + 12, 8421504);
        // secondDescriptionLine is null: FontRenderer.renderString draws
        // nothing (FontRenderer.java:125).
    }
}

/// `GuiTexturePacks`.
pub struct GuiTexturePacks {
    base: ScreenBase,
    /// `guiScreen`: the parent.
    gui_screen: Option<Box<dyn Screen>>,
    /// `refreshTimer`.
    pub refresh_timer: i32,
    /// `guiTexturePackSlot`.
    slot: Option<(GuiSlot, TexturePackSlot)>,
}

impl GuiTexturePacks {
    /// `new GuiTexturePacks(parent)`.
    // src: GuiTexturePacks.java:13-15
    pub fn new(parent: Option<Box<dyn Screen>>) -> Self {
        GuiTexturePacks {
            base: ScreenBase::default(),
            gui_screen: parent,
            refresh_timer: -1,
            slot: None,
        }
    }
}

impl Screen for GuiTexturePacks {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::Other("GuiTexturePacks".into())
    }

    // src: GuiTexturePacks.java:17-25
    fn init_gui(&mut self, ctx: &mut ScreenCtx) {
        let var1 = ctx.lang;
        let (w, h) = (self.base.width, self.base.height);
        let mut open = GuiButton::small(
            5,
            w / 2 - 154,
            h - 48,
            var1.translate_key("texturePack.openFolder"),
        );
        // Deferred (D3): there is no texturepacks/ folder to open.
        open.enabled = false;
        self.base.control_list.push(open);
        self.base.control_list.push(GuiButton::small(
            6,
            w / 2 + 4,
            h - 48,
            var1.translate_key("gui.done"),
        ));
        let mut s = TexturePackSlot::new_slot(w, h);
        s.register_scroll_buttons(7, 8);
        self.slot = Some((
            s,
            TexturePackSlot {
                width: w,
                height: h,
            },
        ));
    }

    // src: GuiTexturePacks.java:27-39
    fn action_performed(&mut self, ctx: &mut ScreenCtx, button: usize) {
        let b = self.base.control_list[button].clone();
        if !b.enabled {
            return;
        }
        if b.id == 5 {
            // Sys.openURL("file://" + fileLocation): deferred (D3).
        } else if b.id == 6 {
            // renderEngine.refreshTextures(): nothing can have changed
            // (module docs).
            display_parent(ctx, &mut self.gui_screen);
        } else if let Some((s, l)) = self.slot.as_mut() {
            s.action_performed(l, &b);
        }
    }

    // src: GuiTexturePacks.java:49-60
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, p: f32) {
        if let Some((s, l)) = self.slot.as_mut() {
            s.draw_screen(l, ctx, g, mx, my, p);
        }
        if self.refresh_timer <= 0 {
            // texturePackList.updateAvaliableTexturePacks(): deferred.
            self.refresh_timer += 20;
        }
        let var4 = ctx.lang;
        let (w, h) = (self.base.width, self.base.height);
        draw::draw_centered_string(
            g,
            ctx.font,
            &var4.translate_key("texturePack.title"),
            w / 2,
            16,
            16777215,
        );
        draw::draw_centered_string(
            g,
            ctx.font,
            &var4.translate_key("texturePack.folderInfo"),
            w / 2 - 77,
            h - 26,
            8421504,
        );
        default_draw_screen(&mut self.base, ctx, g, mx, my);
    }

    // src: GuiTexturePacks.java:62-65
    fn update_screen(&mut self, _ctx: &mut ScreenCtx) {
        self.refresh_timer -= 1;
    }
}
