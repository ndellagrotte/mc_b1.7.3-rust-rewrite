//! Item registry and item data (`Item.java` and its subclasses' data members,
//! `EnumToolMaterial`, `ItemStack`; PLAN_v4 §3.2, WP-1.2).
//!
//! # Shape (mirrors [`crate::block`])
//!
//! - [`ItemDef`]: the data an `Item` instance carries after the static
//!   initialisers of `Block` and `Item` have run: the fields `Item.java`
//!   declares (Item.java:114-121) plus, in [`ItemKind`], the fields its
//!   subclass declares.
//! - [`item`] / [`ITEMS`]: `Item.itemsList`, an immutable table built at
//!   compile time by `table.rs`, which transcribes every row of Item.java's
//!   static initialiser (Item.java:273-381) and the `Item.itemsList` writes in
//!   Block.java's (Block.java:689-702, the `ItemBlock` family).
//! - [`ItemKind`]: one variant per `Item` subclass the initialisers
//!   instantiate (counting `Item` itself as [`ItemKind::Plain`]), carrying
//!   that subclass's instance fields.
//! - The data-only methods (`getIconFromDamage`, `getStrVsBlock`,
//!   `getItemNameIS`, `canHarvestBlock`, …) are methods on [`ItemDef`] that
//!   `match` on the kind. Item *behaviour* (`onItemUse`, `onItemRightClick`,
//!   `hitEntity`, `onBlockDestroyed`, `saddleEntity`, `onUpdate`,
//!   `onCreated`) is WP-2.6 and not here.
//! - [`ItemStack`] ([`stack`]): the value type with its NBT form.
//! - [`EnumToolMaterial`] ([`tool_material`]).
//! - [`block_texture`]: `Block.getBlockTextureFromSide(AndMetadata)`, which
//!   the `ItemBlock` icons are computed from.
//!
//! # Ids
//!
//! `Item.itemsList` has 32000 slots (Item.java:7). Ids 0–255 are blocks
//! (an `ItemBlock` or subclass for every non-null `Block.blocksList` entry),
//! 256–359 the items of Item.java's initialiser (`256 + constructor id`), and
//! 2256/2257 the two records. Every other id is null.
//!
//! # Runtime state the table does not hold
//!
//! Two inputs of the icon lookups are mutable singleton fields in vanilla:
//! `Block.leaves.blockIndexInTexture` (rewritten by `BlockLeaves.setGraphicsLevel`,
//! BlockLeaves.java:185-188, which the client calls for the fancy-graphics
//! option) and the piston base's bounds (`setBlockBoundsBasedOnState`). The
//! table and [`ItemDef::get_icon_from_damage`] use their values after static
//! initialisation (leaves 52, full bounds), which is what the oracle sees and
//! what every `ItemBlock` constructor saw.

pub mod block_texture;
pub mod stack;
mod table;
pub mod tool_material;

pub use stack::ItemStack;
pub use table::{ITEMS, RECORDS};
pub use tool_material::EnumToolMaterial;

use crate::block::{BlockDef, Material};

/// A Java exception a data method can throw on some inputs (out-of-range
/// damage values index name tables). Returned instead of a value so callers
/// can reproduce exactly where vanilla would crash.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JavaException {
    /// `ArrayIndexOutOfBoundsException`.
    ArrayIndexOutOfBounds,
    /// `ClassCastException` (an NBT tag of the wrong type).
    ClassCast,
}

impl JavaException {
    /// The Java class's simple name.
    pub const fn simple_name(self) -> &'static str {
        match self {
            JavaException::ArrayIndexOutOfBounds => "ArrayIndexOutOfBoundsException",
            JavaException::ClassCast => "ClassCastException",
        }
    }
}

/// `ItemTool`'s instance fields (ItemTool.java:4-7), shared by the spade,
/// pickaxe and axe.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToolData {
    /// `blocksEffectiveAgainst`: the subclass's static array, as block ids.
    pub blocks_effective_against: &'static [i32],
    /// `efficiencyOnProperMaterial`.
    pub efficiency_on_proper_material: f32,
    /// `damageVsEntity` (`base + toolMaterial.getDamageVsEntity()`).
    pub damage_vs_entity: i32,
    /// `toolMaterial`.
    pub tool_material: EnumToolMaterial,
}

/// `ItemFood`'s instance fields (ItemFood.java:4-5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FoodData {
    /// `healAmount`.
    pub heal_amount: i32,
    /// `isWolfsFavoriteMeat`.
    pub is_wolfs_favorite_meat: bool,
}

/// One variant per `Item` subclass the static initialisers instantiate,
/// named after the class without its `Item` prefix; fields are that class's
/// instance fields (for subclasses of `ItemTool`, `ItemFood` and `ItemBlock`,
/// the parent's fields, which is where those classes keep theirs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ItemKind {
    /// `Item` itself.
    Plain,
    /// `ItemSpade extends ItemTool`.
    Spade(ToolData),
    /// `ItemPickaxe extends ItemTool`.
    Pickaxe(ToolData),
    /// `ItemAxe extends ItemTool`.
    Axe(ToolData),
    FlintAndSteel,
    /// `ItemFood`.
    Food(FoodData),
    Bow,
    Coal,
    Sword {
        /// `weaponDamage` (ItemSword.java:4).
        weapon_damage: i32,
    },
    /// `ItemSoup extends ItemFood`.
    Soup(FoodData),
    Hoe,
    Seeds {
        /// `blockType` (ItemSeeds.java:4): the block id placed.
        block_type: i32,
    },
    Armor {
        /// `armorLevel` (ItemArmor.java:6): the material tier, 0–3.
        armor_level: i32,
        /// `armorType` (ItemArmor.java:7): 0 helmet, 1 chest, 2 legs, 3 boots.
        armor_type: i32,
        /// `damageReduceAmount` (ItemArmor.java:8).
        damage_reduce_amount: i32,
        /// `renderIndex` (ItemArmor.java:9): the armour texture set.
        render_index: i32,
    },
    Painting,
    Sign,
    Door {
        /// `doorMaterial` (ItemDoor.java:4): `Material.wood` or `Material.iron`.
        door_material: Material,
    },
    Bucket {
        /// `isFull` (ItemBucket.java:4): the fluid block id placed (0 empty,
        /// -1 milk).
        is_full: i32,
    },
    Minecart {
        /// `minecartType` (ItemMinecart.java:4).
        minecart_type: i32,
    },
    Saddle,
    Redstone,
    Snowball,
    Boat,
    Reed {
        /// `spawnID` (ItemReed.java:4): the block id placed.
        spawn_id: i32,
    },
    Egg,
    FishingRod,
    Dye,
    Bed,
    /// `ItemCookie extends ItemFood`.
    Cookie(FoodData),
    /// `ItemMap extends ItemMapBase`.
    Map,
    Shears,
    Record {
        /// `recordName` (ItemRecord.java:4).
        record_name: &'static str,
    },
    /// `ItemBlock`.
    Block {
        /// `ItemBlock.blockID` (ItemBlock.java:4).
        block_id: i32,
    },
    /// `ItemCloth extends ItemBlock`.
    Cloth {
        /// `ItemBlock.blockID`.
        block_id: i32,
    },
    /// `ItemLog extends ItemBlock`.
    Log {
        /// `ItemBlock.blockID`.
        block_id: i32,
    },
    /// `ItemSlab extends ItemBlock`.
    Slab {
        /// `ItemBlock.blockID`.
        block_id: i32,
    },
    /// `ItemSapling extends ItemBlock`.
    Sapling {
        /// `ItemBlock.blockID`.
        block_id: i32,
    },
    /// `ItemLeaves extends ItemBlock`.
    Leaves {
        /// `ItemBlock.blockID`.
        block_id: i32,
    },
    /// `ItemPiston extends ItemBlock`.
    Piston {
        /// `ItemBlock.blockID`.
        block_id: i32,
    },
}

impl ItemKind {
    /// The Java class chain from the concrete class up to `Item`, as simple
    /// names (diagnostics and the golden dump's per-class field groups).
    pub const fn java_class_chain(self) -> &'static [&'static str] {
        use ItemKind as K;
        match self {
            K::Plain => &["Item"],
            K::Spade(_) => &["ItemSpade", "ItemTool", "Item"],
            K::Pickaxe(_) => &["ItemPickaxe", "ItemTool", "Item"],
            K::Axe(_) => &["ItemAxe", "ItemTool", "Item"],
            K::FlintAndSteel => &["ItemFlintAndSteel", "Item"],
            K::Food(_) => &["ItemFood", "Item"],
            K::Bow => &["ItemBow", "Item"],
            K::Coal => &["ItemCoal", "Item"],
            K::Sword { .. } => &["ItemSword", "Item"],
            K::Soup(_) => &["ItemSoup", "ItemFood", "Item"],
            K::Hoe => &["ItemHoe", "Item"],
            K::Seeds { .. } => &["ItemSeeds", "Item"],
            K::Armor { .. } => &["ItemArmor", "Item"],
            K::Painting => &["ItemPainting", "Item"],
            K::Sign => &["ItemSign", "Item"],
            K::Door { .. } => &["ItemDoor", "Item"],
            K::Bucket { .. } => &["ItemBucket", "Item"],
            K::Minecart { .. } => &["ItemMinecart", "Item"],
            K::Saddle => &["ItemSaddle", "Item"],
            K::Redstone => &["ItemRedstone", "Item"],
            K::Snowball => &["ItemSnowball", "Item"],
            K::Boat => &["ItemBoat", "Item"],
            K::Reed { .. } => &["ItemReed", "Item"],
            K::Egg => &["ItemEgg", "Item"],
            K::FishingRod => &["ItemFishingRod", "Item"],
            K::Dye => &["ItemDye", "Item"],
            K::Bed => &["ItemBed", "Item"],
            K::Cookie(_) => &["ItemCookie", "ItemFood", "Item"],
            K::Map => &["ItemMap", "ItemMapBase", "Item"],
            K::Shears => &["ItemShears", "Item"],
            K::Record { .. } => &["ItemRecord", "Item"],
            K::Block { .. } => &["ItemBlock", "Item"],
            K::Cloth { .. } => &["ItemCloth", "ItemBlock", "Item"],
            K::Log { .. } => &["ItemLog", "ItemBlock", "Item"],
            K::Slab { .. } => &["ItemSlab", "ItemBlock", "Item"],
            K::Sapling { .. } => &["ItemSapling", "ItemBlock", "Item"],
            K::Leaves { .. } => &["ItemLeaves", "ItemBlock", "Item"],
            K::Piston { .. } => &["ItemPiston", "ItemBlock", "Item"],
        }
    }

    /// The concrete Java class's simple name.
    pub const fn java_class(self) -> &'static str {
        self.java_class_chain()[0]
    }

    /// `ItemTool`'s fields, for the spade, pickaxe and axe.
    pub const fn tool(&self) -> Option<&ToolData> {
        match self {
            ItemKind::Spade(t) | ItemKind::Pickaxe(t) | ItemKind::Axe(t) => Some(t),
            _ => None,
        }
    }

    /// `ItemFood`'s fields, for food, soup and the cookie.
    pub const fn food(&self) -> Option<&FoodData> {
        match self {
            ItemKind::Food(f) | ItemKind::Soup(f) | ItemKind::Cookie(f) => Some(f),
            _ => None,
        }
    }

    /// `ItemBlock.blockID`, for `ItemBlock` and its subclasses.
    pub const fn item_block_id(&self) -> Option<i32> {
        match *self {
            ItemKind::Block { block_id }
            | ItemKind::Cloth { block_id }
            | ItemKind::Log { block_id }
            | ItemKind::Slab { block_id }
            | ItemKind::Sapling { block_id }
            | ItemKind::Leaves { block_id }
            | ItemKind::Piston { block_id } => Some(block_id),
            _ => None,
        }
    }
}

/// An `Item` instance's data after the static initialisers (module docs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ItemDef {
    /// `shiftedIndex`: the item id (`256 + constructor id`).
    pub shifted_index: i32,
    /// The instantiated subclass and its fields.
    pub kind: ItemKind,
    /// `maxStackSize` (64 unless set).
    pub max_stack_size: i32,
    /// `maxDamage` (private in Java; `getMaxDamage()`).
    pub max_damage: i32,
    /// `iconIndex`.
    pub icon_index: i32,
    /// `bFull3D`.
    pub b_full_3d: bool,
    /// `hasSubtypes`.
    pub has_subtypes: bool,
    /// `containerItem` (private in Java), as the container's
    /// `shiftedIndex`.
    pub container_item: Option<i32>,
    /// The `setItemName` argument; Java's `itemName` is `"item." + this`
    /// (Item.java:229-232). `None` where vanilla never names the item (the
    /// piston `ItemBlock`s and every plain `ItemBlock`: their names come from
    /// the block).
    pub item_name: Option<&'static str>,
}

// Block ids the data methods compare against.
// src: Block.java:593-688
const BLOCK_WOOD: i32 = 17; // Block.wood (the log)
const BLOCK_LEAVES: i32 = 18;
const BLOCK_ORE_LAPIS: i32 = 21;
const BLOCK_LAPIS: i32 = 22;
const BLOCK_WEB: i32 = 30;
const BLOCK_CLOTH: i32 = 35;
const BLOCK_GOLD: i32 = 41;
const BLOCK_STEEL: i32 = 42;
const BLOCK_STAIR_SINGLE: i32 = 44;
const BLOCK_SAPLING: i32 = 6;
const BLOCK_ORE_GOLD: i32 = 14;
const BLOCK_ORE_IRON: i32 = 15;
const BLOCK_OBSIDIAN: i32 = 49;
const BLOCK_ORE_DIAMOND: i32 = 56;
const BLOCK_DIAMOND: i32 = 57;
const BLOCK_ORE_REDSTONE: i32 = 73;
const BLOCK_ORE_REDSTONE_GLOWING: i32 = 74;
const BLOCK_SNOW: i32 = 78;
const BLOCK_SNOW_BLOCK: i32 = 80;

/// `ItemDye.dyeColorNames`.
// src: ItemDye.java:4
pub const DYE_COLOR_NAMES: [&str; 16] = [
    "black",
    "red",
    "green",
    "brown",
    "blue",
    "purple",
    "cyan",
    "silver",
    "gray",
    "pink",
    "lime",
    "yellow",
    "lightBlue",
    "magenta",
    "orange",
    "white",
];

/// `ItemDye.dyeColors` (used by recipes and sheep; data only here).
// src: ItemDye.java:5
pub const DYE_COLORS: [i32; 16] = [
    1973019, 11743532, 3887386, 5320730, 2437522, 8073150, 2651799, 2651799, 4408131, 14188952,
    4312372, 14602026, 6719955, 12801229, 15435844, 15790320,
];

/// `BlockStep.field_22037_a`: the slab type names. Name kept; see
/// `docs/mappings.md`.
// src: BlockStep.java:6
pub const BLOCK_STEP_FIELD_22037_A: [&str; 4] = ["stone", "sand", "wood", "cobble"];

/// `BlockCloth.getBlockFromDye` (also `getDyeFromBlock`): `~v & 15`.
// src: BlockCloth.java:21-27
pub const fn block_cloth_get_block_from_dye(v: i32) -> i32 {
    !v & 15
}

/// `ColorizerFoliage.getFoliageColorPine/Birch/Basic`.
// src: ColorizerFoliage.java:17-27
const FOLIAGE_COLOR_PINE: i32 = 6396257;
const FOLIAGE_COLOR_BIRCH: i32 = 8431445;
const FOLIAGE_COLOR_BASIC: i32 = 4764952;

/// `array[i]` with Java's bounds check.
fn java_index<T: Copy>(array: &[T], i: i32) -> Result<T, JavaException> {
    usize::try_from(i)
        .ok()
        .and_then(|i| array.get(i).copied())
        .ok_or(JavaException::ArrayIndexOutOfBounds)
}

/// Java's `String + String` where the left side may be null (`"null"`).
fn java_concat(left: Option<String>, right: &str) -> String {
    format!("{}{}", left.as_deref().unwrap_or("null"), right)
}

impl ItemDef {
    /// The block an `ItemBlock` places; panics for other kinds and absent
    /// blocks (never the case for table rows).
    fn placed_block(&self) -> &'static BlockDef {
        let id = self.kind.item_block_id().expect("ItemBlock");
        crate::block::block(id).expect("ItemBlock of an existing block")
    }

    /// `getIconFromDamage(damage)`. The leaves case reads
    /// `Block.leaves.blockIndexInTexture` at its static-init value (module
    /// docs).
    // src: Item.java:147-149, ItemDye.java:13-15, ItemLog.java:10-12,
    // src: ItemSapling.java:14-16, ItemCloth.java:10-12, ItemSlab.java:10-12, ItemLeaves.java:14-16
    pub fn get_icon_from_damage(&self, damage: i32) -> i32 {
        use block_texture::get_block_texture_from_side_and_metadata as tex;
        let checked = |r: Result<i32, JavaException>| r.expect("texture lookup in range");
        match self.kind {
            ItemKind::Dye => self.icon_index + damage % 8 * 16 + damage / 8,
            ItemKind::Log { .. } => checked(tex(BLOCK_WOOD, 2, damage)),
            ItemKind::Sapling { .. } => checked(tex(BLOCK_SAPLING, 0, damage)),
            ItemKind::Cloth { .. } => {
                checked(tex(BLOCK_CLOTH, 2, block_cloth_get_block_from_dye(damage)))
            }
            ItemKind::Slab { .. } => checked(tex(BLOCK_STAIR_SINGLE, 2, damage)),
            ItemKind::Leaves { .. } => checked(tex(BLOCK_LEAVES, 0, damage)),
            _ => self.icon_index,
        }
    }

    /// `getIconIndex(stack)`: `getIconFromDamage(stack.getItemDamage())`.
    // src: Item.java:151-153
    pub fn get_icon_index(&self, stack: &ItemStack) -> i32 {
        self.get_icon_from_damage(stack.get_item_damage())
    }

    /// `getStrVsBlock(stack, block)`: the dig speed multiplier (the stack is
    /// unused by every override).
    // src: Item.java:159-161, ItemTool.java:19-27, ItemSword.java:13-15, ItemShears.java:22-28
    pub fn get_str_vs_block(&self, block: &BlockDef) -> f32 {
        let id = block.block_id;
        match self.kind {
            ItemKind::Spade(t) | ItemKind::Pickaxe(t) | ItemKind::Axe(t) => {
                // Identity comparison against the singletons in Java; ids are
                // equivalent.
                if t.blocks_effective_against.contains(&id) {
                    t.efficiency_on_proper_material
                } else {
                    1.0
                }
            }
            ItemKind::Sword { .. } => {
                if id == BLOCK_WEB {
                    15.0
                } else {
                    1.5
                }
            }
            ItemKind::Shears => {
                if id != BLOCK_WEB && id != BLOCK_LEAVES {
                    if id == BLOCK_CLOTH { 5.0 } else { 1.0 }
                } else {
                    15.0
                }
            }
            _ => 1.0,
        }
    }

    /// `getItemStackLimit()`.
    // src: Item.java:167-169
    pub const fn get_item_stack_limit(&self) -> i32 {
        self.max_stack_size
    }

    /// `getPlacedBlockMetadata(damage)`: the block metadata an `ItemBlock`
    /// places for a stack of this damage.
    // src: Item.java:171-173, ItemPiston.java:8-10, ItemLog.java:14-16, ItemSapling.java:10-12,
    // src: ItemCloth.java:14-16, ItemSlab.java:14-16, ItemLeaves.java:10-12
    pub const fn get_placed_block_metadata(&self, damage: i32) -> i32 {
        match self.kind {
            ItemKind::Piston { .. } => 7,
            ItemKind::Log { .. }
            | ItemKind::Sapling { .. }
            | ItemKind::Cloth { .. }
            | ItemKind::Slab { .. } => damage,
            ItemKind::Leaves { .. } => damage | 8,
            _ => 0,
        }
    }

    /// `getHasSubtypes()`.
    // src: Item.java:175-177
    pub const fn get_has_subtypes(&self) -> bool {
        self.has_subtypes
    }

    /// `getMaxDamage()`.
    // src: Item.java:184-186
    pub const fn get_max_damage(&self) -> i32 {
        self.max_damage
    }

    /// `isDamagable()`: `maxDamage > 0 && !hasSubtypes`.
    // src: Item.java:193-195
    pub const fn is_damagable(&self) -> bool {
        self.max_damage > 0 && !self.has_subtypes
    }

    /// `getDamageVsEntity(entity)` (the entity is unused by every override).
    // src: Item.java:205-207, ItemTool.java:39-41, ItemSword.java:27-29
    pub const fn get_damage_vs_entity(&self) -> i32 {
        match self.kind {
            ItemKind::Spade(t) | ItemKind::Pickaxe(t) | ItemKind::Axe(t) => t.damage_vs_entity,
            ItemKind::Sword { weapon_damage } => weapon_damage,
            _ => 1,
        }
    }

    /// `canHarvestBlock(block)`: whether this tool makes the block drop
    /// (consulted for materials that are not `isHarvestable`).
    // src: Item.java:209-211, ItemSword.java:35-37, ItemSpade.java:10-16,
    // src: ItemPickaxe.java:10-38, ItemShears.java:18-20
    // The pickaxe arms keep Java's one-branch-per-block-pair shape.
    #[allow(clippy::if_same_then_else)]
    pub fn can_harvest_block(&self, block: &BlockDef) -> bool {
        let id = block.block_id;
        match self.kind {
            ItemKind::Sword { .. } | ItemKind::Shears => id == BLOCK_WEB,
            ItemKind::Spade(_) => id == BLOCK_SNOW || id == BLOCK_SNOW_BLOCK,
            ItemKind::Pickaxe(t) => {
                let level = t.tool_material.get_harvest_level();
                if id == BLOCK_OBSIDIAN {
                    level == 3
                } else if id == BLOCK_DIAMOND || id == BLOCK_ORE_DIAMOND {
                    level >= 2
                } else if id == BLOCK_GOLD || id == BLOCK_ORE_GOLD {
                    level >= 2
                } else if id == BLOCK_STEEL || id == BLOCK_ORE_IRON {
                    level >= 1
                } else if id == BLOCK_LAPIS || id == BLOCK_ORE_LAPIS {
                    level >= 1
                } else if id == BLOCK_ORE_REDSTONE || id == BLOCK_ORE_REDSTONE_GLOWING {
                    level >= 2
                } else {
                    matches!(block.block_material, Material::Rock | Material::Iron)
                }
            }
            _ => false,
        }
    }

    /// `isFull3D()`: rendered as a held tool.
    // src: Item.java:221-223, ItemTool.java:43-45, ItemSword.java:31-33,
    // src: ItemHoe.java:28-30, ItemFishingRod.java:10-12
    pub const fn is_full_3d(&self) -> bool {
        match self.kind {
            ItemKind::Spade(_)
            | ItemKind::Pickaxe(_)
            | ItemKind::Axe(_)
            | ItemKind::Sword { .. }
            | ItemKind::Hoe
            | ItemKind::FishingRod => true,
            _ => self.b_full_3d,
        }
    }

    /// `shouldRotateAroundWhenRendering()`.
    // src: Item.java:225-227, ItemFishingRod.java:14-16
    pub const fn should_rotate_around_when_rendering(&self) -> bool {
        matches!(self.kind, ItemKind::FishingRod)
    }

    /// `getItemName()`: `itemName`, or for the `ItemBlock` family the block's
    /// `getBlockName()`. `None` is Java's null.
    // src: Item.java:234-236, ItemBlock.java:64-66
    pub fn get_item_name(&self) -> Option<String> {
        if self.kind.item_block_id().is_some() {
            self.placed_block().get_block_name()
        } else {
            self.item_name.map(|n| format!("item.{n}"))
        }
    }

    /// `getItemNameIS(stack)`: the translation key for a stack of this item.
    /// Damage values outside the name tables throw in Java (slabs above 3,
    /// dyes above 15, negative values).
    // src: Item.java:238-240, ItemCoal.java:10-12, ItemCloth.java:18-20, ItemSlab.java:18-20,
    // src: ItemDye.java:17-19, ItemBlock.java:60-62
    pub fn get_item_name_is(&self, stack: &ItemStack) -> Result<Option<String>, JavaException> {
        let damage = stack.get_item_damage();
        Ok(match self.kind {
            ItemKind::Coal => Some(
                if damage == 1 {
                    "item.charcoal"
                } else {
                    "item.coal"
                }
                .to_string(),
            ),
            ItemKind::Cloth { .. } => Some(java_concat(
                self.get_item_name(),
                &format!(
                    ".{}",
                    java_index(&DYE_COLOR_NAMES, block_cloth_get_block_from_dye(damage))?
                ),
            )),
            ItemKind::Slab { .. } => Some(java_concat(
                self.get_item_name(),
                &format!(".{}", java_index(&BLOCK_STEP_FIELD_22037_A, damage)?),
            )),
            ItemKind::Dye => Some(java_concat(
                self.item_name.map(|n| format!("item.{n}")),
                &format!(".{}", java_index(&DYE_COLOR_NAMES, damage)?),
            )),
            _ => self.get_item_name(),
        })
    }

    /// `getContainerItem()`: what stays in the crafting grid (the empty
    /// bucket for the full ones).
    // src: Item.java:251-253
    pub fn get_container_item(&self) -> Option<&'static ItemDef> {
        self.container_item.and_then(item)
    }

    /// `hasContainerItem()`.
    // src: Item.java:255-257
    pub const fn has_container_item(&self) -> bool {
        self.container_item.is_some()
    }

    /// `getColorFromDamage(damage)`: the render tint (RGB).
    // src: Item.java:263-265, ItemLeaves.java:18-24
    pub const fn get_color_from_damage(&self, damage: i32) -> i32 {
        match self.kind {
            ItemKind::Leaves { .. } => {
                if (damage & 1) == 1 {
                    FOLIAGE_COLOR_PINE
                } else if (damage & 2) == 2 {
                    FOLIAGE_COLOR_BIRCH
                } else {
                    FOLIAGE_COLOR_BASIC
                }
            }
            _ => 16777215,
        }
    }

    /// `ItemFood.getHealAmount()`; `None` for non-food items (Java would
    /// not compile the call).
    // src: ItemFood.java:20-22
    pub const fn get_heal_amount(&self) -> Option<i32> {
        match self.kind.food() {
            Some(f) => Some(f.heal_amount),
            None => None,
        }
    }

    /// `ItemFood.getIsWolfsFavoriteMeat()`; `None` for non-food items.
    // src: ItemFood.java:24-26
    pub const fn get_is_wolfs_favorite_meat(&self) -> Option<bool> {
        match self.kind.food() {
            Some(f) => Some(f.is_wolfs_favorite_meat),
            None => None,
        }
    }
}

/// `Item.itemsList.length` (Item.java:7).
pub const ITEMS_LIST_LEN: i32 = 32000;

/// `Item.itemsList[id]`; `None` for null entries and for ids outside
/// `0..32000` (where Java would throw).
pub fn item(id: i32) -> Option<&'static ItemDef> {
    if (0..ITEMS.len() as i32).contains(&id) {
        ITEMS[id as usize].as_ref()
    } else if (table::RECORD_BASE..table::RECORD_BASE + RECORDS.len() as i32).contains(&id) {
        Some(&RECORDS[(id - table::RECORD_BASE) as usize])
    } else {
        None
    }
}

/// Every non-null `Item.itemsList` entry in id order.
pub fn all_items() -> impl Iterator<Item = &'static ItemDef> {
    ITEMS.iter().flatten().chain(RECORDS.iter())
}

/// The highest id with a non-null `Item.itemsList` entry (2257, `recordCat`).
pub fn max_item_id() -> i32 {
    table::RECORD_BASE + RECORDS.len() as i32 - 1
}
