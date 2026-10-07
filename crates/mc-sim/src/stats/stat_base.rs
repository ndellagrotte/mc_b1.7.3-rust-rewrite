//! `StatBase`, `StatBasic`, `StatCrafting` (StatBase.java, StatBasic.java,
//! StatCrafting.java), the three `IStatType` formats (StatTypeSimple.java,
//! StatTypeTime.java, StatTypeDistance.java) and the `StatCollector`
//! translation lookup (StatCollector.java, StringTranslate.java).
//!
//! A stat's name is a translated string in Java, built once from the jar's
//! `lang/` files. The port keeps the translation keys instead
//! ([`StatName`]): the lang files are the user's (PLAN_v4 D8), and the
//! client translates with [`StatName::translate`].

use super::achievement::Achievement;

/// The `IStatType` instances (StatBase.java:14-17): how a stat's value is
/// shown in the statistics screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatType {
    /// `StatTypeSimple` (`StatBase.field_27087_i`, the default): the
    /// integer with US grouping.
    Simple,
    /// `StatTypeTime` (`StatBase.timeStatType`): ticks as seconds, minutes,
    /// hours, days or years.
    Time,
    /// `StatTypeDistance` (`StatBase.distanceStatType`): centimetres as cm,
    /// m or km.
    Distance,
}

impl StatType {
    /// The Java class's simple name (the `stat_table` fixture's `type`).
    pub const fn class_name(self) -> &'static str {
        match self {
            StatType::Simple => "StatTypeSimple",
            StatType::Time => "StatTypeTime",
            StatType::Distance => "StatTypeDistance",
        }
    }

    /// `format(value)`.
    ///
    /// - `StatTypeSimple`: `NumberFormat.getIntegerInstance(Locale.US)`
    ///   (StatBase.java:69): groups of three digits separated by `,`.
    /// - `StatTypeTime`: `value / 20` seconds; the first unit (years of 365
    ///   days, days, hours, minutes) whose amount exceeds 0.5 is shown with
    ///   two decimals (`DecimalFormat("########0.00")`, StatBase.java:71),
    ///   otherwise the seconds as Java's `double + String` (`"0.05 s"`,
    ///   `"1.0 s"`).
    /// - `StatTypeDistance`: `value / 100` metres; km above 0.5 km, m above
    ///   0.5 m (two decimals), otherwise the centimetres as an integer.
    ///
    /// `DecimalFormat` takes the default locale's symbols when `StatBase`
    /// initialises; the port uses en_US's (`.`), as the oracle does.
    // src: StatTypeSimple.java:4-6, StatTypeTime.java:4-19, StatTypeDistance.java:4-12
    pub fn format(self, var1: i32) -> String {
        match self {
            StatType::Simple => integer_format_us(var1 as i64),
            StatType::Time => {
                let var2 = var1 as f64 / 20.0;
                let var4 = var2 / 60.0;
                let var6 = var4 / 60.0;
                let var8 = var6 / 24.0;
                let var10 = var8 / 365.0;
                if var10 > 0.5 {
                    format!("{} y", decimal_format_2(var10))
                } else if var8 > 0.5 {
                    format!("{} d", decimal_format_2(var8))
                } else if var6 > 0.5 {
                    format!("{} h", decimal_format_2(var6))
                } else if var4 > 0.5 {
                    format!("{} m", decimal_format_2(var4))
                } else {
                    format!("{} s", java_double_to_string(var2))
                }
            }
            StatType::Distance => {
                let var3 = var1 as f64 / 100.0;
                let var5 = var3 / 1000.0;
                if var5 > 0.5 {
                    format!("{} km", decimal_format_2(var5))
                } else if var3 > 0.5 {
                    format!("{} m", decimal_format_2(var3))
                } else {
                    format!("{var1} cm")
                }
            }
        }
    }
}

/// `NumberFormat.getIntegerInstance(Locale.US).format(long)`: decimal digits
/// in groups of three separated by `,`, a leading `-` for negatives.
fn integer_format_us(v: i64) -> String {
    let digits = v.unsigned_abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    if v < 0 {
        out.push('-');
    }
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// `new DecimalFormat("########0.00").format(double)` (StatBase.java:71):
/// at least one integer digit, exactly two decimals, no grouping, rounded
/// half-even on the exact binary value. Rust's `{:.2}` rounds the exact
/// value half-to-even too.
fn decimal_format_2(v: f64) -> String {
    format!("{v:.2}")
}

/// Java's `Double.toString(d)` (JDK 19+: the shortest decimal that rounds
/// back to `d`) for finite values: plain notation with at least one
/// decimal for `1e-3 <= |d| < 1e7`, otherwise `d.dddE±n`.
pub(crate) fn java_double_to_string(d: f64) -> String {
    if d == 0.0 {
        return if d.is_sign_negative() { "-0.0" } else { "0.0" }.to_string();
    }
    // Rust's `{:e}` is the shortest round-trip digit string: "1.073741824e8".
    let sci = format!("{d:e}");
    let (mantissa, exp) = sci.split_once('e').expect("{:e} has an exponent");
    let exp: i32 = exp.parse().expect("{:e} exponent is an integer");
    let (sign, mantissa) = match mantissa.strip_prefix('-') {
        Some(m) => ("-", m),
        None => ("", mantissa),
    };
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let a = d.abs();
    if (1e-3..1e7).contains(&a) {
        // Plain: the decimal point sits after digit `exp + 1`.
        let point = exp + 1;
        if point <= 0 {
            format!("{sign}0.{}{digits}", "0".repeat((-point) as usize))
        } else {
            let point = point as usize;
            if digits.len() <= point {
                format!("{sign}{digits}{}.0", "0".repeat(point - digits.len()))
            } else {
                format!("{sign}{}.{}", &digits[..point], &digits[point..])
            }
        }
    } else {
        let frac = if digits.len() > 1 { &digits[1..] } else { "0" };
        format!("{sign}{}.{frac}E{exp}", &digits[..1])
    }
}

/// A stat's `statName` as translation keys: `StatCollector.translateToLocal
/// (key)` for the plain stats and achievements, `translateToLocalFormatted
/// (key, translateToLocal(arg))` for the per-block and per-item stats,
/// whose argument is the block's or item's own name key
/// (`Block.translateBlockName()`, `Item.getStatName()`).
// src: StatList.java:86, :100, :117, :136, Achievement.java:21
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatName {
    /// The key, e.g. `"stat.jump"`, `"stat.mineBlock"`,
    /// `"achievement.mineWood"`.
    pub key: String,
    /// The format argument's key, e.g. `"tile.stone.name"`, for the
    /// formatted names.
    pub arg: Option<String>,
}

impl StatName {
    /// A plain key.
    pub fn plain(key: impl Into<String>) -> StatName {
        StatName {
            key: key.into(),
            arg: None,
        }
    }

    /// A formatted key with its argument's key.
    pub fn formatted(key: &str, arg: String) -> StatName {
        StatName {
            key: key.to_string(),
            arg: Some(arg),
        }
    }

    /// The name under the `stat_table` fixture's identity translation
    /// (tools/oracle/java/harness/StatTable.java): the key, or
    /// `key(arg)` for a formatted one.
    pub fn key_form(&self) -> String {
        match &self.arg {
            None => self.key.clone(),
            Some(a) => format!("{}({a})", self.key),
        }
    }

    /// `statName` as Java builds it, given the lang table (`lookup(key)`:
    /// the `en_US.lang` / `stats_US.lang` entry). A missing key translates
    /// to itself (`Properties.getProperty(key, key)`, StringTranslate.java:24-26).
    /// `String.format` is applied as far as the vanilla stat texts use it:
    /// `%1$s` and `%s` take the argument.
    // src: StatCollector.java:6-12, StringTranslate.java:24-31
    pub fn translate(&self, lookup: &dyn Fn(&str) -> Option<String>) -> String {
        let t = |k: &str| lookup(k).unwrap_or_else(|| k.to_string());
        match &self.arg {
            None => t(&self.key),
            Some(a) => t(&self.key)
                .replacen("%1$s", &t(a), 1)
                .replacen("%s", &t(a), 1),
        }
    }
}

/// The class-specific part of a stat.
#[derive(Clone, Debug, PartialEq)]
pub enum StatKind {
    /// `StatBasic` (StatBasic.java): the general stats.
    Basic,
    /// `StatCrafting` (StatCrafting.java): a per-block or per-item stat;
    /// `item_id` is `field_25073_a` (`getItemID()`), the block or item id.
    Crafting {
        /// `field_25073_a`, returned by `getItemID()` (StatCrafting.java:4-13).
        item_id: i32,
    },
    /// `Achievement` (Achievement.java).
    Achievement(Achievement),
}

/// One `StatBase` instance.
#[derive(Clone, Debug, PartialEq)]
pub struct StatBase {
    /// `statId`: unique (`registerStat` throws on a duplicate,
    /// StatBase.java:36-37).
    pub stat_id: i32,
    /// `statName`, as keys ([`StatName`]).
    pub stat_name: StatName,
    /// `isIndependent`: set by `initIndependentStat()` /
    /// `Achievement.setIndependent()`.
    pub is_independent: bool,
    /// `field_26902_a` (private): the `IStatType`; `field_27087_i`
    /// (`StatTypeSimple`) unless the constructor names one.
    // src: StatBase.java:12, :19-28
    pub stat_type: StatType,
    /// The subclass.
    pub kind: StatKind,
}

// `statGuid` (StatBase.java:11, :41) is `AchievementMap.getGuid(statId)`,
// read from the jar's `/achievement/map.txt`; the port looks it up when the
// stats file needs it ([`super::AchievementMap`]).

impl StatBase {
    /// `new StatBasic(id, name, type)` / `new StatBasic(id, name)`
    /// (`StatTypeSimple`).
    // src: StatBasic.java:4-10, StatBase.java:19-28
    pub fn basic(stat_id: i32, key: &str, stat_type: StatType) -> StatBase {
        StatBase {
            stat_id,
            stat_name: StatName::plain(key),
            is_independent: false,
            stat_type,
            kind: StatKind::Basic,
        }
    }

    /// `new StatCrafting(id, name, itemId)`: `StatTypeSimple`.
    // src: StatCrafting.java:6-9
    pub fn crafting(stat_id: i32, stat_name: StatName, item_id: i32) -> StatBase {
        StatBase {
            stat_id,
            stat_name,
            is_independent: false,
            stat_type: StatType::Simple,
            kind: StatKind::Crafting { item_id },
        }
    }

    /// `initIndependentStat()` (and `Achievement.setIndependent()`, its
    /// bridge override).
    // src: StatBase.java:30-33, Achievement.java:45-48
    pub fn init_independent_stat(mut self) -> StatBase {
        self.is_independent = true;
        self
    }

    /// `isAchievement()`: false, true for `Achievement`.
    // src: StatBase.java:46-48, Achievement.java:61-63
    pub fn is_achievement(&self) -> bool {
        matches!(self.kind, StatKind::Achievement(_))
    }

    /// The achievement data, for an `Achievement`.
    pub fn achievement(&self) -> Option<&Achievement> {
        match &self.kind {
            StatKind::Achievement(a) => Some(a),
            _ => None,
        }
    }

    /// `StatCrafting.getItemID()`, for a `StatCrafting`.
    // src: StatCrafting.java:11-13
    pub fn get_item_id(&self) -> Option<i32> {
        match self.kind {
            StatKind::Crafting { item_id } => Some(item_id),
            _ => None,
        }
    }

    /// The Java class's simple name (the `stat_table` fixture's `class`).
    pub fn class_name(&self) -> &'static str {
        match self.kind {
            StatKind::Basic => "StatBasic",
            StatKind::Crafting { .. } => "StatCrafting",
            StatKind::Achievement(_) => "Achievement",
        }
    }

    /// `func_27084_a(value)`: the value in this stat's format.
    // src: StatBase.java:50-52
    pub fn func_27084_a(&self, var1: i32) -> String {
        self.stat_type.format(var1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_double_to_string_matches_java() {
        assert_eq!(java_double_to_string(0.0), "0.0");
        assert_eq!(java_double_to_string(0.05), "0.05");
        assert_eq!(java_double_to_string(1.0), "1.0");
        assert_eq!(java_double_to_string(-617.25), "-617.25");
        assert_eq!(java_double_to_string(-1.073741824e8), "-1.073741824E8");
        assert_eq!(java_double_to_string(1.0e7), "1.0E7");
        assert_eq!(java_double_to_string(9999999.0), "9999999.0");
        assert_eq!(java_double_to_string(0.001), "0.001");
        assert_eq!(java_double_to_string(0.0001), "1.0E-4");
        assert_eq!(java_double_to_string(123.0), "123.0");
    }

    #[test]
    fn integer_format_groups() {
        assert_eq!(integer_format_us(0), "0");
        assert_eq!(integer_format_us(999), "999");
        assert_eq!(integer_format_us(1000), "1,000");
        assert_eq!(integer_format_us(-2147483648), "-2,147,483,648");
    }

    #[test]
    fn translate_formats_the_argument() {
        let lang = |k: &str| match k {
            "stat.mineBlock" => Some("%1$s Mined".to_string()),
            "tile.stone.name" => Some("Stone".to_string()),
            _ => None,
        };
        let n = StatName::formatted("stat.mineBlock", "tile.stone.name".into());
        assert_eq!(n.translate(&lang), "Stone Mined");
        assert_eq!(n.key_form(), "stat.mineBlock(tile.stone.name)");
        assert_eq!(StatName::plain("stat.jump").translate(&lang), "stat.jump");
    }
}
