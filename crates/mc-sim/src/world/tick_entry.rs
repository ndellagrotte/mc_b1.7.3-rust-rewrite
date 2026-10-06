//! `NextTickListEntry` (NextTickListEntry.java): a scheduled block update.

use std::cmp::Ordering;

/// `NextTickListEntry`.
///
/// Two notions of equality coexist in vanilla and both are kept:
///
/// - `World.scheduledTickTreeSet` is a `TreeSet` ordered by `compareTo`
///   (`scheduledTime`, then `tickEntryID`; NextTickListEntry.java:38-54). This
///   type's `Ord`/`Eq` are exactly that order, so `BTreeSet<NextTickListEntry>`
///   iterates and removes like the `TreeSet`. `tickEntryID` is unique, so the
///   order is total.
/// - `World.scheduledTickSet` is a `HashSet` using `equals` (position and block
///   id; NextTickListEntry.java:20-27). It is only used for membership, never
///   iterated, so the port keys a `BTreeSet` by [`NextTickListEntry::key`]
///   (PLAN_v4 §3.4 rule 4).
#[derive(Clone, Copy, Debug)]
pub struct NextTickListEntry {
    /// `xCoord`.
    pub x_coord: i32,
    /// `yCoord`.
    pub y_coord: i32,
    /// `zCoord`.
    pub z_coord: i32,
    /// `blockID`.
    pub block_id: i32,
    /// `scheduledTime` (0 unless `setScheduledTime` is called).
    pub scheduled_time: i64,
    /// `tickEntryID`: creation order, from the JVM-global
    /// `NextTickListEntry.nextTickEntryID`, here
    /// [`crate::world::World::next_tick_entry_id`].
    pub tick_entry_id: i64,
}

/// `equals`/`hashCode` identity of an entry: `(x, y, z, blockID)`.
pub type TickKey = (i32, i32, i32, i32);

impl NextTickListEntry {
    /// `new NextTickListEntry(x, y, z, blockID)`; `tick_entry_id` is the value
    /// of the static counter, which the caller post-increments.
    // src: NextTickListEntry.java:12-18
    pub fn new(x: i32, y: i32, z: i32, block_id: i32, tick_entry_id: i64) -> Self {
        NextTickListEntry {
            x_coord: x,
            y_coord: y,
            z_coord: z,
            block_id,
            scheduled_time: 0,
            tick_entry_id,
        }
    }

    /// `setScheduledTime`.
    // src: NextTickListEntry.java:33-36
    pub fn set_scheduled_time(&mut self, t: i64) -> &mut Self {
        self.scheduled_time = t;
        self
    }

    /// The `equals` key (NextTickListEntry.java:20-27).
    pub fn key(&self) -> TickKey {
        (self.x_coord, self.y_coord, self.z_coord, self.block_id)
    }

    /// `hashCode()`, kept for reference; unused because the hash set is never
    /// iterated.
    // src: NextTickListEntry.java:29-31
    pub fn java_hash_code(&self) -> i32 {
        self.x_coord
            .wrapping_mul(128)
            .wrapping_mul(1024)
            .wrapping_add(self.z_coord.wrapping_mul(128))
            .wrapping_add(self.y_coord)
            .wrapping_mul(256)
            .wrapping_add(self.block_id)
    }

    /// `comparer`: scheduled time, then creation order.
    // src: NextTickListEntry.java:38-48
    pub fn comparer(&self, other: &Self) -> Ordering {
        if self.scheduled_time < other.scheduled_time {
            Ordering::Less
        } else if self.scheduled_time > other.scheduled_time {
            Ordering::Greater
        } else if self.tick_entry_id < other.tick_entry_id {
            Ordering::Less
        } else if self.tick_entry_id > other.tick_entry_id {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}

impl PartialEq for NextTickListEntry {
    fn eq(&self, other: &Self) -> bool {
        self.comparer(other) == Ordering::Equal
    }
}

impl Eq for NextTickListEntry {}

impl PartialOrd for NextTickListEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NextTickListEntry {
    // src: NextTickListEntry.java:52-54
    fn cmp(&self, other: &Self) -> Ordering {
        self.comparer(other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn tree_set_order_is_time_then_creation() {
        let mut a = NextTickListEntry::new(0, 64, 0, 8, 0);
        a.set_scheduled_time(15);
        let mut b = NextTickListEntry::new(1, 64, 0, 8, 1);
        b.set_scheduled_time(10);
        let mut c = NextTickListEntry::new(2, 64, 0, 8, 2);
        c.set_scheduled_time(10);
        let set: BTreeSet<_> = [a, c, b].into_iter().collect();
        let order: Vec<i32> = set.iter().map(|e| e.x_coord).collect();
        assert_eq!(order, vec![1, 2, 0]);
        // equals ignores time and id; the ordering does not.
        let d = NextTickListEntry::new(0, 64, 0, 8, 7);
        assert_eq!(d.key(), a.key());
        assert_ne!(d, a);
    }
}
