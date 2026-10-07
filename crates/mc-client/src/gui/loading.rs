//! The loading screen's drawing: `LoadingScreenRenderer.setLoadingProgress(int)`
//! (LoadingScreenRenderer.java:59-122) over the shell's state
//! ([`crate::loading::LoadingScreenRenderer`], in `Game::loading_screen`):
//! the dirt background tinted `0x404040`, the title line and the message
//! line centred with shadows, and with a progress of 0 or more the 100 × 2
//! bar (grey `0x808080`, green `0x80ff80` up to the percentage).
//!
//! Vanilla repaints from inside the blocking load, at most every 20 ms
//! (`field_1006_d`); the shell's load is spread over frames
//! ([`crate::game::PendingLoad`]), so the screen is drawn every frame while
//! one is pending ([`draw_loading_screen`], in [`super::GuiSet::Screen`] after
//! the screen, which is not drawn meanwhile). Leaving a world saves inside
//! one frame, so its "Saving level" / "Saving chunks" screen is never seen.

use bevy::prelude::*;

use super::batch::{GuiGl, GuiLayer, gl};
use super::font::FontRenderer;
use super::screen::BACKGROUND_PNG;
use crate::app::ScaledResolution;
use crate::game::Game;
use crate::loading::LoadingScreenRenderer;

/// The bar's width and height.
// src: LoadingScreenRenderer.java:91-92
pub const BAR_WIDTH: i32 = 100;
/// The bar's height.
pub const BAR_HEIGHT: i32 = 2;
/// The bar's background (`8421504`, `0x808080`).
// src: LoadingScreenRenderer.java:97
pub const BAR_BACK: i32 = 8421504;
/// The bar's progress colour (`8454016`, `0x80ff80`).
// src: LoadingScreenRenderer.java:102
pub const BAR_FRONT: i32 = 8454016;

/// Where the bar starts: `(width / 2 - 50, height / 2 + 16)`.
// src: LoadingScreenRenderer.java:93-94
pub fn bar_origin(var5: i32, var6: i32) -> (i32, i32) {
    (var5 / 2 - BAR_WIDTH / 2, var6 / 2 + 16)
}

/// `setLoadingProgress(int)`'s drawing at the scaled size `var5 × var6`.
// src: LoadingScreenRenderer.java:79-111
pub fn draw_loading(
    g: &mut GuiGl,
    font: &FontRenderer,
    l: &LoadingScreenRenderer,
    var5: i32,
    var6: i32,
) {
    let var1 = l.progress;
    g.bind_texture(BACKGROUND_PNG);
    let var9 = 32.0f32;
    g.start_drawing_quads();
    g.set_color_opaque_i(4210752);
    g.add_vertex_with_uv(0.0, var6 as f64, 0.0, 0.0, (var6 as f32 / var9) as f64);
    g.add_vertex_with_uv(
        var5 as f64,
        var6 as f64,
        0.0,
        (var5 as f32 / var9) as f64,
        (var6 as f32 / var9) as f64,
    );
    g.add_vertex_with_uv(var5 as f64, 0.0, 0.0, (var5 as f32 / var9) as f64, 0.0);
    g.add_vertex_with_uv(0.0, 0.0, 0.0, 0.0, 0.0);
    g.draw();
    if var1 >= 0 {
        let var10 = BAR_WIDTH;
        let var11 = BAR_HEIGHT;
        let (var12, var13) = bar_origin(var5, var6);
        g.disable(gl::TEXTURE_2D);
        g.start_drawing_quads();
        g.set_color_opaque_i(BAR_BACK);
        g.add_vertex(var12 as f64, var13 as f64, 0.0);
        g.add_vertex(var12 as f64, (var13 + var11) as f64, 0.0);
        g.add_vertex((var12 + var10) as f64, (var13 + var11) as f64, 0.0);
        g.add_vertex((var12 + var10) as f64, var13 as f64, 0.0);
        g.set_color_opaque_i(BAR_FRONT);
        g.add_vertex(var12 as f64, var13 as f64, 0.0);
        g.add_vertex(var12 as f64, (var13 + var11) as f64, 0.0);
        g.add_vertex((var12 + var1) as f64, (var13 + var11) as f64, 0.0);
        g.add_vertex((var12 + var1) as f64, var13 as f64, 0.0);
        g.draw();
        g.enable(gl::TEXTURE_2D);
    }
    let t = &l.currently_displayed_text;
    font.draw_string_with_shadow(
        g,
        t,
        (var5 - font.get_string_width(t)) / 2,
        var6 / 2 - 4 - 16,
        16777215,
    );
    let m = &l.field_1004_a;
    font.draw_string_with_shadow(
        g,
        m,
        (var5 - font.get_string_width(m)) / 2,
        var6 / 2 - 4 + 8,
        16777215,
    );
}

/// The loading screen while a world change is pending (module docs). Drawn
/// on the screen layer, over anything else of the frame.
pub fn draw_loading_screen(game: NonSend<Game>, font: Res<FontRenderer>, mut g: ResMut<GuiGl>) {
    if game.pending.is_none() {
        return;
    }
    let r = ScaledResolution::new(
        game.game_settings.gui_scale,
        game.display_width,
        game.display_height,
    );
    g.set_layer(GuiLayer::Screen);
    draw_loading(
        &mut g,
        &font,
        &game.loading_screen,
        r.scaled_width,
        r.scaled_height,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::batch::TexRef;
    use crate::gui::font::ChatAllowedCharacters;
    use crate::loading::IProgressUpdate;
    use crate::textures::ArgbImage;

    #[test]
    fn bar_and_background() {
        let font = FontRenderer::new(
            &ArgbImage::new(128, 128),
            false,
            ChatAllowedCharacters::from_chars(Vec::new()),
        );
        let mut l = LoadingScreenRenderer::default();
        l.print_text("Generating level");
        l.display_loading_string("Building terrain");
        let mut g = GuiGl::default();
        draw_loading(&mut g, &font, &l, 427, 240);
        // No progress yet: only the background.
        let d = g.take_draws();
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].texture, Some(TexRef::from(BACKGROUND_PNG)));
        l.set_loading_progress(42);
        draw_loading(&mut g, &font, &l, 427, 240);
        let d = g.take_draws();
        assert_eq!(d.len(), 2);
        assert_eq!(d[1].texture, None);
        assert_eq!(bar_origin(427, 240), (163, 136));
    }
}
