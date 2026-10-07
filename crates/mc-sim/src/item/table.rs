//! Item.java's static initialiser (Item.java:273-381) and the
//! `Item.itemsList` writes in Block.java's (Block.java:689-702), row by row.
//!
//! Each row is one `public static Item` field: the subclass constructor it
//! calls (the `const fn`s below, each transcribing that class's constructor
//! and the constructors it chains to), then the setter chain in source order.
//!
//! Not in this table: `StatList.initStats()` (Item.java:380; WP-2.7).
//! `Item.itemRand` (Item.java:6) is world state: [`crate::world::World::item_rand`].

use super::*;
use crate::block::BLOCKS;

// ---- Item's constructor and setters -------------------------------------------

impl ItemDef {
    /// `Item(int id)`: `shiftedIndex = 256 + id` and the field initialisers.
    // src: Item.java:114-130
    pub(crate) const fn new(id: i32, kind: ItemKind) -> Self {
        ItemDef {
            shifted_index: 256 + id,
            kind,
            max_stack_size: 64,
            max_damage: 0,
            icon_index: 0,
            b_full_3d: false,
            has_subtypes: false,
            container_item: None,
            item_name: None,
        }
    }

    /// `setIconIndex`.
    // src: Item.java:132-135
    pub(crate) const fn set_icon_index(mut self, v: i32) -> Self {
        self.icon_index = v;
        self
    }

    /// `setMaxStackSize`, and a subclass constructor's `this.maxStackSize = v`.
    // src: Item.java:137-140
    pub(crate) const fn set_max_stack_size(mut self, v: i32) -> Self {
        self.max_stack_size = v;
        self
    }

    /// `setIconCoord(x, y)`: `iconIndex = x + y * 16`.
    // src: Item.java:142-145
    pub(crate) const fn set_icon_coord(mut self, x: i32, y: i32) -> Self {
        self.icon_index = x + y * 16;
        self
    }

    /// `setHasSubtypes`.
    // src: Item.java:179-182
    pub(crate) const fn set_has_subtypes(mut self, v: bool) -> Self {
        self.has_subtypes = v;
        self
    }

    /// `setMaxDamage`.
    // src: Item.java:188-191
    pub(crate) const fn set_max_damage(mut self, v: i32) -> Self {
        self.max_damage = v;
        self
    }

    /// `setFull3D`.
    // src: Item.java:216-219
    pub(crate) const fn set_full_3d(mut self) -> Self {
        self.b_full_3d = true;
        self
    }

    /// `setItemName`.
    // src: Item.java:229-232
    pub(crate) const fn set_item_name(mut self, name: &'static str) -> Self {
        self.item_name = Some(name);
        self
    }

    /// `setContainerItem`: only for items that do not stack.
    // src: Item.java:242-249
    pub(crate) const fn set_container_item(mut self, container: &ItemDef) -> Self {
        if self.max_stack_size > 1 {
            panic!("Max stack size must be 1 for items with crafting results");
        }
        self.container_item = Some(container.shifted_index);
        self
    }
}

// ---- subclass constructors ------------------------------------------------------

/// `ItemTool(id, base, material, blocks)`: the fields it sets.
// src: ItemTool.java:9-17
const fn tool_data(
    base_damage: i32,
    material: EnumToolMaterial,
    blocks: &'static [i32],
) -> ToolData {
    ToolData {
        blocks_effective_against: blocks,
        efficiency_on_proper_material: material.get_efficiency_on_proper_material(),
        damage_vs_entity: base_damage + material.get_damage_vs_entity(),
        tool_material: material,
    }
}

/// `ItemTool(id, base, material, blocks)` for a `kind` built from
/// [`tool_data`].
// src: ItemTool.java:9-17
const fn item_tool(id: i32, kind: ItemKind, material: EnumToolMaterial) -> ItemDef {
    ItemDef::new(id, kind)
        .set_max_stack_size(1)
        .set_max_damage(material.get_max_uses())
}

/// `ItemSpade.blocksEffectiveAgainst`: grass, dirt, sand, gravel, snow,
/// blockSnow, blockClay, tilledField.
// src: ItemSpade.java:18-20
const SPADE_BLOCKS: [i32; 8] = [2, 3, 12, 13, 78, 80, 82, 60];

/// `ItemPickaxe.blocksEffectiveAgainst`: cobblestone, stairDouble,
/// stairSingle, stone, sandStone, cobblestoneMossy, oreIron, blockSteel,
/// oreCoal, blockGold, oreGold, oreDiamond, blockDiamond, ice, netherrack,
/// oreLapis, blockLapis.
// src: ItemPickaxe.java:40-42
const PICKAXE_BLOCKS: [i32; 17] = [
    4, 43, 44, 1, 24, 48, 15, 42, 16, 41, 14, 56, 57, 79, 87, 21, 22,
];

/// `ItemAxe.blocksEffectiveAgainst`: planks, bookShelf, wood, chest.
// src: ItemAxe.java:10-12
const AXE_BLOCKS: [i32; 4] = [5, 47, 17, 54];

/// `ItemSpade(id, material)`: base damage 1.
// src: ItemSpade.java:6-8
const fn item_spade(id: i32, m: EnumToolMaterial) -> ItemDef {
    item_tool(id, ItemKind::Spade(tool_data(1, m, &SPADE_BLOCKS)), m)
}

/// `ItemPickaxe(id, material)`: base damage 2.
// src: ItemPickaxe.java:6-8
const fn item_pickaxe(id: i32, m: EnumToolMaterial) -> ItemDef {
    item_tool(id, ItemKind::Pickaxe(tool_data(2, m, &PICKAXE_BLOCKS)), m)
}

/// `ItemAxe(id, material)`: base damage 3.
// src: ItemAxe.java:6-8
const fn item_axe(id: i32, m: EnumToolMaterial) -> ItemDef {
    item_tool(id, ItemKind::Axe(tool_data(3, m, &AXE_BLOCKS)), m)
}

/// `ItemSword(id, material)`: `weaponDamage = 4 + material.getDamageVsEntity() * 2`.
// src: ItemSword.java:6-11
const fn item_sword(id: i32, m: EnumToolMaterial) -> ItemDef {
    ItemDef::new(
        id,
        ItemKind::Sword {
            weapon_damage: 4 + m.get_damage_vs_entity() * 2,
        },
    )
    .set_max_stack_size(1)
    .set_max_damage(m.get_max_uses())
}

/// `ItemHoe(id, material)`.
// src: ItemHoe.java:4-8
const fn item_hoe(id: i32, m: EnumToolMaterial) -> ItemDef {
    ItemDef::new(id, ItemKind::Hoe)
        .set_max_stack_size(1)
        .set_max_damage(m.get_max_uses())
}

/// `ItemFood(id, heal, wolfMeat)`: the fields it sets.
// src: ItemFood.java:7-12
const fn food_data(heal: i32, wolf: bool) -> FoodData {
    FoodData {
        heal_amount: heal,
        is_wolfs_favorite_meat: wolf,
    }
}

/// `ItemFood(id, heal, wolfMeat)` for a `kind` built from [`food_data`]
/// (also the super call of `ItemSoup` and `ItemCookie`).
// src: ItemFood.java:7-12
const fn item_food_as(id: i32, kind: ItemKind) -> ItemDef {
    ItemDef::new(id, kind).set_max_stack_size(1)
}

const fn item_food(id: i32, heal: i32, wolf: bool) -> ItemDef {
    item_food_as(id, ItemKind::Food(food_data(heal, wolf)))
}

/// `ItemSoup(id, heal)`: `super(id, heal, false)`.
// src: ItemSoup.java:4-6
const fn item_soup(id: i32, heal: i32) -> ItemDef {
    item_food_as(id, ItemKind::Soup(food_data(heal, false)))
}

/// `ItemCookie(id, heal, wolfMeat, maxStack)`.
// src: ItemCookie.java:4-7
const fn item_cookie(id: i32, heal: i32, wolf: bool, max_stack: i32) -> ItemDef {
    item_food_as(id, ItemKind::Cookie(food_data(heal, wolf))).set_max_stack_size(max_stack)
}

/// `ItemArmor.damageReduceAmountArray` / `maxDamageArray`.
// src: ItemArmor.java:4-5
const ARMOR_DAMAGE_REDUCE_AMOUNT: [i32; 4] = [3, 8, 6, 3];
const ARMOR_MAX_DAMAGE: [i32; 4] = [11, 16, 15, 13];

/// `ItemArmor(id, armorLevel, renderIndex, armorType)`: max damage
/// `maxDamageArray[type] * 3 << level`.
// src: ItemArmor.java:11-19
const fn item_armor(id: i32, level: i32, render_index: i32, armor_type: i32) -> ItemDef {
    ItemDef::new(
        id,
        ItemKind::Armor {
            armor_level: level,
            armor_type,
            damage_reduce_amount: ARMOR_DAMAGE_REDUCE_AMOUNT[armor_type as usize],
            render_index,
        },
    )
    .set_max_damage((ARMOR_MAX_DAMAGE[armor_type as usize] * 3) << level)
    .set_max_stack_size(1)
}

/// Constructors that only call `super(id)` and set `maxStackSize` (when
/// `stack` is not 64): ItemBow, ItemSaddle, ItemSign, ItemBoat (1),
/// ItemSnowball, ItemEgg (16).
// src: ItemBow.java:4-7, ItemSaddle.java:4-7, ItemSign.java:4-7, ItemBoat.java:4-7,
// src: ItemSnowball.java:4-7, ItemEgg.java:4-7
const fn item_stack_of(id: i32, kind: ItemKind, stack: i32) -> ItemDef {
    ItemDef::new(id, kind).set_max_stack_size(stack)
}

/// `ItemFlintAndSteel(id)`.
// src: ItemFlintAndSteel.java:4-8
const fn item_flint_and_steel(id: i32) -> ItemDef {
    ItemDef::new(id, ItemKind::FlintAndSteel)
        .set_max_stack_size(1)
        .set_max_damage(64)
}

/// `ItemCoal(id)` and `ItemDye(id)`: `setHasSubtypes(true); setMaxDamage(0)`.
// src: ItemCoal.java:4-8, ItemDye.java:7-11
const fn item_subtypes(id: i32, kind: ItemKind) -> ItemDef {
    ItemDef::new(id, kind)
        .set_has_subtypes(true)
        .set_max_damage(0)
}

/// `ItemDoor(id, material)`.
// src: ItemDoor.java:6-10
const fn item_door(id: i32, door_material: Material) -> ItemDef {
    ItemDef::new(id, ItemKind::Door { door_material }).set_max_stack_size(1)
}

/// `ItemBucket(id, isFull)`.
// src: ItemBucket.java:6-10
const fn item_bucket(id: i32, is_full: i32) -> ItemDef {
    ItemDef::new(id, ItemKind::Bucket { is_full }).set_max_stack_size(1)
}

/// `ItemMinecart(id, type)`.
// src: ItemMinecart.java:6-10
const fn item_minecart(id: i32, minecart_type: i32) -> ItemDef {
    ItemDef::new(id, ItemKind::Minecart { minecart_type }).set_max_stack_size(1)
}

/// `ItemFishingRod(id)`.
// src: ItemFishingRod.java:4-8
const fn item_fishing_rod(id: i32) -> ItemDef {
    ItemDef::new(id, ItemKind::FishingRod)
        .set_max_damage(64)
        .set_max_stack_size(1)
}

/// `ItemMap(id)` (`ItemMapBase(id)` only calls super).
// src: ItemMap.java:4-7, ItemMapBase.java:4-6
const fn item_map(id: i32) -> ItemDef {
    ItemDef::new(id, ItemKind::Map).set_max_stack_size(1)
}

/// `ItemShears(id)`.
// src: ItemShears.java:4-8
const fn item_shears(id: i32) -> ItemDef {
    ItemDef::new(id, ItemKind::Shears)
        .set_max_stack_size(1)
        .set_max_damage(238)
}

/// `ItemRecord(id, name)`.
// src: ItemRecord.java:6-10
const fn item_record(id: i32, record_name: &'static str) -> ItemDef {
    ItemDef::new(id, ItemKind::Record { record_name }).set_max_stack_size(1)
}

/// `ItemReed(id, block)`: `spawnID = block.blockID`.
// src: ItemReed.java:6-9
const fn item_reed(id: i32, block_id: i32) -> ItemDef {
    ItemDef::new(id, ItemKind::Reed { spawn_id: block_id })
}

/// `ItemBlock(id)` (`id` = block id − 256): `blockID = id + 256`, icon
/// `Block.blocksList[blockID].getBlockTextureFromSide(2)`.
// src: ItemBlock.java:6-10
/// `kind` carries `blockID = id + 256` (built by the caller).
const fn item_block(id: i32, kind: ItemKind) -> ItemDef {
    let block_id = id + 256;
    ItemDef::new(id, kind).set_icon_index(crate::block::block_texture::get_block_texture_from_side(
        block_id, 2,
    ))
}

/// `ItemCloth`, `ItemLog`, `ItemSlab`, `ItemSapling`, `ItemLeaves`:
/// `super(id); setMaxDamage(0); setHasSubtypes(true)`.
// src: ItemCloth.java:4-8, ItemLog.java:4-8, ItemSlab.java:4-8, ItemSapling.java:4-8, ItemLeaves.java:4-8
const fn item_block_subtypes(id: i32, kind: ItemKind) -> ItemDef {
    item_block(id, kind)
        .set_max_damage(0)
        .set_has_subtypes(true)
}

// ---- the rows -------------------------------------------------------------------

use EnumToolMaterial::{Emerald, Gold, Iron, Stone, Wood};

// Rows needed by later rows' constructors.
// src: Item.java:343
const BUCKET_EMPTY: ItemDef = item_bucket(69, 0)
    .set_icon_coord(10, 4)
    .set_item_name("bucket");

/// Item.java:274-377, in source order (ids 256..=359).
// src: Item.java:274-377
const ITEM_ROWS: [ItemDef; 104] = [
    item_spade(0, Iron)
        .set_icon_coord(2, 5)
        .set_item_name("shovelIron"),
    item_pickaxe(1, Iron)
        .set_icon_coord(2, 6)
        .set_item_name("pickaxeIron"),
    item_axe(2, Iron)
        .set_icon_coord(2, 7)
        .set_item_name("hatchetIron"),
    item_flint_and_steel(3)
        .set_icon_coord(5, 0)
        .set_item_name("flintAndSteel"),
    item_food(4, 4, false)
        .set_icon_coord(10, 0)
        .set_item_name("apple"),
    item_stack_of(5, ItemKind::Bow, 1)
        .set_icon_coord(5, 1)
        .set_item_name("bow"),
    ItemDef::new(6, ItemKind::Plain)
        .set_icon_coord(5, 2)
        .set_item_name("arrow"),
    item_subtypes(7, ItemKind::Coal)
        .set_icon_coord(7, 0)
        .set_item_name("coal"),
    ItemDef::new(8, ItemKind::Plain)
        .set_icon_coord(7, 3)
        .set_item_name("emerald"),
    ItemDef::new(9, ItemKind::Plain)
        .set_icon_coord(7, 1)
        .set_item_name("ingotIron"),
    ItemDef::new(10, ItemKind::Plain)
        .set_icon_coord(7, 2)
        .set_item_name("ingotGold"),
    item_sword(11, Iron)
        .set_icon_coord(2, 4)
        .set_item_name("swordIron"),
    item_sword(12, Wood)
        .set_icon_coord(0, 4)
        .set_item_name("swordWood"),
    item_spade(13, Wood)
        .set_icon_coord(0, 5)
        .set_item_name("shovelWood"),
    item_pickaxe(14, Wood)
        .set_icon_coord(0, 6)
        .set_item_name("pickaxeWood"),
    item_axe(15, Wood)
        .set_icon_coord(0, 7)
        .set_item_name("hatchetWood"),
    item_sword(16, Stone)
        .set_icon_coord(1, 4)
        .set_item_name("swordStone"),
    item_spade(17, Stone)
        .set_icon_coord(1, 5)
        .set_item_name("shovelStone"),
    item_pickaxe(18, Stone)
        .set_icon_coord(1, 6)
        .set_item_name("pickaxeStone"),
    item_axe(19, Stone)
        .set_icon_coord(1, 7)
        .set_item_name("hatchetStone"),
    item_sword(20, Emerald)
        .set_icon_coord(3, 4)
        .set_item_name("swordDiamond"),
    item_spade(21, Emerald)
        .set_icon_coord(3, 5)
        .set_item_name("shovelDiamond"),
    item_pickaxe(22, Emerald)
        .set_icon_coord(3, 6)
        .set_item_name("pickaxeDiamond"),
    item_axe(23, Emerald)
        .set_icon_coord(3, 7)
        .set_item_name("hatchetDiamond"),
    ItemDef::new(24, ItemKind::Plain)
        .set_icon_coord(5, 3)
        .set_full_3d()
        .set_item_name("stick"),
    ItemDef::new(25, ItemKind::Plain)
        .set_icon_coord(7, 4)
        .set_item_name("bowl"),
    item_soup(26, 10)
        .set_icon_coord(8, 4)
        .set_item_name("mushroomStew"),
    item_sword(27, Gold)
        .set_icon_coord(4, 4)
        .set_item_name("swordGold"),
    item_spade(28, Gold)
        .set_icon_coord(4, 5)
        .set_item_name("shovelGold"),
    item_pickaxe(29, Gold)
        .set_icon_coord(4, 6)
        .set_item_name("pickaxeGold"),
    item_axe(30, Gold)
        .set_icon_coord(4, 7)
        .set_item_name("hatchetGold"),
    ItemDef::new(31, ItemKind::Plain)
        .set_icon_coord(8, 0)
        .set_item_name("string"),
    ItemDef::new(32, ItemKind::Plain)
        .set_icon_coord(8, 1)
        .set_item_name("feather"),
    ItemDef::new(33, ItemKind::Plain)
        .set_icon_coord(8, 2)
        .set_item_name("sulphur"),
    item_hoe(34, Wood)
        .set_icon_coord(0, 8)
        .set_item_name("hoeWood"),
    item_hoe(35, Stone)
        .set_icon_coord(1, 8)
        .set_item_name("hoeStone"),
    item_hoe(36, Iron)
        .set_icon_coord(2, 8)
        .set_item_name("hoeIron"),
    item_hoe(37, Emerald)
        .set_icon_coord(3, 8)
        .set_item_name("hoeDiamond"),
    item_hoe(38, Gold)
        .set_icon_coord(4, 8)
        .set_item_name("hoeGold"),
    // `new ItemSeeds(39, Block.crops.blockID)`: crops is id 59 (Block.java:651).
    // src: ItemSeeds.java:6-9
    ItemDef::new(39, ItemKind::Seeds { block_type: 59 })
        .set_icon_coord(9, 0)
        .set_item_name("seeds"),
    ItemDef::new(40, ItemKind::Plain)
        .set_icon_coord(9, 1)
        .set_item_name("wheat"),
    item_food(41, 5, false)
        .set_icon_coord(9, 2)
        .set_item_name("bread"),
    item_armor(42, 0, 0, 0)
        .set_icon_coord(0, 0)
        .set_item_name("helmetCloth"),
    item_armor(43, 0, 0, 1)
        .set_icon_coord(0, 1)
        .set_item_name("chestplateCloth"),
    item_armor(44, 0, 0, 2)
        .set_icon_coord(0, 2)
        .set_item_name("leggingsCloth"),
    item_armor(45, 0, 0, 3)
        .set_icon_coord(0, 3)
        .set_item_name("bootsCloth"),
    item_armor(46, 1, 1, 0)
        .set_icon_coord(1, 0)
        .set_item_name("helmetChain"),
    item_armor(47, 1, 1, 1)
        .set_icon_coord(1, 1)
        .set_item_name("chestplateChain"),
    item_armor(48, 1, 1, 2)
        .set_icon_coord(1, 2)
        .set_item_name("leggingsChain"),
    item_armor(49, 1, 1, 3)
        .set_icon_coord(1, 3)
        .set_item_name("bootsChain"),
    item_armor(50, 2, 2, 0)
        .set_icon_coord(2, 0)
        .set_item_name("helmetIron"),
    item_armor(51, 2, 2, 1)
        .set_icon_coord(2, 1)
        .set_item_name("chestplateIron"),
    item_armor(52, 2, 2, 2)
        .set_icon_coord(2, 2)
        .set_item_name("leggingsIron"),
    item_armor(53, 2, 2, 3)
        .set_icon_coord(2, 3)
        .set_item_name("bootsIron"),
    item_armor(54, 3, 3, 0)
        .set_icon_coord(3, 0)
        .set_item_name("helmetDiamond"),
    item_armor(55, 3, 3, 1)
        .set_icon_coord(3, 1)
        .set_item_name("chestplateDiamond"),
    item_armor(56, 3, 3, 2)
        .set_icon_coord(3, 2)
        .set_item_name("leggingsDiamond"),
    item_armor(57, 3, 3, 3)
        .set_icon_coord(3, 3)
        .set_item_name("bootsDiamond"),
    // Gold armour has armorLevel 1 (chain's durability tier) and renderIndex 4.
    item_armor(58, 1, 4, 0)
        .set_icon_coord(4, 0)
        .set_item_name("helmetGold"),
    item_armor(59, 1, 4, 1)
        .set_icon_coord(4, 1)
        .set_item_name("chestplateGold"),
    item_armor(60, 1, 4, 2)
        .set_icon_coord(4, 2)
        .set_item_name("leggingsGold"),
    item_armor(61, 1, 4, 3)
        .set_icon_coord(4, 3)
        .set_item_name("bootsGold"),
    ItemDef::new(62, ItemKind::Plain)
        .set_icon_coord(6, 0)
        .set_item_name("flint"),
    item_food(63, 3, true)
        .set_icon_coord(7, 5)
        .set_item_name("porkchopRaw"),
    item_food(64, 8, true)
        .set_icon_coord(8, 5)
        .set_item_name("porkchopCooked"),
    ItemDef::new(65, ItemKind::Painting)
        .set_icon_coord(10, 1)
        .set_item_name("painting"),
    item_food(66, 42, false)
        .set_icon_coord(11, 0)
        .set_item_name("appleGold"),
    item_stack_of(67, ItemKind::Sign, 1)
        .set_icon_coord(10, 2)
        .set_item_name("sign"),
    item_door(68, Material::Wood)
        .set_icon_coord(11, 2)
        .set_item_name("doorWood"),
    BUCKET_EMPTY,
    // `Block.waterMoving.blockID` = 8, `Block.lavaMoving.blockID` = 10 (Block.java:600, :602).
    item_bucket(70, 8)
        .set_icon_coord(11, 4)
        .set_item_name("bucketWater")
        .set_container_item(&BUCKET_EMPTY),
    item_bucket(71, 10)
        .set_icon_coord(12, 4)
        .set_item_name("bucketLava")
        .set_container_item(&BUCKET_EMPTY),
    item_minecart(72, 0)
        .set_icon_coord(7, 8)
        .set_item_name("minecart"),
    item_stack_of(73, ItemKind::Saddle, 1)
        .set_icon_coord(8, 6)
        .set_item_name("saddle"),
    item_door(74, Material::Iron)
        .set_icon_coord(12, 2)
        .set_item_name("doorIron"),
    ItemDef::new(75, ItemKind::Redstone)
        .set_icon_coord(8, 3)
        .set_item_name("redstone"),
    item_stack_of(76, ItemKind::Snowball, 16)
        .set_icon_coord(14, 0)
        .set_item_name("snowball"),
    item_stack_of(77, ItemKind::Boat, 1)
        .set_icon_coord(8, 8)
        .set_item_name("boat"),
    ItemDef::new(78, ItemKind::Plain)
        .set_icon_coord(7, 6)
        .set_item_name("leather"),
    item_bucket(79, -1)
        .set_icon_coord(13, 4)
        .set_item_name("milk")
        .set_container_item(&BUCKET_EMPTY),
    ItemDef::new(80, ItemKind::Plain)
        .set_icon_coord(6, 1)
        .set_item_name("brick"),
    ItemDef::new(81, ItemKind::Plain)
        .set_icon_coord(9, 3)
        .set_item_name("clay"),
    // `Block.reed` = 83 (Block.java:675).
    item_reed(82, 83)
        .set_icon_coord(11, 1)
        .set_item_name("reeds"),
    ItemDef::new(83, ItemKind::Plain)
        .set_icon_coord(10, 3)
        .set_item_name("paper"),
    ItemDef::new(84, ItemKind::Plain)
        .set_icon_coord(11, 3)
        .set_item_name("book"),
    ItemDef::new(85, ItemKind::Plain)
        .set_icon_coord(14, 1)
        .set_item_name("slimeball"),
    item_minecart(86, 1)
        .set_icon_coord(7, 9)
        .set_item_name("minecartChest"),
    item_minecart(87, 2)
        .set_icon_coord(7, 10)
        .set_item_name("minecartFurnace"),
    item_stack_of(88, ItemKind::Egg, 16)
        .set_icon_coord(12, 0)
        .set_item_name("egg"),
    ItemDef::new(89, ItemKind::Plain)
        .set_icon_coord(6, 3)
        .set_item_name("compass"),
    item_fishing_rod(90)
        .set_icon_coord(5, 4)
        .set_item_name("fishingRod"),
    ItemDef::new(91, ItemKind::Plain)
        .set_icon_coord(6, 4)
        .set_item_name("clock"),
    ItemDef::new(92, ItemKind::Plain)
        .set_icon_coord(9, 4)
        .set_item_name("yellowDust"),
    item_food(93, 2, false)
        .set_icon_coord(9, 5)
        .set_item_name("fishRaw"),
    item_food(94, 5, false)
        .set_icon_coord(10, 5)
        .set_item_name("fishCooked"),
    item_subtypes(95, ItemKind::Dye)
        .set_icon_coord(14, 4)
        .set_item_name("dyePowder"),
    ItemDef::new(96, ItemKind::Plain)
        .set_icon_coord(12, 1)
        .set_item_name("bone")
        .set_full_3d(),
    ItemDef::new(97, ItemKind::Plain)
        .set_icon_coord(13, 0)
        .set_item_name("sugar")
        .set_full_3d(),
    // `Block.cake` = 92 (Block.java:684).
    item_reed(98, 92)
        .set_max_stack_size(1)
        .set_icon_coord(13, 1)
        .set_item_name("cake"),
    ItemDef::new(99, ItemKind::Bed)
        .set_max_stack_size(1)
        .set_icon_coord(13, 2)
        .set_item_name("bed"),
    // `Block.redstoneRepeaterIdle` = 93 (Block.java:685).
    item_reed(100, 93)
        .set_icon_coord(6, 5)
        .set_item_name("diode"),
    item_cookie(101, 1, false, 8)
        .set_icon_coord(12, 5)
        .set_item_name("cookie"),
    item_map(102).set_icon_coord(12, 3).set_item_name("map"),
    item_shears(103)
        .set_icon_coord(13, 5)
        .set_item_name("shears"),
];

/// The id of the first record (`256 + 2000`).
// src: Item.java:378
pub(crate) const RECORD_BASE: i32 = 2256;

/// `record13` and `recordCat` (ids 2256, 2257).
// src: Item.java:378-379
pub static RECORDS: [ItemDef; 2] = [
    item_record(2000, "13")
        .set_icon_coord(0, 15)
        .set_item_name("record"),
    item_record(2001, "cat")
        .set_icon_coord(1, 15)
        .set_item_name("record"),
];

// Block ids that get an ItemBlock subclass instead of a plain ItemBlock.
// src: Block.java:598, :609, :610, :621, :625, :627, :636
const CLOTH: i32 = 35;
const WOOD: i32 = 17;
const STAIR_SINGLE: i32 = 44;
const SAPLING: i32 = 6;
const LEAVES: i32 = 18;
const PISTON_BASE: i32 = 33;
const PISTON_STICKY_BASE: i32 = 29;

/// `Item.itemsList[0..360]`: the `ItemBlock`s (Block.java:689-702) and
/// Item.java's rows.
const fn build() -> [Option<ItemDef>; 360] {
    let mut t: [Option<ItemDef>; 360] = [None; 360];
    // src: Block.java:689-695 (each also passes through Item(int)'s
    // itemsList write, Item.java:129).
    t[CLOTH as usize] = Some(
        item_block_subtypes(CLOTH - 256, ItemKind::Cloth { block_id: CLOTH })
            .set_item_name("cloth"),
    );
    t[WOOD as usize] = Some(
        item_block_subtypes(WOOD - 256, ItemKind::Log { block_id: WOOD }).set_item_name("log"),
    );
    t[STAIR_SINGLE as usize] = Some(
        item_block_subtypes(
            STAIR_SINGLE - 256,
            ItemKind::Slab {
                block_id: STAIR_SINGLE,
            },
        )
        .set_item_name("stoneSlab"),
    );
    t[SAPLING as usize] = Some(
        item_block_subtypes(SAPLING - 256, ItemKind::Sapling { block_id: SAPLING })
            .set_item_name("sapling"),
    );
    t[LEAVES as usize] = Some(
        item_block_subtypes(LEAVES - 256, ItemKind::Leaves { block_id: LEAVES })
            .set_item_name("leaves"),
    );
    // src: ItemPiston.java:4-6
    t[PISTON_BASE as usize] = Some(item_block(
        PISTON_BASE - 256,
        ItemKind::Piston {
            block_id: PISTON_BASE,
        },
    ));
    t[PISTON_STICKY_BASE as usize] = Some(item_block(
        PISTON_STICKY_BASE - 256,
        ItemKind::Piston {
            block_id: PISTON_STICKY_BASE,
        },
    ));
    // src: Block.java:697-702
    let mut id = 0;
    while id < 256 {
        if BLOCKS[id].is_some() && t[id].is_none() {
            // Block.java:700 blocksList[var0].initializeBlock() is block
            // state (BlockFire burn rates), see block/table.rs; not item data.
            t[id] = Some(item_block(
                id as i32 - 256,
                ItemKind::Block {
                    block_id: id as i32,
                },
            ));
        }
        id += 1;
    }
    let mut i = 0;
    while i < ITEM_ROWS.len() {
        let row = ITEM_ROWS[i];
        t[row.shifted_index as usize] = Some(row);
        i += 1;
    }
    t
}

/// `Item.itemsList[0..360]` (ids 2256 and 2257 are [`RECORDS`]).
pub static ITEMS: [Option<ItemDef>; 360] = build();
