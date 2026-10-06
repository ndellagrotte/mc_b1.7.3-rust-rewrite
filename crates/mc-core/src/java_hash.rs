//! [`JavaHashMap`] and [`JavaHashSet`]: hash collections that iterate in the
//! order a JDK 8+ `java.util.HashMap` / `HashSet` would (PLAN_v4 §3.4 rule 4,
//! §7 risk row 2).
//!
//! Vanilla iterates hash collections in gameplay (`World.positionsToUpdate`,
//! `Chunk.chunkTileEntityMap`, NBT compounds, …), so the order is observable.
//! It depends only on the keys' Java `hashCode` ([`JavaHash`]), the insertion
//! and removal history and the table capacity. The parity target is the JDK 8+
//! `HashMap` the oracle runs on (see `tests/golden/README.md`,
//! `java_hash_order`); Java 6/7 iterated differently.
//!
//! Written clean-room from the behavioural description in that README and the
//! `tests/golden/hash/*` fixtures; no JDK source is used. The behaviour:
//!
//! - **Table.** `2^n` buckets, allocated at 16 on the first insert. The stored
//!   hash is `spread(h) = h ^ (h >>> 16)`; the bucket is `spread & (cap - 1)`.
//!   The capacity doubles when the size exceeds `0.75 * cap` after an insert.
//! - **List bins.** A bucket is a singly linked list; new entries go to the
//!   tail, removal unlinks in place. On resize each bucket splits into the
//!   entries staying at index `i` and those moving to `i + oldCap`, both
//!   keeping their relative order.
//! - **Tree bins.** An insert that appends to a list already holding 8 entries
//!   ([`TREEIFY_THRESHOLD`]) resizes instead when the capacity is below 64
//!   ([`MIN_TREEIFY_CAPACITY`]), and otherwise turns the bucket into a
//!   red-black tree keyed by (spread hash, then [`JavaHash::java_compare`]).
//!   The tree keeps a `next`/`prev` list through its nodes, which is the
//!   iteration order:
//!   - building a tree (from a list) keeps the list order, then moves the
//!     tree's root to the front of the list;
//!   - inserting into a tree links the new node right after its tree parent,
//!     rebalances, then moves the root to the front;
//!   - removing unlinks the node from the list first. If the remaining tree is
//!     small (its root lacks a child, or the root's left child lacks a left
//!     child) the bucket simply becomes a list in the current order; otherwise
//!     the node is deleted from the tree (CLRS, the successor taking the
//!     removed node's place and colour), and the root moves to the front;
//!   - a resize splits the list as for list bins; a half with at most 6
//!     entries ([`UNTREEIFY_THRESHOLD`]) becomes a list, a larger half is
//!     rebuilt as a tree from its list order, except that a tree whose
//!     entries all stay in one half is kept as it is.
//! - **Iteration.** Buckets in index order, each in its list order.
//! - **`clear`** empties every bucket and keeps the capacity.
//!
//! Not provided: Java's `Iterator.remove` (the JDK removes without moving the
//! tree root to the front there, so it orders differently from
//! [`JavaHashMap::remove`]), `putAll`/`new HashMap(Map)` and the
//! capacity-taking constructors (they size the table differently).
//!
//! **Irreproducible case.** Two distinct keys with the same hash in one tree
//! bin, when the key type is not Java `Comparable` with itself
//! ([`JavaHash::java_compare`] returns `None`), are ordered by the JDK with
//! `System.identityHashCode`, which no port can reproduce (the oracle refuses
//! such scenarios). Debug builds panic when that happens; release builds put
//! the new key to the left, deterministically. With vanilla's key classes this
//! needs e.g. `ChunkCoordIntPair`s 32768 chunks apart in one 9+-entry bucket.

use core::cmp::Ordering;
use core::fmt;

/// Bucket count of the first table.
pub const DEFAULT_CAPACITY: usize = 16;
/// A list bin that already holds this many entries is treeified (or the table
/// resized) when another entry is appended.
pub const TREEIFY_THRESHOLD: usize = 8;
/// A tree half of at most this many entries becomes a list on resize.
pub const UNTREEIFY_THRESHOLD: usize = 6;
/// Below this capacity a long bin makes the table grow instead of treeifying.
pub const MIN_TREEIFY_CAPACITY: usize = 64;
/// Capacity limit, as Java's (`1 << 30`); beyond it the table stops growing.
const MAXIMUM_CAPACITY: usize = 1 << 30;

/// Java `hashCode`/`equals` (through [`Eq`]) and, for `Comparable<Self>`
/// classes, `compareTo`.
pub trait JavaHash: Eq {
    /// The Java `hashCode()` of the value.
    fn java_hash(&self) -> i32;

    /// `Some(self.compareTo(other))` when the Java class implements
    /// `Comparable` of itself (`Integer`, `String`); `None` otherwise (the
    /// default, and the case of every vanilla key class). Only consulted to
    /// order distinct keys with equal hashes inside a tree bin.
    fn java_compare(&self, _other: &Self) -> Option<Ordering> {
        None
    }
}

/// `Integer`: the hash is the value; `Comparable`.
impl JavaHash for i32 {
    fn java_hash(&self) -> i32 {
        *self
    }

    fn java_compare(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// `String`: `s[0]*31^(n-1) + … + s[n-1]` over UTF-16 code units, wrapping;
/// `compareTo` is lexicographic over UTF-16 code units (shorter prefix first).
impl JavaHash for str {
    fn java_hash(&self) -> i32 {
        self.encode_utf16().fold(0i32, |h, unit| {
            h.wrapping_mul(31).wrapping_add(i32::from(unit))
        })
    }

    fn java_compare(&self, other: &Self) -> Option<Ordering> {
        Some(self.encode_utf16().cmp(other.encode_utf16()))
    }
}

/// `String`, as [`str`].
impl JavaHash for String {
    fn java_hash(&self) -> i32 {
        self.as_str().java_hash()
    }

    fn java_compare(&self, other: &Self) -> Option<Ordering> {
        self.as_str().java_compare(other.as_str())
    }
}

/// A reference hashes and compares as its target (`&str` keys).
impl<T: JavaHash + ?Sized> JavaHash for &T {
    fn java_hash(&self) -> i32 {
        (**self).java_hash()
    }

    fn java_compare(&self, other: &Self) -> Option<Ordering> {
        (**self).java_compare(*other)
    }
}

/// `ChunkCoordIntPair`, with vanilla's `hashCode`/`equals`. Only 15 bits per
/// axis reach the hash, so `(32808, -3)` and `(40, -3)` collide.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChunkCoordIntPair {
    /// `chunkXPos`.
    pub chunk_x_pos: i32,
    /// `chunkZPos`.
    pub chunk_z_pos: i32,
}

impl ChunkCoordIntPair {
    /// `new ChunkCoordIntPair(x, z)`.
    pub const fn new(chunk_x_pos: i32, chunk_z_pos: i32) -> Self {
        Self {
            chunk_x_pos,
            chunk_z_pos,
        }
    }

    /// `ChunkCoordIntPair.chunkXZ2Int`.
    // src: ChunkCoordIntPair.java:13
    pub const fn chunk_xz_to_int(x: i32, z: i32) -> i32 {
        (if x < 0 { i32::MIN } else { 0 })
            | (x & 32767) << 16
            | (if z < 0 { 32768 } else { 0 })
            | (z & 32767)
    }
}

impl JavaHash for ChunkCoordIntPair {
    // src: ChunkCoordIntPair.java:16-18 (equals, derived: :20-23)
    fn java_hash(&self) -> i32 {
        Self::chunk_xz_to_int(self.chunk_x_pos, self.chunk_z_pos)
    }
}

/// `ChunkPosition`, with vanilla's `hashCode`/`equals`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChunkPosition {
    /// `x`.
    pub x: i32,
    /// `y`.
    pub y: i32,
    /// `z`.
    pub z: i32,
}

impl ChunkPosition {
    /// `new ChunkPosition(x, y, z)`.
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

impl JavaHash for ChunkPosition {
    // src: ChunkPosition.java:23-25 (equals, derived: :14-21)
    fn java_hash(&self) -> i32 {
        self.x
            .wrapping_mul(8976890)
            .wrapping_add(self.y.wrapping_mul(981131))
            .wrapping_add(self.z)
    }
}

/// `ChunkCoordinates`, with vanilla's `hashCode`/`equals` (equals, derived:
/// ChunkCoordinates.java:23-30). Only the `HashMap`-relevant parts are here;
/// the constructors, `compareChunkCoordinate` and `getSqDistanceTo`
/// (ChunkCoordinates.java:8-21, 36-55) arrive with their first user. Its raw
/// `Comparable` (not `Comparable<ChunkCoordinates>`) does not count for
/// `HashMap`, so [`JavaHash::java_compare`] is `None`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChunkCoordinates {
    /// `x`.
    pub pos_x: i32,
    /// `y`.
    pub pos_y: i32,
    /// `z`.
    pub pos_z: i32,
}

impl ChunkCoordinates {
    /// `new ChunkCoordinates(x, y, z)`.
    pub const fn new(pos_x: i32, pos_y: i32, pos_z: i32) -> Self {
        Self {
            pos_x,
            pos_y,
            pos_z,
        }
    }
}

impl JavaHash for ChunkCoordinates {
    /// `this.x + this.z << 8 + this.y << 16`, which Java parses as
    /// `((x + z) << (8 + y)) << 16` with shift distances taken mod 32. Vanilla
    /// bug, transcribed (§3.4 rule 6): the low 16 bits are always zero and for
    /// most `y` all keys collide.
    // src: ChunkCoordinates.java:32-34
    fn java_hash(&self) -> i32 {
        let shift = 8i32.wrapping_add(self.pos_y) as u32; // wrapping_shl masks to 0..31
        self.pos_x
            .wrapping_add(self.pos_z)
            .wrapping_shl(shift)
            .wrapping_shl(16)
    }
}

/// No node.
const NIL: u32 = u32::MAX;

/// `h ^ (h >>> 16)`: the hash a node stores and a tree orders by.
const fn spread(h: i32) -> i32 {
    h ^ ((h as u32) >> 16) as i32
}

/// One entry. `next` is the bucket's list; the other links are only
/// meaningful while the bucket is a tree bin (`prev` doubles the list).
#[derive(Clone)]
struct Node<K, V> {
    hash: i32,
    entry: Option<(K, V)>,
    next: u32,
    prev: u32,
    parent: u32,
    left: u32,
    right: u32,
    red: bool,
}

/// A bucket: the head of its list (for a tree bin, the root) and its kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Bucket {
    head: u32,
    tree: bool,
}

const EMPTY: Bucket = Bucket {
    head: NIL,
    tree: false,
};

/// `java.util.HashMap` with JDK 8+ iteration order; see the module docs.
///
/// Created with the default capacity and load factor, as `new HashMap()`.
/// Nodes live in a `Vec` arena linked by index; freed slots are reused.
#[derive(Clone)]
pub struct JavaHashMap<K, V> {
    table: Vec<Bucket>,
    nodes: Vec<Node<K, V>>,
    free: Vec<u32>,
    len: usize,
    threshold: usize,
}

impl<K, V> Default for JavaHashMap<K, V> {
    fn default() -> Self {
        Self {
            table: Vec::new(),
            nodes: Vec::new(),
            free: Vec::new(),
            len: 0,
            threshold: 0,
        }
    }
}

impl<K: JavaHash, V> JavaHashMap<K, V> {
    /// `new HashMap()`: no table until the first insert.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of entries (`size()`).
    pub fn len(&self) -> usize {
        self.len
    }

    /// `isEmpty()`.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Current bucket count (0 before the first insert). Java does not expose
    /// it; useful to check against the oracle's capacity notes.
    pub fn capacity(&self) -> usize {
        self.table.len()
    }

    /// `put(key, value)`: returns the previous value. An existing entry keeps
    /// its key and its position.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        if self.table.is_empty() {
            self.resize();
        }
        let hash = spread(key.java_hash());
        let idx = self.index(hash);
        let found = self.find(idx, hash, &key);
        if found != NIL {
            let (_, v) = self.node_mut(found).entry.as_mut().expect("live node");
            return Some(core::mem::replace(v, value));
        }
        let x = self.alloc(hash, key, value);
        let bucket = self.table[idx];
        if bucket.head == NIL {
            self.table[idx].head = x;
        } else if bucket.tree {
            self.tree_insert(idx, x);
        } else {
            let mut tail = bucket.head;
            let mut count = 1;
            while self.node(tail).next != NIL {
                tail = self.node(tail).next;
                count += 1;
            }
            self.node_mut(tail).next = x;
            if count >= TREEIFY_THRESHOLD {
                if self.table.len() < MIN_TREEIFY_CAPACITY {
                    self.resize();
                } else {
                    self.treeify(idx);
                }
            }
        }
        self.len += 1;
        if self.len > self.threshold {
            self.resize();
        }
        None
    }

    /// `get(key)`.
    pub fn get(&self, key: &K) -> Option<&V> {
        let n = self.lookup(key);
        (n != NIL).then(|| &self.node(n).entry.as_ref().expect("live node").1)
    }

    /// `get(key)`, mutable.
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let n = self.lookup(key);
        if n == NIL {
            return None;
        }
        Some(&mut self.node_mut(n).entry.as_mut().expect("live node").1)
    }

    /// `containsKey(key)`.
    pub fn contains_key(&self, key: &K) -> bool {
        self.lookup(key) != NIL
    }

    /// `remove(key)`: returns the removed value.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        if self.table.is_empty() {
            return None;
        }
        let hash = spread(key.java_hash());
        let idx = self.index(hash);
        let p = self.find(idx, hash, key);
        if p == NIL {
            return None;
        }
        if self.table[idx].tree {
            self.tree_remove(idx, p);
        } else {
            let head = self.table[idx].head;
            let next = self.node(p).next;
            if head == p {
                self.table[idx].head = next;
            } else {
                let mut q = head;
                while self.node(q).next != p {
                    q = self.node(q).next;
                }
                self.node_mut(q).next = next;
            }
        }
        self.len -= 1;
        let (_, v) = self.release(p);
        Some(v)
    }

    /// `clear()`: empties every bucket; the capacity stays.
    pub fn clear(&mut self) {
        self.table.fill(EMPTY);
        self.nodes.clear();
        self.free.clear();
        self.len = 0;
    }

    /// Entries in Java iteration order (`entrySet().iterator()`).
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            map: self,
            bucket: 0,
            node: NIL,
            remaining: self.len,
        }
    }

    /// Keys in Java iteration order (`keySet()`).
    pub fn keys(&self) -> impl Iterator<Item = &K> + '_ {
        self.iter().map(|(k, _)| k)
    }

    /// Values in Java iteration order (`values()`).
    pub fn values(&self) -> impl Iterator<Item = &V> + '_ {
        self.iter().map(|(_, v)| v)
    }

    // ---- arena -------------------------------------------------------------

    fn node(&self, i: u32) -> &Node<K, V> {
        &self.nodes[i as usize]
    }

    fn node_mut(&mut self, i: u32) -> &mut Node<K, V> {
        &mut self.nodes[i as usize]
    }

    fn key(&self, i: u32) -> &K {
        &self.node(i).entry.as_ref().expect("live node").0
    }

    fn alloc(&mut self, hash: i32, key: K, value: V) -> u32 {
        let node = Node {
            hash,
            entry: Some((key, value)),
            next: NIL,
            prev: NIL,
            parent: NIL,
            left: NIL,
            right: NIL,
            red: false,
        };
        if let Some(i) = self.free.pop() {
            self.nodes[i as usize] = node;
            i
        } else {
            let i = u32::try_from(self.nodes.len()).expect("JavaHashMap: too many entries");
            assert!(i != NIL, "JavaHashMap: too many entries");
            self.nodes.push(node);
            i
        }
    }

    fn release(&mut self, i: u32) -> (K, V) {
        self.free.push(i);
        self.node_mut(i).entry.take().expect("live node")
    }

    fn red(&self, i: u32) -> bool {
        i != NIL && self.node(i).red
    }

    // ---- table -------------------------------------------------------------

    fn index(&self, hash: i32) -> usize {
        (hash as u32 as usize) & (self.table.len() - 1)
    }

    /// The node holding `key` in bucket `idx`, or `NIL`. Scans the bucket's
    /// list, which a tree bin maintains too; bins are short.
    fn find(&self, idx: usize, hash: i32, key: &K) -> u32 {
        let mut p = self.table[idx].head;
        while p != NIL {
            let n = self.node(p);
            if n.hash == hash && self.key(p) == key {
                return p;
            }
            p = n.next;
        }
        NIL
    }

    fn lookup(&self, key: &K) -> u32 {
        if self.table.is_empty() {
            return NIL;
        }
        let hash = spread(key.java_hash());
        self.find(self.index(hash), hash, key)
    }

    /// Allocates the first table or doubles it, splitting every bucket.
    fn resize(&mut self) {
        let old_cap = self.table.len();
        if old_cap >= MAXIMUM_CAPACITY {
            self.threshold = usize::MAX;
            return;
        }
        let new_cap = if old_cap == 0 {
            DEFAULT_CAPACITY
        } else {
            old_cap * 2
        };
        self.threshold = new_cap / 4 * 3;
        let old = core::mem::replace(&mut self.table, vec![EMPTY; new_cap]);
        for (j, bucket) in old.into_iter().enumerate() {
            if bucket.head == NIL {
                continue;
            }
            if self.node(bucket.head).next == NIL {
                let i = self.index(self.node(bucket.head).hash);
                self.table[i] = bucket;
            } else {
                self.split(j, bucket, old_cap);
            }
        }
    }

    /// Splits old bucket `j` into `j` and `j + bit`, keeping list order; for a
    /// tree bin, each half is untreeified, kept, or rebuilt.
    fn split(&mut self, j: usize, bucket: Bucket, bit: usize) {
        let mut heads = [NIL; 2];
        let mut tails = [NIL; 2];
        let mut counts = [0usize; 2];
        let mut e = bucket.head;
        while e != NIL {
            let next = self.node(e).next;
            let half = usize::from((self.node(e).hash as u32 as usize) & bit != 0);
            if tails[half] == NIL {
                heads[half] = e;
            } else {
                self.node_mut(tails[half]).next = e;
            }
            self.node_mut(e).prev = tails[half];
            self.node_mut(e).next = NIL;
            tails[half] = e;
            counts[half] += 1;
            e = next;
        }
        let both = heads[0] != NIL && heads[1] != NIL;
        for (half, idx) in [(0, j), (1, j + bit)] {
            if heads[half] == NIL {
                continue;
            }
            let tree = bucket.tree && counts[half] > UNTREEIFY_THRESHOLD;
            self.table[idx] = Bucket {
                head: heads[half],
                tree,
            };
            if tree && both {
                self.treeify(idx);
            }
        }
    }

    // ---- tree bins ---------------------------------------------------------

    /// True when a key with spread hash `hash` belongs left of node `p`.
    fn goes_left(&self, hash: i32, key: &K, p: u32) -> bool {
        let ph = self.node(p).hash;
        if ph != hash {
            return ph > hash;
        }
        match key.java_compare(self.key(p)) {
            Some(Ordering::Less) => true,
            Some(Ordering::Greater) => false,
            _ => {
                if cfg!(debug_assertions) {
                    panic!(
                        "JavaHashMap: distinct non-Comparable keys with equal hash {hash} in one \
                         tree bin; Java orders them by identityHashCode (irreproducible)"
                    );
                }
                true
            }
        }
    }

    /// Descends from `root` to the leaf position for node `x` and hangs it
    /// there (tree links only).
    fn attach(&mut self, root: u32, x: u32) -> u32 {
        let hash = self.node(x).hash;
        let mut p = root;
        loop {
            let left = self.goes_left(hash, self.key(x), p);
            let child = if left {
                self.node(p).left
            } else {
                self.node(p).right
            };
            if child == NIL {
                if left {
                    self.node_mut(p).left = x;
                } else {
                    self.node_mut(p).right = x;
                }
                self.node_mut(x).parent = p;
                return p;
            }
            p = child;
        }
    }

    /// Builds a tree over bucket `idx`'s list, inserting in list order, and
    /// moves the root to the front. `prev` links must already be set or are
    /// set here.
    fn treeify(&mut self, idx: usize) {
        self.table[idx].tree = true;
        let mut root = NIL;
        let mut prev = NIL;
        let mut x = self.table[idx].head;
        while x != NIL {
            let n = self.node_mut(x);
            n.prev = prev;
            n.left = NIL;
            n.right = NIL;
            if root == NIL {
                n.parent = NIL;
                n.red = false;
                root = x;
            } else {
                self.attach(root, x);
                root = self.fix_insert(root, x);
            }
            prev = x;
            x = self.node(x).next;
        }
        self.move_root_to_front(idx, root);
    }

    fn root_of(&self, mut p: u32) -> u32 {
        while self.node(p).parent != NIL {
            p = self.node(p).parent;
        }
        p
    }

    /// Inserts fresh node `x` into tree bin `idx`.
    fn tree_insert(&mut self, idx: usize, x: u32) {
        let root = self.root_of(self.table[idx].head);
        let xp = self.attach(root, x);
        // List: right after the tree parent.
        let after = self.node(xp).next;
        self.node_mut(xp).next = x;
        self.node_mut(x).prev = xp;
        self.node_mut(x).next = after;
        if after != NIL {
            self.node_mut(after).prev = x;
        }
        let root = self.fix_insert(root, x);
        self.move_root_to_front(idx, root);
    }

    /// Removes node `p` from tree bin `idx` (list and tree).
    fn tree_remove(&mut self, idx: usize, p: u32) {
        let (succ, pred) = (self.node(p).next, self.node(p).prev);
        if pred == NIL {
            self.table[idx].head = succ;
        } else {
            self.node_mut(pred).next = succ;
        }
        if succ != NIL {
            self.node_mut(succ).prev = pred;
        }
        let first = self.table[idx].head;
        if first == NIL {
            self.table[idx].tree = false;
            return;
        }
        // The tree still contains `p` here.
        let root = self.root_of(first);
        let (rl, rr) = (self.node(root).left, self.node(root).right);
        if rr == NIL || rl == NIL || self.node(rl).left == NIL {
            self.table[idx].tree = false; // small: back to a list, current order
            return;
        }
        let root = self.tree_delete(root, p);
        self.move_root_to_front(idx, root);
    }

    /// Moves `root` to the front of bucket `idx`'s list.
    fn move_root_to_front(&mut self, idx: usize, root: u32) {
        let first = self.table[idx].head;
        if root == first {
            return;
        }
        let (rn, rp) = (self.node(root).next, self.node(root).prev);
        if rn != NIL {
            self.node_mut(rn).prev = rp;
        }
        if rp != NIL {
            self.node_mut(rp).next = rn;
        }
        self.node_mut(root).next = first;
        self.node_mut(root).prev = NIL;
        if first != NIL {
            self.node_mut(first).prev = root;
        }
        self.table[idx].head = root;
    }

    /// Puts `new` where `old` hangs under `parent` (or as the root).
    fn replace_child(&mut self, root: &mut u32, parent: u32, old: u32, new: u32) {
        if parent == NIL {
            *root = new;
        } else if self.node(parent).left == old {
            self.node_mut(parent).left = new;
        } else {
            self.node_mut(parent).right = new;
        }
    }

    fn rotate_left(&mut self, mut root: u32, p: u32) -> u32 {
        let r = self.node(p).right;
        let rl = self.node(r).left;
        self.node_mut(p).right = rl;
        if rl != NIL {
            self.node_mut(rl).parent = p;
        }
        let pp = self.node(p).parent;
        self.node_mut(r).parent = pp;
        self.replace_child(&mut root, pp, p, r);
        self.node_mut(r).left = p;
        self.node_mut(p).parent = r;
        root
    }

    fn rotate_right(&mut self, mut root: u32, p: u32) -> u32 {
        let l = self.node(p).left;
        let lr = self.node(l).right;
        self.node_mut(p).left = lr;
        if lr != NIL {
            self.node_mut(lr).parent = p;
        }
        let pp = self.node(p).parent;
        self.node_mut(l).parent = pp;
        self.replace_child(&mut root, pp, p, l);
        self.node_mut(l).right = p;
        self.node_mut(p).parent = l;
        root
    }

    /// Red-black insert fix-up (CLRS) for freshly attached `x`; returns the
    /// root.
    fn fix_insert(&mut self, mut root: u32, mut x: u32) -> u32 {
        self.node_mut(x).red = true;
        loop {
            let xp = self.node(x).parent;
            if xp == NIL {
                self.node_mut(x).red = false;
                return x;
            }
            if !self.node(xp).red {
                return root;
            }
            let xpp = self.node(xp).parent;
            if xpp == NIL {
                return root;
            }
            let parent_is_left = self.node(xpp).left == xp;
            let uncle = if parent_is_left {
                self.node(xpp).right
            } else {
                self.node(xpp).left
            };
            if self.red(uncle) {
                self.node_mut(uncle).red = false;
                self.node_mut(xp).red = false;
                self.node_mut(xpp).red = true;
                x = xpp;
                continue;
            }
            let mut xp = xp;
            if parent_is_left && self.node(xp).right == x {
                x = xp;
                root = self.rotate_left(root, x);
                xp = self.node(x).parent;
            } else if !parent_is_left && self.node(xp).left == x {
                x = xp;
                root = self.rotate_right(root, x);
                xp = self.node(x).parent;
            }
            let xpp = self.node(xp).parent;
            self.node_mut(xp).red = false;
            self.node_mut(xpp).red = true;
            root = if parent_is_left {
                self.rotate_right(root, xpp)
            } else {
                self.rotate_left(root, xpp)
            };
        }
    }

    /// Deletes `z` from the tree (CLRS: with two children, its in-order
    /// successor takes its place and colour). Returns the root. `z` stands in
    /// for the missing leaf during the fix-up and is detached afterwards.
    fn tree_delete(&mut self, mut root: u32, z: u32) -> u32 {
        let (zl, zr) = (self.node(z).left, self.node(z).right);
        if zl != NIL && zr != NIL {
            let mut s = zr;
            while self.node(s).left != NIL {
                s = self.node(s).left;
            }
            // Swap the positions (and colours) of z and its successor s.
            let (zc, sc) = (self.node(z).red, self.node(s).red);
            self.node_mut(z).red = sc;
            self.node_mut(s).red = zc;
            let sr = self.node(s).right;
            let zp = self.node(z).parent;
            if s == zr {
                self.node_mut(z).parent = s;
                self.node_mut(s).right = z;
            } else {
                let sp = self.node(s).parent;
                self.node_mut(z).parent = sp;
                self.node_mut(sp).left = z; // s is a left child below zr
                self.node_mut(s).right = zr;
                self.node_mut(zr).parent = s;
            }
            self.node_mut(z).left = NIL;
            self.node_mut(z).right = sr;
            if sr != NIL {
                self.node_mut(sr).parent = z;
            }
            self.node_mut(s).left = zl;
            self.node_mut(zl).parent = s;
            self.node_mut(s).parent = zp;
            self.replace_child(&mut root, zp, z, s);
        }
        // z now has at most one child.
        let child = if self.node(z).left != NIL {
            self.node(z).left
        } else {
            self.node(z).right
        };
        let x = if child != NIL {
            let zp = self.node(z).parent;
            self.node_mut(child).parent = zp;
            self.replace_child(&mut root, zp, z, child);
            if zp == NIL {
                self.node_mut(child).red = false;
            }
            let n = self.node_mut(z);
            (n.left, n.right, n.parent) = (NIL, NIL, NIL);
            child
        } else {
            z
        };
        if !self.node(z).red {
            root = self.fix_delete(root, x);
        }
        if x == z {
            let zp = self.node(z).parent;
            self.node_mut(z).parent = NIL;
            if zp != NIL {
                if self.node(zp).left == z {
                    self.node_mut(zp).left = NIL;
                } else if self.node(zp).right == z {
                    self.node_mut(zp).right = NIL;
                }
            }
        }
        root
    }

    /// Red-black delete fix-up (CLRS) starting at `x`; returns the root.
    fn fix_delete(&mut self, mut root: u32, mut x: u32) -> u32 {
        loop {
            if x == NIL || x == root {
                return root;
            }
            let xp = self.node(x).parent;
            if xp == NIL {
                self.node_mut(x).red = false;
                return x;
            }
            if self.node(x).red {
                self.node_mut(x).red = false;
                return root;
            }
            let x_is_left = self.node(xp).left == x;
            let sibling = |m: &Self| {
                if x_is_left {
                    m.node(xp).right
                } else {
                    m.node(xp).left
                }
            };
            let mut w = sibling(self);
            if self.red(w) {
                self.node_mut(w).red = false;
                self.node_mut(xp).red = true;
                root = if x_is_left {
                    self.rotate_left(root, xp)
                } else {
                    self.rotate_right(root, xp)
                };
                w = sibling(self);
            }
            if w == NIL {
                x = xp;
                continue;
            }
            let (near, far) = if x_is_left {
                (self.node(w).left, self.node(w).right)
            } else {
                (self.node(w).right, self.node(w).left)
            };
            if !self.red(near) && !self.red(far) {
                self.node_mut(w).red = true;
                x = xp;
                continue;
            }
            if !self.red(far) {
                if near != NIL {
                    self.node_mut(near).red = false;
                }
                self.node_mut(w).red = true;
                root = if x_is_left {
                    self.rotate_right(root, w)
                } else {
                    self.rotate_left(root, w)
                };
                w = sibling(self);
            }
            self.node_mut(w).red = self.node(xp).red;
            self.node_mut(xp).red = false;
            let far = if x_is_left {
                self.node(w).right
            } else {
                self.node(w).left
            };
            if far != NIL {
                self.node_mut(far).red = false;
            }
            root = if x_is_left {
                self.rotate_left(root, xp)
            } else {
                self.rotate_right(root, xp)
            };
            x = root;
        }
    }
}

impl<K: JavaHash + fmt::Debug, V: fmt::Debug> fmt::Debug for JavaHashMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

/// Iterator over a [`JavaHashMap`] in Java order.
pub struct Iter<'a, K, V> {
    map: &'a JavaHashMap<K, V>,
    bucket: usize,
    node: u32,
    remaining: usize,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        while self.node == NIL {
            let bucket = self.map.table.get(self.bucket)?;
            self.node = bucket.head;
            self.bucket += 1;
        }
        let n = &self.map.nodes[self.node as usize];
        self.node = n.next;
        self.remaining -= 1;
        let (k, v) = n.entry.as_ref().expect("live node");
        Some((k, v))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<K, V> ExactSizeIterator for Iter<'_, K, V> {}

impl<'a, K: JavaHash, V> IntoIterator for &'a JavaHashMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// `java.util.HashSet` (a [`JavaHashMap`] with unit values, as Java's is a
/// `HashMap` with a dummy value) with JDK 8+ iteration order.
#[derive(Clone, Default)]
pub struct JavaHashSet<K> {
    map: JavaHashMap<K, ()>,
}

impl<K: JavaHash> JavaHashSet<K> {
    /// `new HashSet()`.
    pub fn new() -> Self {
        Self {
            map: JavaHashMap::new(),
        }
    }

    /// `size()`.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// `isEmpty()`.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Current bucket count; see [`JavaHashMap::capacity`].
    pub fn capacity(&self) -> usize {
        self.map.capacity()
    }

    /// `add(key)`: true if the key was not present. A present key is left
    /// untouched (the set keeps the old key).
    pub fn insert(&mut self, key: K) -> bool {
        if self.map.contains_key(&key) {
            return false;
        }
        self.map.insert(key, ());
        true
    }

    /// `contains(key)`.
    pub fn contains(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    /// `remove(key)`: true if the key was present.
    pub fn remove(&mut self, key: &K) -> bool {
        self.map.remove(key).is_some()
    }

    /// `clear()`; the capacity stays.
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Keys in Java iteration order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &K> + '_ {
        self.map.iter().map(|(k, _)| k)
    }
}

impl<K: JavaHash + fmt::Debug> fmt::Debug for JavaHashSet<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JavaRandom;

    /// Hash codes from the fixtures' `hash_codes` lines.
    #[test]
    fn hash_codes_match_fixtures() {
        let ccip = |x, z| ChunkCoordIntPair::new(x, z).java_hash();
        assert_eq!(ccip(0, 0), 0);
        assert_eq!(ccip(1, 0), 65536);
        assert_eq!(ccip(-1, 1), -65535);
        assert_eq!(ccip(-1, -1), -1);
        assert_eq!(ccip(0, -1), 65535);
        assert_eq!(ccip(-9, -9), -524297);
        assert_eq!(ccip(-8, 3), -524285);
        assert_eq!(ccip(32808, -3), ccip(40, -3));
        let cp = |x, y, z| ChunkPosition::new(x, y, z).java_hash();
        assert_eq!(cp(5, 64, -8), 107676826);
        assert_eq!(cp(7, 63, -6), 124649477);
        let cc = |x, y, z| ChunkCoordinates::new(x, y, z).java_hash();
        assert_eq!(cc(1, 0, 2), 50331648);
        assert_eq!(cc(3, 7, -1), 0);
        assert_eq!(cc(-4, 63, 1), -25165824);
        assert_eq!(cc(6, 64, 0), 100663296);
        assert_eq!(cc(9, 127, -2), 58720256);
        assert_eq!(cc(11, -9, 3), 0);
        assert_eq!(cc(-50, 24, -61), -7274496);
        assert_eq!(cc(100, 56, 3), 6750208);
        assert_eq!((-262149).java_hash(), -262149);
        assert_eq!("stone".java_hash(), 109770853);
        assert_eq!("bedrock".java_hash(), -231550106);
        assert_eq!(String::from("stonebrick").java_hash(), 1669588732);
        assert_eq!("Aa".java_hash(), 2112);
        assert_eq!("BB".java_hash(), 2112);
    }

    #[test]
    fn string_compare_is_utf16() {
        // U+FF5E (one unit 0xFF5E) sorts after U+1F600 (surrogates 0xD83D…)
        // in Java, the reverse of UTF-8 byte order.
        assert_eq!(
            "\u{ff5e}".java_compare("\u{1f600}"),
            Some(Ordering::Greater)
        );
        assert_eq!("Aa".java_compare("BB"), Some(Ordering::Less));
        assert_eq!("ab".java_compare("abc"), Some(Ordering::Less));
    }

    fn order<K: JavaHash + Clone>(set: &JavaHashSet<K>) -> Vec<K> {
        set.iter().cloned().collect()
    }

    /// `integer-chunk-keys` after 9 ops: the 9th key appended to a bin of 8
    /// at capacity 16 doubles the table; the bin splits in order.
    #[test]
    fn long_bin_below_64_resizes_preserving_order() {
        let mut set = JavaHashSet::new();
        for k in (0..9).map(|i| i * 16) {
            set.insert(k);
        }
        assert_eq!(set.capacity(), 32);
        assert_eq!(order(&set), [0, 32, 64, 96, 128, 16, 48, 80, 112]);
    }

    #[test]
    fn threshold_resize_and_removal() {
        let mut set = JavaHashSet::new();
        for k in [0, 16, 32, 48] {
            set.insert(k); // all in bucket 0
        }
        assert!(set.remove(&16));
        assert!(!set.remove(&16));
        assert!(!set.insert(32));
        assert!(set.insert(16)); // a re-added key goes to its bucket's tail
        assert_eq!(order(&set), [0, 32, 48, 16]);
        for k in 1..=9 {
            set.insert(k);
        }
        assert_eq!((set.len(), set.capacity()), (13, 32)); // 13 > 0.75 * 16
        assert_eq!(order(&set), [0, 32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 48, 16]);
        set.clear();
        assert!(set.is_empty());
        assert_eq!(set.capacity(), 32);
        assert_eq!(order(&set), Vec::<i32>::new());
    }

    #[test]
    fn map_api() {
        let mut map = JavaHashMap::new();
        assert_eq!(map.insert("a".to_string(), 1), None);
        assert_eq!(map.insert("a".to_string(), 2), Some(1));
        assert_eq!(map.get(&"a".to_string()), Some(&2));
        *map.get_mut(&"a".to_string()).unwrap() += 1;
        assert_eq!(map.values().copied().collect::<Vec<_>>(), [3]);
        assert_eq!(map.remove(&"a".to_string()), Some(3));
        assert_eq!(map.remove(&"a".to_string()), None);
        assert!(map.is_empty());
    }

    /// Checks list/tree consistency and red-black properties of every bucket.
    fn check<K: JavaHash, V>(m: &JavaHashMap<K, V>) {
        let mut seen = 0;
        for (idx, b) in m.table.iter().enumerate() {
            let mut list = Vec::new();
            let mut p = b.head;
            while p != NIL {
                assert_eq!(m.index(m.node(p).hash), idx);
                list.push(p);
                p = m.node(p).next;
            }
            seen += list.len();
            if !b.tree {
                continue;
            }
            let root = b.head;
            assert_eq!(m.node(root).parent, NIL, "root at the front");
            assert!(!m.node(root).red);
            for (i, &n) in list.iter().enumerate() {
                let prev = if i == 0 { NIL } else { list[i - 1] };
                assert_eq!(m.node(n).prev, prev);
            }
            fn walk<K: JavaHash, V>(m: &JavaHashMap<K, V>, n: u32, out: &mut Vec<u32>) -> usize {
                if n == NIL {
                    return 1;
                }
                let node = m.node(n);
                for c in [node.left, node.right] {
                    if c != NIL {
                        assert_eq!(m.node(c).parent, n);
                        assert!(!(node.red && m.node(c).red), "red-red");
                    }
                }
                if node.left != NIL {
                    assert!(m.node(node.left).hash <= node.hash);
                }
                if node.right != NIL {
                    assert!(m.node(node.right).hash >= node.hash);
                }
                let bl = walk(m, node.left, out);
                out.push(n);
                let br = walk(m, node.right, out);
                assert_eq!(bl, br, "black height");
                bl + usize::from(!node.red)
            }
            let mut inorder = Vec::new();
            walk(m, root, &mut inorder);
            let mut a = inorder.clone();
            let mut l = list.clone();
            a.sort_unstable();
            l.sort_unstable();
            assert_eq!(a, l, "tree and list hold the same nodes");
        }
        assert_eq!(seen, m.len());
    }

    /// Random adds/removes with heavy collisions keep the structure valid and
    /// the contents right.
    #[test]
    fn stress_tree_bins() {
        let mut rng = JavaRandom::new(1234);
        let mut set = JavaHashSet::new();
        let mut model = std::collections::BTreeSet::new();
        let mut max_trees = 0;
        for step in 0..40_000 {
            // Keys colliding on the low bits: many share a bucket.
            let k = rng.next_int_bounded(600) << 12;
            if rng.next_int_bounded(3) == 0 {
                assert_eq!(set.remove(&k), model.remove(&k));
            } else {
                assert_eq!(set.insert(k), model.insert(k));
            }
            if step % 97 == 0 {
                check(&set.map);
                max_trees = max_trees.max(set.map.table.iter().filter(|b| b.tree).count());
            }
            if step == 20_000 {
                set.clear();
                model.clear();
            }
        }
        check(&set.map);
        assert!(max_trees > 10, "tree bins exercised");
        let mut got = order(&set);
        got.sort_unstable();
        assert_eq!(got, model.into_iter().collect::<Vec<_>>());
    }
}
