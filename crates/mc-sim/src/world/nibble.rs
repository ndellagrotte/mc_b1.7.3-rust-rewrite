//! `NibbleArray` (NibbleArray.java): 4-bit values packed two per byte, in the
//! chunk's `x << 11 | z << 7 | y` order.

/// `NibbleArray`.
#[derive(Clone, PartialEq, Eq)]
pub struct NibbleArray {
    /// `data`: `len >> 1` bytes. The even index of a pair is the low nibble.
    pub data: Box<[u8]>,
}

impl std::fmt::Debug for NibbleArray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NibbleArray({} bytes)", self.data.len())
    }
}

impl NibbleArray {
    /// `new NibbleArray(len)`: `len >> 1` zero bytes.
    // src: NibbleArray.java:6-8
    pub fn new(len: usize) -> Self {
        NibbleArray {
            data: vec![0u8; len >> 1].into_boxed_slice(),
        }
    }

    /// `new NibbleArray(byte[])`.
    // src: NibbleArray.java:10-12
    pub fn from_bytes(data: Box<[u8]>) -> Self {
        NibbleArray { data }
    }

    /// `getNibble(x, y, z)`.
    // src: NibbleArray.java:14-19
    pub fn get_nibble(&self, x: i32, y: i32, z: i32) -> i32 {
        let var4 = x << 11 | z << 7 | y;
        let var5 = (var4 >> 1) as usize;
        let var6 = var4 & 1;
        if var6 == 0 {
            (self.data[var5] & 15) as i32
        } else {
            (self.data[var5] >> 4 & 15) as i32
        }
    }

    /// `setNibble(x, y, z, value)`: only the low four bits of `value` are kept.
    // src: NibbleArray.java:21-31
    pub fn set_nibble(&mut self, x: i32, y: i32, z: i32, value: i32) {
        let var5 = x << 11 | z << 7 | y;
        let var6 = (var5 >> 1) as usize;
        let var7 = var5 & 1;
        if var7 == 0 {
            self.data[var6] = (self.data[var6] & 240) | (value & 15) as u8;
        } else {
            self.data[var6] = (self.data[var6] & 15) | ((value & 15) << 4) as u8;
        }
    }

    /// `isValid()`: Java checks `data != null`; always true here.
    // src: NibbleArray.java:33-35
    pub fn is_valid(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packing_by_hand() {
        let mut n = NibbleArray::new(32768);
        assert_eq!(n.data.len(), 16384);
        // (x=0,y=0,z=0) -> index 0, low nibble of byte 0
        n.set_nibble(0, 0, 0, 0xA);
        // (x=0,y=1,z=0) -> index 1, high nibble of byte 0
        n.set_nibble(0, 1, 0, 0x5);
        assert_eq!(n.data[0], 0x5A);
        // (x=1,y=3,z=2) -> 1<<11 | 2<<7 | 3 = 2048+256+3 = 2307: byte 1153, high nibble
        n.set_nibble(1, 3, 2, 0x1F); // only 0xF kept
        assert_eq!(n.data[1153], 0xF0);
        assert_eq!(n.get_nibble(1, 3, 2), 15);
        assert_eq!(n.get_nibble(0, 0, 0), 10);
        assert_eq!(n.get_nibble(0, 1, 0), 5);
        // overwrite keeps the other nibble
        n.set_nibble(0, 0, 0, 3);
        assert_eq!(n.data[0], 0x53);
        // last cell (15,127,15) -> 32767: byte 16383 high nibble
        n.set_nibble(15, 127, 15, 9);
        assert_eq!(n.data[16383], 0x90);
    }
}
