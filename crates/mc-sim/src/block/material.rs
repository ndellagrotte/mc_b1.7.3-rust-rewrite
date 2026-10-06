//! `Material` and `MapColor` (Material.java, MaterialLogic.java,
//! MaterialTransparent.java, MaterialLiquid.java, MaterialPortal.java,
//! MapColor.java).
//!
//! Vanilla compares materials by identity (`block.blockMaterial == Material.water`,
//! World.java:1444), so a material is an enum variant, not a bundle of flags: two
//! materials with identical properties (e.g. `sponge` and `ground`) stay distinct.
//! The properties are `const fn`s transcribed from the subclass overrides and the
//! static initialiser's setter chains (Material.java:109-138).
//!
//! Naming follows the Java tree, including its confusing MCP names: the field
//! `isOpaque` is set by `setIsTranslucent()` and read by `getIsTranslucent()`
//! (which returns `!isOpaque && getIsSolid()`, Material.java:87-89).

/// `MapColor` (MapColor.java:3-27): the map palette entry of a material.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapColor {
    /// `colorIndex`.
    pub color_index: i32,
    /// `colorValue` (0xRRGGBB).
    pub color_value: i32,
}

impl MapColor {
    // src: MapColor.java:5-18
    pub const AIR_COLOR: MapColor = MapColor {
        color_index: 0,
        color_value: 0,
    };
    pub const GRASS_COLOR: MapColor = MapColor {
        color_index: 1,
        color_value: 8368696,
    };
    pub const SAND_COLOR: MapColor = MapColor {
        color_index: 2,
        color_value: 16247203,
    };
    pub const CLOTH_COLOR: MapColor = MapColor {
        color_index: 3,
        color_value: 10987431,
    };
    pub const TNT_COLOR: MapColor = MapColor {
        color_index: 4,
        color_value: 16711680,
    };
    pub const ICE_COLOR: MapColor = MapColor {
        color_index: 5,
        color_value: 10526975,
    };
    pub const IRON_COLOR: MapColor = MapColor {
        color_index: 6,
        color_value: 10987431,
    };
    pub const FOLIAGE_COLOR: MapColor = MapColor {
        color_index: 7,
        color_value: 31744,
    };
    pub const SNOW_COLOR: MapColor = MapColor {
        color_index: 8,
        color_value: 16777215,
    };
    pub const CLAY_COLOR: MapColor = MapColor {
        color_index: 9,
        color_value: 10791096,
    };
    pub const DIRT_COLOR: MapColor = MapColor {
        color_index: 10,
        color_value: 12020271,
    };
    pub const STONE_COLOR: MapColor = MapColor {
        color_index: 11,
        color_value: 7368816,
    };
    pub const WATER_COLOR: MapColor = MapColor {
        color_index: 12,
        color_value: 4210943,
    };
    pub const WOOD_COLOR: MapColor = MapColor {
        color_index: 13,
        color_value: 6837042,
    };

    /// `mapColorArray` (MapColor.java:3): each constructor stores itself at
    /// its `colorIndex` (MapColor.java:22-26); slots 14 and 15 stay null.
    pub const MAP_COLOR_ARRAY: [Option<MapColor>; 16] = [
        Some(Self::AIR_COLOR),
        Some(Self::GRASS_COLOR),
        Some(Self::SAND_COLOR),
        Some(Self::CLOTH_COLOR),
        Some(Self::TNT_COLOR),
        Some(Self::ICE_COLOR),
        Some(Self::IRON_COLOR),
        Some(Self::FOLIAGE_COLOR),
        Some(Self::SNOW_COLOR),
        Some(Self::CLAY_COLOR),
        Some(Self::DIRT_COLOR),
        Some(Self::STONE_COLOR),
        Some(Self::WATER_COLOR),
        Some(Self::WOOD_COLOR),
        None,
        None,
    ];
}

/// One of the 28 `Material` singletons (Material.java:4-31), in declaration
/// order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Material {
    Air,
    GrassMaterial,
    Ground,
    Wood,
    Rock,
    Iron,
    Water,
    Lava,
    Leaves,
    Plants,
    Sponge,
    Cloth,
    Fire,
    Sand,
    Circuits,
    Glass,
    Tnt,
    Coral,
    Ice,
    Snow,
    BuiltSnow,
    Cactus,
    Clay,
    Pumpkin,
    Portal,
    CakeMaterial,
    Web,
    Piston,
}

/// Which `Material` class a singleton is an instance of; the subclasses only
/// override `isSolid`, `getCanBlockGrass`, `getIsSolid`, `getIsLiquid`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MaterialClass {
    /// `Material`.
    Plain,
    /// `MaterialLogic` (MaterialLogic.java:3-18).
    Logic,
    /// `MaterialTransparent` (MaterialTransparent.java:3-19).
    Transparent,
    /// `MaterialLiquid` (MaterialLiquid.java:3-19).
    Liquid,
    /// `MaterialPortal` (MaterialPortal.java:3-18).
    Portal,
}

/// The fields a material's construction chain sets (Material.java:32-37).
#[derive(Clone, Copy, Debug)]
struct MaterialProps {
    class: MaterialClass,
    map_color: MapColor,
    /// `canBurn` (`setBurning`).
    can_burn: bool,
    /// `groundCover` (`setIsGroundCover`; also set by the `MaterialTransparent`
    /// and `MaterialLiquid` constructors).
    ground_cover: bool,
    /// `isOpaque` (`setIsTranslucent`).
    is_opaque: bool,
    /// `canHarvest` (default true; `setNoHarvest`).
    can_harvest: bool,
    /// `mobilityFlag` (0; `setNoPushMobility` = 1, `setImmovableMobility` = 2;
    /// the `MaterialLiquid` constructor sets 1).
    mobility_flag: i32,
}

const fn props(class: MaterialClass, map_color: MapColor) -> MaterialProps {
    // Constructor side effects: MaterialTransparent.java:5-6 (groundCover),
    // MaterialLiquid.java:5-7 (groundCover, mobility 1).
    let (ground_cover, mobility_flag) = match class {
        MaterialClass::Transparent => (true, 0),
        MaterialClass::Liquid => (true, 1),
        _ => (false, 0),
    };
    MaterialProps {
        class,
        map_color,
        can_burn: false,
        ground_cover,
        is_opaque: false,
        can_harvest: true,
        mobility_flag,
    }
}

impl MaterialProps {
    const fn burning(mut self) -> Self {
        self.can_burn = true;
        self
    }
    const fn translucent(mut self) -> Self {
        self.is_opaque = true;
        self
    }
    const fn no_harvest(mut self) -> Self {
        self.can_harvest = false;
        self
    }
    const fn ground_cover(mut self) -> Self {
        self.ground_cover = true;
        self
    }
    const fn no_push(mut self) -> Self {
        self.mobility_flag = 1;
        self
    }
    const fn immovable(mut self) -> Self {
        self.mobility_flag = 2;
        self
    }
}

impl Material {
    /// Every singleton in declaration order (Material.java:4-31).
    pub const ALL: [Material; 28] = [
        Material::Air,
        Material::GrassMaterial,
        Material::Ground,
        Material::Wood,
        Material::Rock,
        Material::Iron,
        Material::Water,
        Material::Lava,
        Material::Leaves,
        Material::Plants,
        Material::Sponge,
        Material::Cloth,
        Material::Fire,
        Material::Sand,
        Material::Circuits,
        Material::Glass,
        Material::Tnt,
        Material::Coral,
        Material::Ice,
        Material::Snow,
        Material::BuiltSnow,
        Material::Cactus,
        Material::Clay,
        Material::Pumpkin,
        Material::Portal,
        Material::CakeMaterial,
        Material::Web,
        Material::Piston,
    ];

    /// The name of the `Material` static field holding this singleton.
    // src: Material.java:4-31
    pub const fn java_name(self) -> &'static str {
        match self {
            Material::Air => "air",
            Material::GrassMaterial => "grassMaterial",
            Material::Ground => "ground",
            Material::Wood => "wood",
            Material::Rock => "rock",
            Material::Iron => "iron",
            Material::Water => "water",
            Material::Lava => "lava",
            Material::Leaves => "leaves",
            Material::Plants => "plants",
            Material::Sponge => "sponge",
            Material::Cloth => "cloth",
            Material::Fire => "fire",
            Material::Sand => "sand",
            Material::Circuits => "circuits",
            Material::Glass => "glass",
            Material::Tnt => "tnt",
            Material::Coral => "coral",
            Material::Ice => "ice",
            Material::Snow => "snow",
            Material::BuiltSnow => "builtSnow",
            Material::Cactus => "cactus",
            Material::Clay => "clay",
            Material::Pumpkin => "pumpkin",
            Material::Portal => "portal",
            Material::CakeMaterial => "cakeMaterial",
            Material::Web => "web",
            Material::Piston => "piston",
        }
    }

    /// The Java class of the singleton (`Material` or one of its four
    /// subclasses), for traces.
    pub const fn java_class(self) -> &'static str {
        match self.props().class {
            MaterialClass::Plain => "Material",
            MaterialClass::Logic => "MaterialLogic",
            MaterialClass::Transparent => "MaterialTransparent",
            MaterialClass::Liquid => "MaterialLiquid",
            MaterialClass::Portal => "MaterialPortal",
        }
    }

    /// The private field `isOpaque` itself (set by `setIsTranslucent`; read
    /// only through [`Material::get_is_translucent`]).
    // src: Material.java:34, :59-62
    pub const fn is_opaque(self) -> bool {
        self.props().is_opaque
    }

    /// The static initialiser, one row per singleton.
    const fn props(self) -> MaterialProps {
        use MaterialClass as C;
        match self {
            // src: Material.java:110
            Material::Air => props(C::Transparent, MapColor::AIR_COLOR),
            // src: Material.java:111
            Material::GrassMaterial => props(C::Plain, MapColor::GRASS_COLOR),
            // src: Material.java:112
            Material::Ground => props(C::Plain, MapColor::DIRT_COLOR),
            // src: Material.java:113
            Material::Wood => props(C::Plain, MapColor::WOOD_COLOR).burning(),
            // src: Material.java:114
            Material::Rock => props(C::Plain, MapColor::STONE_COLOR).no_harvest(),
            // src: Material.java:115
            Material::Iron => props(C::Plain, MapColor::IRON_COLOR).no_harvest(),
            // src: Material.java:116
            Material::Water => props(C::Liquid, MapColor::WATER_COLOR).no_push(),
            // src: Material.java:117
            Material::Lava => props(C::Liquid, MapColor::TNT_COLOR).no_push(),
            // src: Material.java:118
            Material::Leaves => props(C::Plain, MapColor::FOLIAGE_COLOR)
                .burning()
                .translucent()
                .no_push(),
            // src: Material.java:119
            Material::Plants => props(C::Logic, MapColor::FOLIAGE_COLOR).no_push(),
            // src: Material.java:120
            Material::Sponge => props(C::Plain, MapColor::CLOTH_COLOR),
            // src: Material.java:121
            Material::Cloth => props(C::Plain, MapColor::CLOTH_COLOR).burning(),
            // src: Material.java:122
            Material::Fire => props(C::Transparent, MapColor::AIR_COLOR).no_push(),
            // src: Material.java:123
            Material::Sand => props(C::Plain, MapColor::SAND_COLOR),
            // src: Material.java:124
            Material::Circuits => props(C::Logic, MapColor::AIR_COLOR).no_push(),
            // src: Material.java:125
            Material::Glass => props(C::Plain, MapColor::AIR_COLOR).translucent(),
            // src: Material.java:126
            Material::Tnt => props(C::Plain, MapColor::TNT_COLOR).burning().translucent(),
            // src: Material.java:127
            Material::Coral => props(C::Plain, MapColor::FOLIAGE_COLOR).no_push(),
            // src: Material.java:128
            Material::Ice => props(C::Plain, MapColor::ICE_COLOR).translucent(),
            // src: Material.java:129
            Material::Snow => props(C::Logic, MapColor::SNOW_COLOR)
                .ground_cover()
                .translucent()
                .no_harvest()
                .no_push(),
            // src: Material.java:130
            Material::BuiltSnow => props(C::Plain, MapColor::SNOW_COLOR).no_harvest(),
            // src: Material.java:131
            Material::Cactus => props(C::Plain, MapColor::FOLIAGE_COLOR)
                .translucent()
                .no_push(),
            // src: Material.java:132
            Material::Clay => props(C::Plain, MapColor::CLAY_COLOR),
            // src: Material.java:133
            Material::Pumpkin => props(C::Plain, MapColor::FOLIAGE_COLOR).no_push(),
            // src: Material.java:134
            Material::Portal => props(C::Portal, MapColor::AIR_COLOR).immovable(),
            // src: Material.java:135
            Material::CakeMaterial => props(C::Plain, MapColor::AIR_COLOR).no_push(),
            // src: Material.java:136
            Material::Web => props(C::Plain, MapColor::CLOTH_COLOR)
                .no_harvest()
                .no_push(),
            // src: Material.java:137
            Material::Piston => props(C::Plain, MapColor::STONE_COLOR).immovable(),
        }
    }

    /// `materialMapColor`.
    pub const fn material_map_color(self) -> MapColor {
        self.props().map_color
    }

    /// `getIsLiquid()`: true only for `MaterialLiquid`.
    // src: Material.java:43-45, MaterialLiquid.java:9-11
    pub const fn get_is_liquid(self) -> bool {
        matches!(self.props().class, MaterialClass::Liquid)
    }

    /// `isSolid()`: false for the four subclasses.
    // src: Material.java:47-49, MaterialLogic.java:8, MaterialTransparent.java:9, MaterialLiquid.java:17, MaterialPortal.java:8
    pub const fn is_solid(self) -> bool {
        matches!(self.props().class, MaterialClass::Plain)
    }

    /// `getCanBlockGrass()`: false for `MaterialLogic`, `MaterialTransparent`,
    /// `MaterialPortal`; `MaterialLiquid` does not override it.
    // src: Material.java:51-53, MaterialLogic.java:12, MaterialTransparent.java:13, MaterialPortal.java:12
    pub const fn get_can_block_grass(self) -> bool {
        matches!(
            self.props().class,
            MaterialClass::Plain | MaterialClass::Liquid
        )
    }

    /// `getIsSolid()`: false for the four subclasses.
    // src: Material.java:55-57, MaterialLogic.java:16, MaterialTransparent.java:17, MaterialLiquid.java:13, MaterialPortal.java:16
    pub const fn get_is_solid(self) -> bool {
        matches!(self.props().class, MaterialClass::Plain)
    }

    /// `getBurning()`.
    // src: Material.java:74-76
    pub const fn get_burning(self) -> bool {
        self.props().can_burn
    }

    /// `getIsGroundCover()`.
    // src: Material.java:83-85
    pub const fn get_is_ground_cover(self) -> bool {
        self.props().ground_cover
    }

    /// `getIsTranslucent()`: `isOpaque ? false : getIsSolid()` (the MCP names
    /// are inverted; see the module docs).
    // src: Material.java:87-89
    pub const fn get_is_translucent(self) -> bool {
        if self.props().is_opaque {
            false
        } else {
            self.get_is_solid()
        }
    }

    /// `getIsHarvestable()`.
    // src: Material.java:91-93
    pub const fn get_is_harvestable(self) -> bool {
        self.props().can_harvest
    }

    /// `getMaterialMobility()`: 0 normal, 1 no push, 2 immovable.
    // src: Material.java:95-97
    pub const fn get_material_mobility(self) -> i32 {
        self.props().mobility_flag
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subclass_overrides() {
        assert!(Material::Water.get_is_liquid());
        assert!(!Material::Water.is_solid());
        // MaterialLiquid does not override getCanBlockGrass.
        assert!(Material::Water.get_can_block_grass());
        assert!(!Material::Air.get_can_block_grass());
        assert!(Material::Air.get_is_ground_cover());
        assert!(Material::Water.get_is_ground_cover());
        assert_eq!(Material::Water.get_material_mobility(), 1);
        assert_eq!(Material::Portal.get_material_mobility(), 2);
        // leaves: setIsTranslucent makes getIsTranslucent false.
        assert!(!Material::Leaves.get_is_translucent());
        assert!(Material::Rock.get_is_translucent());
        assert!(!Material::Rock.get_is_harvestable());
        assert!(Material::Snow.get_is_ground_cover());
        assert!(!Material::Snow.is_solid());
    }
}
