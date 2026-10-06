//! `RegionFile` and `RegionFileChunkBuffer` (RegionFile.java,
//! RegionFileChunkBuffer.java): one McRegion file `r.<x>.<z>.mcr` holding up
//! to 32×32 chunks (PLAN_v4 D5).
//!
//! # Layout (byte-compatible with the Java, so vanilla and Prism read it)
//!
//! The file is a sequence of 4096-byte sectors. Sector 0 is the offset table:
//! 1024 big-endian ints, entry `x + z * 32` = `sector << 8 | sectorCount`, 0
//! for an absent chunk (RegionFile.java:67-75, :251-263). Sector 1 is the
//! timestamp table: 1024 big-endian ints, the second at which the chunk was
//! last written (RegionFile.java:77-80, :232, :265-269). A chunk entry starts
//! at its first sector: a big-endian int `length` (the compressed bytes plus
//! one), a version byte (1 = gzip, 2 = zlib; vanilla writes only 2), then
//! `length - 1` compressed bytes (RegionFile.java:128-150, :239-245). Bytes
//! after the entry up to the sector boundary are whatever was there before
//! (vanilla never clears them; neither do we).
//!
//! # Storage model (PLAN_v4 D9)
//!
//! [`Storage`] is a whole-blob interface, so a [`RegionFile`] holds the whole
//! file in memory (`data`) and stands in for the Java `RandomAccessFile`:
//! every seek/read/write of the Java is the same operation on `data`, writes
//! past the end extend it with zeros. The blob is written back through
//! [`Storage::write`] wherever the Java's writes reach the disk: once at the
//! end of [`RegionFile::open`] when the constructor changed the file (a new
//! file's empty header, the padding loop), and once at the end of every
//! [`RegionFile::write`] that changed it. The on-disk result after each call is
//! therefore byte-identical to what the Java leaves (up to the deflate output,
//! which differs between `java.util.zip` and `miniz_oxide`, PLAN_v4 §4.3 Tier
//! S). Cost: a chunk save rewrites the whole region file (up to a few MB);
//! Phase 4 revisits incremental writes (D4 table, `anvil.rs` row). While a
//! [`RegionFile`] is open its in-memory copy is authoritative, as the Java's
//! open `RandomAccessFile` is for this process.
//!
//! # Errors
//!
//! The Java catches `IOException` inside the class and either returns `null`
//! (reads) or prints it and carries on (constructor, `write`). The port maps
//! "returns null" to `Ok(None)`. Failures the Java cannot have here (the
//! [`Storage`] backend) and Java runtime exceptions on corrupt files, which
//! escape the class (`IndexOutOfBoundsException`,
//! `NegativeArraySizeException`; `McRegionChunkLoader.saveChunk` catches them,
//! `loadChunk` does not), are [`RegionError`]s. Decompression is done eagerly
//! (the Java decompresses lazily while `CompressedStreamTools.read` pulls
//! bytes), so a corrupt deflate stream is an error here, where the Java throws
//! the same `IOException` from the NBT read (`loadChunk` propagates it). The
//! one observable difference: damage *after* the end of the NBT document
//! (e.g. a wrong zlib Adler-32 trailer) is never read by the Java and is an
//! error here.
//!
//! # MinecraftOSS
//!
//! PLAN_v4 D4 allows adapting MinecraftOSS `core/src/anvil.rs`. It was read and
//! nothing was copied: it repacks every chunk from sector 2 on each save and
//! reads leniently (no `sectorFree` bitmap, no first-fit reuse, no in-place
//! rewrite, no constructor padding), none of which matches RegionFile.java.
//! Everything here is transcribed from the Java.
//!
//! # Debug output
//!
//! `debugln`/`func_22199_a`/`func_22197_a` build messages for `func_22211_a`,
//! whose body is empty (RegionFile.java:93-110), so they are not ported. The
//! write path's three outcomes they name ("rewrite", "reuse", "grow") are
//! returned as [`RegionWrite`] instead.

use std::io;

use super::storage::{Storage, StorageError};

/// Bytes per sector. src: RegionFile.java:15 (`emptySector`), :128, :170, :241
pub const SECTOR_BYTES: usize = 4096;

/// Entries in each header table (32×32 chunks). src: RegionFile.java:18-19
pub const REGION_ENTRIES: usize = 1024;

/// A write needing this many sectors or more is dropped (the sector count
/// must fit in the offset entry's low byte). src: RegionFile.java:171-173
pub const MAX_CHUNK_SECTORS: i32 = 256;

/// Why a region operation failed (module docs, "Errors").
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegionError {
    /// The [`Storage`] backend failed.
    Storage(StorageError),
    /// The Java would throw a runtime exception out of `RegionFile` here (a
    /// corrupt file); the message names it.
    Corrupt(String),
    /// The chunk's compressed bytes do not decompress (the Java's
    /// `IOException` from the stream during the NBT read).
    Compression(String),
    /// [`RegionFile::write`] called with local coordinates outside 0..32. The
    /// Java's `write` is `protected` and only reached through a bounds-checked
    /// `getChunkDataOutputStream` (RegionFile.java:161-163).
    OutOfBounds {
        /// Local chunk x.
        x: i32,
        /// Local chunk z.
        z: i32,
    },
    /// A [`ChunkDataOutputStream`] was closed through a
    /// [`super::region_cache::RegionFileCache`] that no longer holds its
    /// region (cleared in between). The Java writes into the closed
    /// `RandomAccessFile`, which throws `IOException` on the first seek; `write`
    /// prints it and nothing is saved (RegionFile.java:233-235).
    StaleStream(String),
}

impl std::fmt::Display for RegionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegionError::Storage(e) => write!(f, "region storage: {e}"),
            RegionError::Corrupt(m) => write!(f, "corrupt region file: {m}"),
            RegionError::Compression(m) => write!(f, "region chunk decompression: {m}"),
            RegionError::OutOfBounds { x, z } => {
                write!(f, "region chunk [{x},{z}] out of bounds")
            }
            RegionError::StaleStream(p) => {
                write!(f, "chunk output stream for closed region {p}")
            }
        }
    }
}

impl std::error::Error for RegionError {}

impl From<StorageError> for RegionError {
    fn from(e: StorageError) -> Self {
        RegionError::Storage(e)
    }
}

/// Which branch of `RegionFile.write(int, int, byte[], int)` ran (the strings
/// its debug call would print, RegionFile.java:176, :207, :217).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionWrite {
    /// Same sector count as stored: written in place, offset entry unchanged.
    Rewrite,
    /// Old sectors freed, a first-fit run of free sectors reused.
    Reuse,
    /// No run long enough: new sectors appended at the end of the file.
    Grow,
    /// `>= 256` sectors needed: nothing written, timestamp unchanged
    /// (RegionFile.java:171-173). The chunk is silently not saved.
    TooLarge,
}

/// `RegionFile` (RegionFile.java:14). Field names follow the Java;
/// `field_22217_e` is [`RegionFile::chunk_timestamps`] (docs/mappings.md).
/// `fileName`'s `lastModified()` (`field_22214_h`, RegionFile.java:22, :30-32)
/// is never read and is not ported (D9: no clocks or file metadata).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegionFile {
    /// `fileName`: the storage path.
    file_name: String,
    /// `dataFile`: the whole file (module docs, "Storage model").
    data: Vec<u8>,
    /// `offsets`, index `x + z * 32`.
    offsets: [i32; REGION_ENTRIES],
    /// `field_22217_e`: the timestamp table, index `x + z * 32`.
    chunk_timestamps: [i32; REGION_ENTRIES],
    /// `sectorFree`: one flag per whole sector of the file at open, then
    /// appended to by growth (RegionFile.java:54-63, :223).
    sector_free: Vec<bool>,
    /// `sizeDelta`: bytes the file grew by since the last
    /// [`RegionFile::take_size_delta`].
    size_delta: i32,
}

impl RegionFile {
    /// `new RegionFile(File)` (RegionFile.java:24-85). A missing file is
    /// created (`RandomAccessFile(file, "rw")`, :34).
    ///
    /// Errors: the storage backend, or what the Java constructor would throw
    /// past its `IOException` handler on a corrupt file: a file of 4096..8191
    /// bytes (`sectorFree.set(1, false)` on a one-element list, :62-63), an
    /// offset entry with a negative sector (:70-72), a length beyond `i32`
    /// (`new ArrayList(negative)`, :54-55).
    pub fn open(storage: &mut dyn Storage, path: &str) -> Result<Self, RegionError> {
        let existing = storage.read(path)?;
        // `RandomAccessFile(.., "rw")` creates the (empty) file. src: RegionFile.java:34
        let mut dirty = existing.is_none();
        let mut rf = RegionFile {
            file_name: path.to_string(),
            data: existing.unwrap_or_default(),
            offsets: [0; REGION_ENTRIES],
            chunk_timestamps: [0; REGION_ENTRIES],
            sector_free: Vec::new(),
            size_delta: 0,
        };
        // The file pointer starts at 0.
        let mut pos: usize = 0;
        // src: RegionFile.java:36-46: an empty header (both tables) over
        // whatever the first 8 KiB were.
        if rf.data.len() < SECTOR_BYTES {
            for _ in 0..REGION_ENTRIES * 2 {
                rf.write_at(pos, &0i32.to_be_bytes());
                pos += 4;
            }
            rf.size_delta = rf.size_delta.wrapping_add(8192);
            dirty = true;
        }
        // src: RegionFile.java:48-52.
        // vanilla bug: meant to pad the file to a sector multiple, it writes
        // `length & 4095` zero bytes at the file pointer, which is still 0 for
        // a file that was not just created, so it zeroes the first
        // `length & 4095` bytes of the offset table (the chunks of those
        // entries are lost) and the length stays unaligned. Writing inside the file never changes its
        // length, so the Java's per-iteration `length()` is this constant.
        let unaligned = rf.data.len() & (SECTOR_BYTES - 1);
        if unaligned != 0 {
            for _ in 0..unaligned {
                rf.write_at(pos, &[0]);
                pos += 1;
            }
            dirty = true;
        }
        if dirty {
            storage.write(path, &rf.data)?;
        }

        // src: RegionFile.java:54: `(int)length() / 4096`, the cast first.
        let sectors = (rf.data.len() as i64 as i32) / SECTOR_BYTES as i32;
        if sectors < 0 {
            return Err(RegionError::Corrupt(format!(
                "IllegalArgumentException: Illegal Capacity: {sectors} ({path})"
            )));
        }
        rf.sector_free = vec![true; sectors as usize];
        // src: RegionFile.java:62-63 (the two header sectors)
        if sectors < 2 {
            return Err(RegionError::Corrupt(format!(
                "IndexOutOfBoundsException: sectorFree.set(1) with {sectors} sector(s) ({path})"
            )));
        }
        rf.sector_free[0] = false;
        rf.sector_free[1] = false;

        // src: RegionFile.java:64-75: mark the sectors of every entry that
        // fits in the file as used; entries reaching past the end are kept in
        // `offsets` but their sectors stay free.
        for i in 0..REGION_ENTRIES {
            let v = rf.read_int(i * 4);
            rf.offsets[i] = v;
            if v != 0 && (v >> 8) + (v & 255) <= sectors {
                for k in 0..(v & 255) {
                    rf.set_sector_free((v >> 8) + k, false)?;
                }
            }
        }
        // src: RegionFile.java:77-80
        for i in 0..REGION_ENTRIES {
            rf.chunk_timestamps[i] = rf.read_int(SECTOR_BYTES + i * 4);
        }
        Ok(rf)
    }

    /// `func_22209_a()` (RegionFile.java:87-91): the bytes the file grew by
    /// since the last call, resetting the count. `RegionFileCache.getSizeDelta`
    /// returns it; `McRegionChunkLoader.saveChunk` adds it to
    /// `WorldInfo.sizeOnDisk` (McRegionChunkLoader.java:54).
    pub fn take_size_delta(&mut self) -> i32 {
        std::mem::take(&mut self.size_delta)
    }

    /// `getChunkDataInputStream(x, z)` (RegionFile.java:112-159), local
    /// coordinates: the chunk's NBT bytes, decompressed, or `None` where the
    /// Java returns `null` (out of bounds, absent, entry past the end of the
    /// file, stored length too big for its sectors, unknown version, an
    /// unreadable gzip header, a read past the end of the file).
    ///
    /// Errors: `Corrupt` for a stored length of 0 with version 1 or 2 (the
    /// Java's `new byte[-1]`, :138, :143), `Compression` for a stream that does
    /// not decompress (module docs, "Errors").
    pub fn get_chunk_data_input_stream(
        &self,
        x: i32,
        z: i32,
    ) -> Result<Option<Vec<u8>>, RegionError> {
        // src: RegionFile.java:113-115
        if out_of_bounds(x, z) {
            return Ok(None);
        }
        // src: RegionFile.java:118-127
        let offset = self.get_offset(x, z);
        if offset == 0 {
            return Ok(None);
        }
        let sector = offset >> 8;
        let count = offset & 255;
        if sector + count > self.sector_free.len() as i32 {
            return Ok(None);
        }
        // src: RegionFile.java:128: `(long)(sector * 4096)`, an int product.
        // A negative position makes `seek` throw `IOException` (caught: null).
        let start = sector.wrapping_mul(SECTOR_BYTES as i32);
        if start < 0 {
            return Ok(None);
        }
        let start = start as usize;
        // src: RegionFile.java:129: `readInt`; EOF is an `IOException` (null).
        let Some(length) = self.try_read_int(start) else {
            return Ok(None);
        };
        // src: RegionFile.java:130-132
        if i64::from(length) > SECTOR_BYTES as i64 * i64::from(count) {
            return Ok(None);
        }
        // src: RegionFile.java:134
        let Some(&version) = self.data.get(start + 4) else {
            return Ok(None);
        };
        let version = version as i8;
        if version != 1 && version != 2 {
            // src: RegionFile.java:147-149: unknown version
            return Ok(None);
        }
        // src: RegionFile.java:138, :143: `new byte[length - 1]`
        if length < 1 {
            return Err(RegionError::Corrupt(format!(
                "NegativeArraySizeException: chunk [{x},{z}] of {} has stored length {length}",
                self.file_name
            )));
        }
        // src: RegionFile.java:139, :144: `RandomAccessFile.read(byte[])` reads
        // what is there; a short read at the end of the file leaves zeros.
        let mut body = vec![0u8; (length - 1) as usize];
        let from = (start + 5).min(self.data.len());
        let avail = (self.data.len() - from).min(body.len());
        body[..avail].copy_from_slice(&self.data[from..from + avail]);
        if version == 1 {
            // src: RegionFile.java:140: `GZIPInputStream`'s constructor reads
            // the header; a bad or truncated one is an `IOException` (null).
            if !gzip_header_ok(&body) {
                return Ok(None);
            }
            mc_nbt::decompress_gzip(&body)
                .map(Some)
                .map_err(|e| RegionError::Compression(e.to_string()))
        } else {
            // src: RegionFile.java:145: `InflaterInputStream`
            mc_nbt::decompress_zlib(&body)
                .map(Some)
                .map_err(|e| RegionError::Compression(e.to_string()))
        }
    }

    /// `getChunkDataOutputStream(x, z)` (RegionFile.java:161-163), local
    /// coordinates: `None` out of bounds, else a stream that compresses what is
    /// written to it and stores it on [`ChunkDataOutputStream::close`].
    pub fn get_chunk_data_output_stream(&self, x: i32, z: i32) -> Option<ChunkDataOutputStream> {
        if out_of_bounds(x, z) {
            None
        } else {
            Some(ChunkDataOutputStream {
                region_path: self.file_name.clone(),
                buffer: RegionFileChunkBuffer::new(x, z),
                uncompressed: Vec::new(),
            })
        }
    }

    /// `write(x, z, buf, count)` (RegionFile.java:165-237): stores `data`, an
    /// already compressed (zlib) chunk body, for local chunk `(x, z)`, then
    /// writes the file back through `storage`. `now_millis` is
    /// `System.currentTimeMillis()` (D9: passed in); the timestamp table gets
    /// `(int)(now_millis / 1000)` (:232).
    ///
    /// Errors: storage; `OutOfBounds` (see [`RegionError::OutOfBounds`]);
    /// `Corrupt` where the Java would throw `IndexOutOfBoundsException` (the
    /// chunk's old entry reaches past the sectors known at open, :180-182).
    pub fn write(
        &mut self,
        storage: &mut dyn Storage,
        x: i32,
        z: i32,
        data: &[u8],
        now_millis: i64,
    ) -> Result<RegionWrite, RegionError> {
        if out_of_bounds(x, z) {
            return Err(RegionError::OutOfBounds { x, z });
        }
        let len = data.len() as i32;
        // src: RegionFile.java:167-170
        let offset = self.get_offset(x, z);
        let mut sector = offset >> 8;
        let old_count = offset & 255;
        // vanilla quirk: `(len + 5) / 4096 + 1` takes one sector more than
        // needed when `len + 5` is a multiple of 4096.
        let count = len.wrapping_add(5) / SECTOR_BYTES as i32 + 1;
        // src: RegionFile.java:171-173
        if count >= MAX_CHUNK_SECTORS {
            return Ok(RegionWrite::TooLarge);
        }

        let outcome;
        if sector != 0 && old_count == count {
            // src: RegionFile.java:175-177: same size, in place
            outcome = RegionWrite::Rewrite;
            self.write_sector(sector, data)?;
        } else {
            // src: RegionFile.java:180-182: free the old sectors.
            // vanilla bug: a corrupt entry pointing into sectors 0-1 frees the
            // header sectors, which a later write may then reuse.
            for k in 0..old_count {
                self.set_sector_free(sector + k, true)?;
            }
            // src: RegionFile.java:184-204: first fit. `run` counts the free
            // sectors from `start`; the scan stops at the first long enough run.
            let mut start = self
                .sector_free
                .iter()
                .position(|&f| f)
                .map_or(-1, |i| i as i32);
            let mut run = 0;
            if start != -1 {
                for i in start as usize..self.sector_free.len() {
                    if run != 0 {
                        if self.sector_free[i] {
                            run += 1;
                        } else {
                            run = 0;
                        }
                    } else if self.sector_free[i] {
                        start = i as i32;
                        run = 1;
                    }
                    if run >= count {
                        break;
                    }
                }
            }

            if run >= count {
                // src: RegionFile.java:206-215
                outcome = RegionWrite::Reuse;
                sector = start;
                self.set_offset(x, z, start << 8 | count);
                for k in 0..count {
                    self.sector_free[(sector + k) as usize] = false;
                }
                self.write_sector(sector, data)?;
            } else {
                // src: RegionFile.java:216-229: append `count` empty sectors at
                // the end of the file; the chunk goes at `sectorFree.size()`
                // (which is below the end when the file length is unaligned,
                // see `open`).
                outcome = RegionWrite::Grow;
                let mut pos = self.data.len();
                sector = self.sector_free.len() as i32;
                for _ in 0..count {
                    self.write_at(pos, &[0u8; SECTOR_BYTES]);
                    pos += SECTOR_BYTES;
                    self.sector_free.push(false);
                }
                self.size_delta = self
                    .size_delta
                    .wrapping_add((SECTOR_BYTES as i32).wrapping_mul(count));
                self.write_sector(sector, data)?;
                self.set_offset(x, z, sector << 8 | count);
            }
        }

        // src: RegionFile.java:232: `(int)(System.currentTimeMillis() / 1000L)`
        self.set_chunk_timestamp(x, z, (now_millis / 1000) as i32);
        storage.write(&self.file_name, &self.data)?;
        Ok(outcome)
    }

    /// `func_22202_c(x, z)` (RegionFile.java:255-257): whether the offset entry
    /// is non-zero. Like the Java, no bounds check on `x`/`z`; an index outside
    /// the table (the Java's `ArrayIndexOutOfBoundsException`) is `false`.
    pub fn has_chunk(&self, x: i32, z: i32) -> bool {
        let i = x.wrapping_add(z.wrapping_mul(32));
        usize::try_from(i)
            .ok()
            .and_then(|i| self.offsets.get(i))
            .is_some_and(|&v| v != 0)
    }

    /// `func_22196_b()` (RegionFile.java:271-273): `dataFile.close()`. Every
    /// write already reached the storage, so closing only drops the copy.
    pub fn close(self) {}

    /// `fileName`: the storage path this region was opened from.
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// The whole file as it is stored.
    pub fn bytes(&self) -> &[u8] {
        &self.data
    }

    /// `offsets`, index `x + z * 32`: `sector << 8 | sectorCount`.
    pub fn offsets(&self) -> &[i32; REGION_ENTRIES] {
        &self.offsets
    }

    /// `field_22217_e`, index `x + z * 32`: the second each chunk was last
    /// written, 0 if never.
    pub fn chunk_timestamps(&self) -> &[i32; REGION_ENTRIES] {
        &self.chunk_timestamps
    }

    /// `sectorFree`: `true` for a free sector.
    pub fn sector_free(&self) -> &[bool] {
        &self.sector_free
    }

    /// `sizeDelta` without resetting it.
    pub fn size_delta(&self) -> i32 {
        self.size_delta
    }

    // -- private, as in the Java --------------------------------------------

    /// `write(sector, buf, count)` (RegionFile.java:239-245): length + 1,
    /// version 2 (zlib), the bytes. Nothing pads the rest of the sector.
    ///
    /// A negative position (a corrupt entry rewritten in place) is the Java's
    /// `IOException` from `seek`, which `write` prints before returning with
    /// nothing written (:233-235); here it is an error.
    fn write_sector(&mut self, sector: i32, data: &[u8]) -> Result<(), RegionError> {
        // src: RegionFile.java:241: `(long)(sector * 4096)`, an int product.
        let pos = sector.wrapping_mul(SECTOR_BYTES as i32);
        let Ok(pos) = usize::try_from(pos) else {
            return Err(RegionError::Corrupt(format!(
                "IOException: Negative seek offset {pos} ({})",
                self.file_name
            )));
        };
        self.write_at(pos, &(data.len() as i32).wrapping_add(1).to_be_bytes());
        self.write_at(pos + 4, &[2]);
        self.write_at(pos + 5, data);
        Ok(())
    }

    /// `getOffset(x, z)` (RegionFile.java:251-253).
    fn get_offset(&self, x: i32, z: i32) -> i32 {
        self.offsets[(x + z * 32) as usize]
    }

    /// `setOffset(x, z, v)` (RegionFile.java:259-263): table and file.
    fn set_offset(&mut self, x: i32, z: i32, v: i32) {
        let i = (x + z * 32) as usize;
        self.offsets[i] = v;
        self.write_at(i * 4, &v.to_be_bytes());
    }

    /// `func_22208_b(x, z, t)` (RegionFile.java:265-269): table and file.
    fn set_chunk_timestamp(&mut self, x: i32, z: i32, t: i32) {
        let i = (x + z * 32) as usize;
        self.chunk_timestamps[i] = t;
        self.write_at(SECTOR_BYTES + i * 4, &t.to_be_bytes());
    }

    /// `sectorFree.set(i, free)`; out of range is the Java's
    /// `IndexOutOfBoundsException`.
    fn set_sector_free(&mut self, i: i32, free: bool) -> Result<(), RegionError> {
        match usize::try_from(i)
            .ok()
            .and_then(|i| self.sector_free.get_mut(i))
        {
            Some(slot) => {
                *slot = free;
                Ok(())
            }
            None => Err(RegionError::Corrupt(format!(
                "IndexOutOfBoundsException: sectorFree.set({i}) with {} sectors ({})",
                self.sector_free.len(),
                self.file_name
            ))),
        }
    }

    /// `RandomAccessFile` write at `pos`: overwrites, extending the file with
    /// zeros when it reaches past the end.
    fn write_at(&mut self, pos: usize, bytes: &[u8]) {
        let end = pos + bytes.len();
        if self.data.len() < end {
            self.data.resize(end, 0);
        }
        self.data[pos..end].copy_from_slice(bytes);
    }

    /// `readInt` at `pos` where the file is known to be long enough.
    fn read_int(&self, pos: usize) -> i32 {
        self.try_read_int(pos).unwrap_or(0)
    }

    /// `readInt` at `pos`; `None` at end of file (`EOFException`).
    fn try_read_int(&self, pos: usize) -> Option<i32> {
        let b = self.data.get(pos..pos + 4)?;
        Some(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
}

/// `outOfBounds(x, z)` (RegionFile.java:247-249).
fn out_of_bounds(x: i32, z: i32) -> bool {
    !(0..32).contains(&x) || !(0..32).contains(&z)
}

/// Whether `java.util.zip.GZIPInputStream`'s constructor accepts the header
/// of `b` (its `readHeader`): magic `1f 8b`, method 8, then the optional
/// FEXTRA / FNAME / FCOMMENT fields present and FHCRC matching. Anything else
/// is a `ZipException` or `EOFException`, which `getChunkDataInputStream`
/// turns into `null` (RegionFile.java:154-157).
fn gzip_header_ok(b: &[u8]) -> bool {
    const FHCRC: u8 = 2;
    const FEXTRA: u8 = 4;
    const FNAME: u8 = 8;
    const FCOMMENT: u8 = 16;
    if b.len() < 10 || b[0] != 0x1f || b[1] != 0x8b || b[2] != 8 {
        return false;
    }
    let flags = b[3];
    let mut pos = 10;
    if flags & FEXTRA != 0 {
        let Some(n) = b.get(pos..pos + 2) else {
            return false;
        };
        pos += 2 + usize::from(u16::from_le_bytes([n[0], n[1]]));
        if pos > b.len() {
            return false;
        }
    }
    for flag in [FNAME, FCOMMENT] {
        if flags & flag != 0 {
            match b[pos.min(b.len())..].iter().position(|&c| c == 0) {
                Some(i) => pos += i + 1,
                None => return false,
            }
        }
    }
    if flags & FHCRC != 0 {
        let Some(c) = b.get(pos..pos + 2) else {
            return false;
        };
        let want = u16::from_le_bytes([c[0], c[1]]);
        if (crc32(&b[..pos]) & 0xffff) as u16 != want {
            return false;
        }
    }
    true
}

/// CRC-32 (IEEE, reflected), bitwise; only for the rare gzip FHCRC check.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in bytes {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// `RegionFileChunkBuffer` (RegionFileChunkBuffer.java): the compressed
/// bytes of one chunk, handed to [`RegionFile::write`] on close. The Java's
/// initial capacity of 8096 (:12) has no observable effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegionFileChunkBuffer {
    /// `field_22283_b`: local chunk x.
    chunk_x: i32,
    /// `field_22285_c`: local chunk z.
    chunk_z: i32,
    /// `buf[0..count]`.
    buf: Vec<u8>,
}

impl RegionFileChunkBuffer {
    /// `new RegionFileChunkBuffer(region, x, z)` (RegionFileChunkBuffer.java:11-16).
    /// The region (`field_22284_a`) is passed to [`Self::close`] instead.
    pub fn new(chunk_x: i32, chunk_z: i32) -> Self {
        RegionFileChunkBuffer {
            chunk_x,
            chunk_z,
            buf: Vec::new(),
        }
    }

    /// `close()` (RegionFileChunkBuffer.java:18-20):
    /// `region.write(x, z, buf, count)`.
    pub fn close(
        self,
        region: &mut RegionFile,
        storage: &mut dyn Storage,
        now_millis: i64,
    ) -> Result<RegionWrite, RegionError> {
        region.write(storage, self.chunk_x, self.chunk_z, &self.buf, now_millis)
    }
}

impl io::Write for RegionFileChunkBuffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// What `getChunkDataOutputStream` returns:
/// `DataOutputStream(DeflaterOutputStream(RegionFileChunkBuffer))`
/// (RegionFile.java:162). Collects the uncompressed chunk NBT; [`Self::close`]
/// deflates it (zlib, default level, like `new Deflater()`) into the buffer
/// and closes that, which stores the chunk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkDataOutputStream {
    region_path: String,
    buffer: RegionFileChunkBuffer,
    uncompressed: Vec<u8>,
}

impl ChunkDataOutputStream {
    /// The storage path of the region this stream writes into.
    pub fn region_path(&self) -> &str {
        &self.region_path
    }

    /// Local chunk coordinates `(x, z)`.
    pub fn chunk(&self) -> (i32, i32) {
        (self.buffer.chunk_x, self.buffer.chunk_z)
    }

    /// `close()`: `DeflaterOutputStream.finish()` then
    /// `RegionFileChunkBuffer.close()` into `region`.
    pub fn close(
        mut self,
        region: &mut RegionFile,
        storage: &mut dyn Storage,
        now_millis: i64,
    ) -> Result<RegionWrite, RegionError> {
        self.buffer.buf = mc_nbt::compress_zlib(&self.uncompressed);
        self.buffer.close(region, storage, now_millis)
    }
}

impl io::Write for ChunkDataOutputStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.uncompressed.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_check_value() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }

    #[test]
    fn gzip_header() {
        let gz = mc_nbt::compress_gzip(b"x");
        assert!(gzip_header_ok(&gz));
        assert!(!gzip_header_ok(&gz[..9]));
        assert!(!gzip_header_ok(&mc_nbt::compress_zlib(b"x")));
        // FNAME without its terminator
        let mut named = gz[..10].to_vec();
        named[3] = 8;
        named.extend_from_slice(b"name");
        assert!(!gzip_header_ok(&named));
        named.push(0);
        assert!(gzip_header_ok(&named));
        // FHCRC
        let mut h = gz[..10].to_vec();
        h[3] = 2;
        let c = (crc32(&h) & 0xffff) as u16;
        h.extend_from_slice(&c.to_le_bytes());
        assert!(gzip_header_ok(&h));
        let n = h.len();
        h[n - 1] ^= 1;
        assert!(!gzip_header_ok(&h));
    }
}
