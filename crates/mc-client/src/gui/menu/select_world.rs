//! `GuiSelectWorld` (GuiSelectWorld.java): the saves list (sorted by
//! `SaveFormatComparator`: most recently played first), Play Selected
//! World / Rename / Delete (with a `GuiYesNo`) / Create New World / Cancel.
//!
//! Project addition (PLAN_v4 D5, §3.5): an "Import..." button in the
//! title band's right corner (`width - 74, 6`, 70 × 20; the vanilla
//! buttons keep their places) opens [`GuiImportWorld`].
//!
//! [`GuiImportWorld`]: super::import_world::GuiImportWorld

use mc_sim::player_controller::PlayerControllerSp;
use mc_sim::save::{SaveFormat, SaveFormatComparator};

use super::create_world::GuiCreateWorld;
use super::display_parent;
use super::import_world::GuiImportWorld;
use super::rename_world::GuiRenameWorld;
use super::world_slot::{WorldSlotList, new_slot};
use crate::app::ScreenKind;
use crate::gui::batch::GuiGl;
use crate::gui::draw;
use crate::gui::screen::{Screen, ScreenBase, ScreenCtx, default_draw_screen};
use crate::gui::widgets::button::GuiButton;
use crate::gui::widgets::slot::GuiSlot;
use crate::gui::widgets::yesno::GuiYesNo;

/// The Import... button's id (project-owned; vanilla's are 0-6).
pub const IMPORT_BUTTON: i32 = 100;
/// Its label.
pub const IMPORT_LABEL: &str = "Import...";

/// `GuiSelectWorld`.
pub struct GuiSelectWorld {
    base: ScreenBase,
    /// `parentScreen`.
    parent_screen: Option<Box<dyn Screen>>,
    /// `screenTitle`.
    pub screen_title: String,
    /// `selected`: a world is being opened (a second Play does nothing).
    pub selected: bool,
    /// `selectedWorld`.
    pub selected_world: i32,
    /// `saveList`.
    pub save_list: Vec<SaveFormatComparator>,
    /// `worldSlotContainer`.
    world_slot_container: Option<GuiSlot>,
    /// `localizedWorldText`.
    localized_world_text: String,
    /// `localizedMustConvertText`.
    localized_must_convert_text: String,
    /// `deleting`: a delete confirmation is open.
    deleting: bool,
    /// `buttonSelect`, `buttonRename`, `buttonDelete` as control-list
    /// indices.
    buttons: [usize; 3],
}

impl GuiSelectWorld {
    /// `new GuiSelectWorld(parent)`.
    // src: GuiSelectWorld.java:23-25
    pub fn new(parent: Option<Box<dyn Screen>>) -> Self {
        GuiSelectWorld {
            base: ScreenBase::default(),
            parent_screen: parent,
            screen_title: "Select world".to_string(),
            selected: false,
            selected_world: 0,
            save_list: Vec::new(),
            world_slot_container: None,
            localized_world_text: String::new(),
            localized_must_convert_text: String::new(),
            deleting: false,
            buttons: [0, 1, 2],
        }
    }

    /// `loadSaves()`: `getSaveList()`, sorted, nothing selected.
    // src: GuiSelectWorld.java:38-43
    fn load_saves(&mut self, ctx: &mut ScreenCtx) {
        self.save_list = ctx.game.save_loader.get_save_list();
        sort_saves(&mut self.save_list);
        self.selected_world = -1;
    }

    /// `initButtons()`.
    // src: GuiSelectWorld.java:59-69
    fn init_buttons(&mut self, ctx: &mut ScreenCtx) {
        let var1 = ctx.lang;
        let (w, h) = (self.base.width, self.base.height);
        let c = &mut self.base.control_list;
        c.push(GuiButton::with_size(
            1,
            w / 2 - 154,
            h - 52,
            150,
            20,
            var1.translate_key("selectWorld.select"),
        ));
        let select = c.len() - 1;
        c.push(GuiButton::with_size(
            6,
            w / 2 - 154,
            h - 28,
            70,
            20,
            var1.translate_key("selectWorld.rename"),
        ));
        let rename = c.len() - 1;
        c.push(GuiButton::with_size(
            2,
            w / 2 - 74,
            h - 28,
            70,
            20,
            var1.translate_key("selectWorld.delete"),
        ));
        let delete = c.len() - 1;
        c.push(GuiButton::with_size(
            3,
            w / 2 + 4,
            h - 52,
            150,
            20,
            var1.translate_key("selectWorld.create"),
        ));
        c.push(GuiButton::with_size(
            0,
            w / 2 + 4,
            h - 28,
            150,
            20,
            var1.translate_key("gui.cancel"),
        ));
        c[select].enabled = false;
        c[rename].enabled = false;
        c[delete].enabled = false;
        self.buttons = [select, rename, delete];
        // D5: the import (project-owned).
        c.push(GuiButton::with_size(
            IMPORT_BUTTON,
            w - 74,
            6,
            70,
            20,
            IMPORT_LABEL,
        ));
    }
}

/// `Collections.sort(saveList)`: a stable sort by
/// `SaveFormatComparator.compareTo` (latest played first, then the folder
/// name).
// src: GuiSelectWorld.java:41, SaveFormatComparator.java:38-44
pub fn sort_saves(list: &mut [SaveFormatComparator]) {
    list.sort();
}

/// `getSaveFileName(i)`.
// src: GuiSelectWorld.java:45-47
pub fn get_save_file_name(list: &[SaveFormatComparator], var1: i32) -> Option<String> {
    usize::try_from(var1)
        .ok()
        .and_then(|i| list.get(i))
        .map(|s| s.file_name.clone())
}

/// `getSaveName(i)`: the display name, or `"World N"` (translated) when
/// empty.
// src: GuiSelectWorld.java:49-57
pub fn get_save_name(ctx: &ScreenCtx, list: &[SaveFormatComparator], var1: i32) -> Option<String> {
    let s = usize::try_from(var1).ok().and_then(|i| list.get(i))?;
    let mut var2 = s.display_name.clone();
    if var2.is_empty() {
        var2 = format!(
            "{} {}",
            ctx.lang.translate_key("selectWorld.world"),
            var1 + 1
        );
    }
    Some(var2)
}

/// `selectWorld(i)`: closes the screen, then (once) a new
/// `PlayerControllerSP` and `startWorld(folder, name, 0)`.
// src: GuiSelectWorld.java:100-113
pub fn select_world(
    ctx: &mut ScreenCtx,
    list: &[SaveFormatComparator],
    selected: &mut bool,
    var1: i32,
) {
    ctx.display_gui_screen(None);
    if !*selected {
        *selected = true;
        ctx.game.player_controller = Some(PlayerControllerSp::new());
        let var2 = get_save_file_name(list, var1).unwrap_or_else(|| format!("World{var1}"));
        let name = get_save_name(ctx, list, var1).unwrap_or_default();
        if let Err(e) = ctx.game.start_world(&var2, &name, 0) {
            bevy::log::error!("{e}");
            ctx.game.last_error = Some(e);
        }
        ctx.display_gui_screen(None);
    }
}

impl Screen for GuiSelectWorld {
    fn base(&self) -> &ScreenBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn kind(&self) -> ScreenKind {
        ScreenKind::Other("GuiSelectWorld".into())
    }

    // src: GuiSelectWorld.java:27-36
    fn init_gui(&mut self, ctx: &mut ScreenCtx) {
        let var1 = ctx.lang;
        self.screen_title = var1.translate_key("selectWorld.title");
        self.localized_world_text = var1.translate_key("selectWorld.world");
        self.localized_must_convert_text = var1.translate_key("selectWorld.conversion");
        self.load_saves(ctx);
        let mut slot = new_slot(self.base.width, self.base.height);
        slot.register_scroll_buttons(4, 5);
        self.world_slot_container = Some(slot);
        self.init_buttons(ctx);
    }

    // src: GuiSelectWorld.java:71-98
    fn action_performed(&mut self, ctx: &mut ScreenCtx, button: usize) {
        if !self.base.control_list[button].enabled {
            return;
        }
        let id = self.base.control_list[button].id;
        if id == 2 {
            if let Some(var2) = get_save_name(ctx, &self.save_list, self.selected_world) {
                self.deleting = true;
                let var3 = ctx.lang;
                let var4 = var3.translate_key("selectWorld.deleteQuestion");
                let var5 = format!(
                    "'{var2}' {}",
                    var3.translate_key("selectWorld.deleteWarning")
                );
                let var6 = var3.translate_key("selectWorld.deleteButton");
                let var7 = var3.translate_key("gui.cancel");
                let n = self.selected_world;
                ctx.display_wrapping_this(move |this| {
                    Box::new(GuiYesNo::new(this, var4, var5, var6, var7, n))
                });
            }
        } else if id == 1 {
            let n = self.selected_world;
            select_world(ctx, &self.save_list, &mut self.selected, n);
        } else if id == 3 {
            ctx.display_wrapping_this(|this| Box::new(GuiCreateWorld::new(Some(this))));
        } else if id == 6 {
            let name = get_save_file_name(&self.save_list, self.selected_world).unwrap_or_default();
            ctx.display_wrapping_this(move |this| Box::new(GuiRenameWorld::new(Some(this), name)));
        } else if id == 0 {
            display_parent(ctx, &mut self.parent_screen);
        } else if id == IMPORT_BUTTON {
            ctx.display_wrapping_this(|this| Box::new(GuiImportWorld::new(Some(this))));
        } else if let Some(s) = self.world_slot_container.as_mut() {
            // worldSlotContainer.actionPerformed(button): the scroll buttons
            // 4 and 5 (never in the control list in Beta).
            let b = self.base.control_list[button].clone();
            let list = WorldSlotList {
                save_list: &self.save_list,
                selected_world: &mut self.selected_world,
                selected: &mut self.selected,
                control_list: &mut [],
                buttons: [0; 3],
                localized_world_text: "",
                localized_must_convert_text: "",
                width: 0,
                height: 0,
            };
            s.action_performed(&list, &b);
        }
    }

    // src: GuiSelectWorld.java:115-128
    fn delete_world(&mut self, ctx: &mut ScreenCtx, var1: bool, var2: i32) {
        if self.deleting {
            self.deleting = false;
            if var1 {
                let loader = &mut ctx.game.save_loader;
                loader.flush_cache();
                if let Some(name) = get_save_file_name(&self.save_list, var2) {
                    loader.delete_world_directory(&name);
                }
                self.load_saves(ctx);
            }
            ctx.display_this();
        }
    }

    // src: GuiSelectWorld.java:130-134
    fn draw_screen(&mut self, ctx: &mut ScreenCtx, g: &mut GuiGl, mx: i32, my: i32, p: f32) {
        let (w, h) = (self.base.width, self.base.height);
        if let Some(slot) = self.world_slot_container.as_mut() {
            let mut list = WorldSlotList {
                save_list: &self.save_list,
                selected_world: &mut self.selected_world,
                selected: &mut self.selected,
                control_list: &mut self.base.control_list,
                buttons: self.buttons,
                localized_world_text: &self.localized_world_text,
                localized_must_convert_text: &self.localized_must_convert_text,
                width: w,
                height: h,
            };
            slot.draw_screen(&mut list, ctx, g, mx, my, p);
        }
        draw::draw_centered_string(g, ctx.font, &self.screen_title, w / 2, 20, 16777215);
        default_draw_screen(&mut self.base, ctx, g, mx, my);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_sort_latest_first_then_by_folder() {
        let mut l = vec![
            SaveFormatComparator::new("b", "B", 100, 0, false),
            SaveFormatComparator::new("a", "A", 50, 0, false),
            SaveFormatComparator::new("c", "C", 300, 0, false),
            SaveFormatComparator::new("Z", "Z", 100, 0, false),
        ];
        sort_saves(&mut l);
        let names: Vec<&str> = l.iter().map(|s| s.file_name.as_str()).collect();
        // Same time: String.compareTo ('Z' 90 < 'b' 98).
        assert_eq!(names, vec!["c", "Z", "b", "a"]);
        assert_eq!(get_save_file_name(&l, 1).as_deref(), Some("Z"));
        assert_eq!(get_save_file_name(&l, -1), None);
        assert_eq!(get_save_file_name(&l, 4), None);
    }
}
