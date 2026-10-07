//! WP-3.6c's screens in the [`ScreenRegistry`] (the shell opens
//! `GuiMainMenu` at start-up and when no world is left, and `GuiIngameMenu`
//! on Escape, by kind; the others are registered for the `open_screen`
//! dev action, with no parent) and [`MenuPlugin`]: the registrations, the
//! title screen's splash lines from the jar, and the loading screen's
//! drawing ([`super::loading`]).

use bevy::prelude::*;

use super::GuiSet;
use super::menu::{
    achievements, controls, create_world, import_world, ingame_menu, main_menu, options,
    select_world, stats, texture_packs, video_settings,
};
use super::screen::ScreenRegistry;
use crate::assets::JarAssets;

/// Registers the menus by class name.
pub fn register(r: &mut ScreenRegistry) {
    r.register("GuiMainMenu", main_menu::create);
    r.register("GuiIngameMenu", || {
        Box::new(ingame_menu::GuiIngameMenu::default())
    });
    r.register("GuiOptions", || Box::new(options::GuiOptions::new(None)));
    r.register("GuiVideoSettings", || {
        Box::new(video_settings::GuiVideoSettings::new(None))
    });
    r.register("GuiControls", || Box::new(controls::GuiControls::new(None)));
    r.register("GuiSelectWorld", || {
        Box::new(select_world::GuiSelectWorld::new(None))
    });
    r.register("GuiCreateWorld", || {
        Box::new(create_world::GuiCreateWorld::new(None))
    });
    r.register("GuiAchievements", || {
        Box::new(achievements::GuiAchievements::new())
    });
    r.register("GuiStats", || Box::new(stats::GuiStats::new(None)));
    r.register("GuiTexturePacks", || {
        Box::new(texture_packs::GuiTexturePacks::new(None))
    });
    r.register("GuiImportWorld", || {
        Box::new(import_world::GuiImportWorld::new(None))
    });
}

/// The menus' plugin (module docs). Added by [`crate::ClientPlugin`] after
/// [`super::GuiPlugin`].
pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        let splashes = app
            .world()
            .get_resource::<JarAssets>()
            .and_then(|j| j.get(main_menu::SPLASHES_TXT).map(<[u8]>::to_vec));
        main_menu::set_splashes(splashes.as_deref());
        if let Some(mut r) = app.world_mut().get_resource_mut::<ScreenRegistry>() {
            register(&mut r);
        } else {
            let mut r = ScreenRegistry::default();
            register(&mut r);
            app.insert_resource(r);
        }
        app.add_systems(
            Update,
            super::loading::draw_loading_screen
                .in_set(GuiSet::Screen)
                .after(super::draw_current_screen),
        );
    }
}
