//! `EnumToolMaterial` (EnumToolMaterial.java): the five tool tiers.

/// `EnumToolMaterial`, constants in declaration (= ordinal) order.
// src: EnumToolMaterial.java:3-8
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EnumToolMaterial {
    Wood,
    Stone,
    Iron,
    /// Diamond tools (`EMERALD` is the MCP name).
    Emerald,
    Gold,
}

impl EnumToolMaterial {
    /// Every constant in ordinal order (`values()`).
    pub const VALUES: [EnumToolMaterial; 5] = [
        EnumToolMaterial::Wood,
        EnumToolMaterial::Stone,
        EnumToolMaterial::Iron,
        EnumToolMaterial::Emerald,
        EnumToolMaterial::Gold,
    ];

    /// `name()`.
    pub const fn name(self) -> &'static str {
        match self {
            EnumToolMaterial::Wood => "WOOD",
            EnumToolMaterial::Stone => "STONE",
            EnumToolMaterial::Iron => "IRON",
            EnumToolMaterial::Emerald => "EMERALD",
            EnumToolMaterial::Gold => "GOLD",
        }
    }

    /// `ordinal()`.
    pub const fn ordinal(self) -> i32 {
        self as i32
    }

    /// The constructor arguments `(harvestLevel, maxUses,
    /// efficiencyOnProperMaterial, damageVsEntity)`.
    // src: EnumToolMaterial.java:4-8, :15-20
    const fn args(self) -> (i32, i32, f32, i32) {
        match self {
            EnumToolMaterial::Wood => (0, 59, 2.0, 0),
            EnumToolMaterial::Stone => (1, 131, 4.0, 1),
            EnumToolMaterial::Iron => (2, 250, 6.0, 2),
            EnumToolMaterial::Emerald => (3, 1561, 8.0, 3),
            EnumToolMaterial::Gold => (0, 32, 12.0, 0),
        }
    }

    /// `getHarvestLevel()` (`harvestLevel`).
    // src: EnumToolMaterial.java:34-36
    pub const fn get_harvest_level(self) -> i32 {
        self.args().0
    }

    /// `getMaxUses()` (`maxUses`).
    // src: EnumToolMaterial.java:22-24
    pub const fn get_max_uses(self) -> i32 {
        self.args().1
    }

    /// `getEfficiencyOnProperMaterial()` (`efficiencyOnProperMaterial`).
    // src: EnumToolMaterial.java:26-28
    pub const fn get_efficiency_on_proper_material(self) -> f32 {
        self.args().2
    }

    /// `getDamageVsEntity()` (`damageVsEntity`).
    // src: EnumToolMaterial.java:30-32
    pub const fn get_damage_vs_entity(self) -> i32 {
        self.args().3
    }
}
