//! `Achievement` (Achievement.java): a stat with a place in the achievement
//! map, a parent, an icon and a description.
//!
//! The `IStatStringFormat` (`statStringFormatter`, set only on
//! `openInventory` by the client with `StatStringFormatKeyInv`, which puts
//! the inventory key's name into the description: Minecraft.java:248,
//! StatStringFormatKeyInv.java:14-16) and `getDescription()` are the GUI's;
//! the port keeps the description key ([`Achievement::achievement_description`]).

use super::stat_base::{StatBase, StatKind, StatName, StatType};
use crate::item::ItemStack;

/// The offset of an achievement's `statId`: `5242880 + id`.
// src: Achievement.java:21
pub const ACHIEVEMENT_ID_BASE: i32 = 5242880;

/// The fields `Achievement` declares.
#[derive(Clone, Debug, PartialEq)]
pub struct Achievement {
    /// `displayColumn`.
    pub display_column: i32,
    /// `displayRow`.
    pub display_row: i32,
    /// `parentAchievement`, as its `statId`.
    pub parent_achievement: Option<i32>,
    /// `achievementDescription` (private), as its key
    /// `"achievement." + name + ".desc"`.
    pub achievement_description: String,
    /// `theItemStack`: the icon, `new ItemStack(item)` / `new
    /// ItemStack(block)` (count 1, damage 0).
    pub the_item_stack: ItemStack,
    /// `isSpecial` (private; `getSpecial()`): drawn with the special frame.
    pub is_special: bool,
}

impl Achievement {
    /// `new Achievement(id, name, column, row, icon, parent)`: `statId`
    /// `5242880 + id`, name key `"achievement." + name`, `StatTypeSimple`.
    /// The constructor also widens `AchievementList`'s display bounds; the
    /// list does that when it registers ([`super::StatList`]).
    // src: Achievement.java:12-43
    pub fn new_stat(
        id: i32,
        name: &str,
        column: i32,
        row: i32,
        icon: ItemStack,
        parent: Option<i32>,
    ) -> StatBase {
        StatBase {
            stat_id: ACHIEVEMENT_ID_BASE + id,
            stat_name: StatName::plain(format!("achievement.{name}")),
            is_independent: false,
            stat_type: StatType::Simple,
            kind: StatKind::Achievement(Achievement {
                display_column: column,
                display_row: row,
                parent_achievement: parent,
                achievement_description: format!("achievement.{name}.desc"),
                the_item_stack: icon,
                is_special: false,
            }),
        }
    }

    /// `getSpecial()`.
    // src: Achievement.java:74-76
    pub fn get_special(&self) -> bool {
        self.is_special
    }
}

/// `setSpecial()` on an achievement stat.
// src: Achievement.java:50-53
pub fn set_special(mut s: StatBase) -> StatBase {
    if let StatKind::Achievement(a) = &mut s.kind {
        a.is_special = true;
    }
    s
}
