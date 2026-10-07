//! `Session` (Session.java): the player name the game runs as.
//!
//! Login and the session id are online services (D2, deferred): the id is
//! kept as given and never used. The username names the player
//! (`EntityPlayerSP`'s `username`, EntityPlayerSP.java:20) and the stats
//! file (`stats_<name>_unsent.dat`).

/// `Session`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session {
    /// `username`.
    // src: Session.java:7
    pub username: String,
    /// `sessionId` (unused offline).
    // src: Session.java:8
    pub session_id: String,
    /// `mpPassParameter` (multiplayer, D3).
    // src: Session.java:9
    pub mp_pass_parameter: Option<String>,
}

impl Session {
    /// `new Session(username, sessionId)`.
    // src: Session.java:11-14
    pub fn new(var1: &str, var2: &str) -> Self {
        Session {
            username: var1.to_string(),
            session_id: var2.to_string(),
            mp_pass_parameter: None,
        }
    }

    /// `Minecraft.main`'s offline session: `"Player" +
    /// System.currentTimeMillis() % 1000`, session id `"-"`.
    // src: Minecraft.java:1515-1528
    pub fn offline(now_millis: i64) -> Self {
        Session::new(&format!("Player{}", now_millis % 1000), "-")
    }
}

/// `Session.registeredBlocksList`: the blocks of the creative test
/// controller's palette, as block ids in list order.
// src: Session.java:16-46
pub const REGISTERED_BLOCKS_LIST: [i32; 28] = [
    1,  // stone
    4,  // cobblestone
    45, // brick
    3,  // dirt
    5,  // planks
    17, // wood
    18, // leaves
    50, // torchWood
    44, // stairSingle
    20, // glass
    48, // cobblestoneMossy
    6,  // sapling
    37, // plantYellow
    38, // plantRed
    39, // mushroomBrown
    40, // mushroomRed
    12, // sand
    13, // gravel
    19, // sponge
    35, // cloth
    16, // oreCoal
    15, // oreIron
    14, // oreGold
    42, // blockSteel
    41, // blockGold
    47, // bookShelf
    46, // tnt
    49, // obsidian
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_name() {
        assert_eq!(Session::offline(1_234_567).username, "Player567");
        assert_eq!(Session::offline(1_234_000).username, "Player0");
    }

    #[test]
    fn registered_blocks_exist() {
        for id in REGISTERED_BLOCKS_LIST {
            assert!(mc_sim::block::block(id).is_some(), "{id}");
        }
    }
}
