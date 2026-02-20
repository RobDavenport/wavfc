//! Generic fixed-size bitset for tracking possible states.
//!
//! `BitSet<W>` is implemented as `[u64; W]` and supports up to `W * 64` states.
//! All operations are branchless where practical.

use core::ops::{BitAndAssign, BitOrAssign, Not};

/// A fixed-size bitset supporting up to `W * 64` bits.
///
/// Internally stored as `[u64; W]`. Bits `0..63` are in `words[0]`,
/// bits `64..127` are in `words[1]`, etc.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BitSet<const W: usize> {
    words: [u64; W],
}

/// Alias for 128-bit (2-word) bitset -- the original default.
pub type BitSet128 = BitSet<2>;

/// Alias for 256-bit (4-word) bitset.
pub type BitSet256 = BitSet<4>;

impl<const W: usize> Default for BitSet<W> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const W: usize> BitSet<W> {
    /// Maximum number of bits supported.
    pub const MAX_BITS: usize = W * 64;

    /// Creates a new empty bitset with all bits cleared.
    #[inline]
    pub fn new() -> Self {
        Self { words: [0; W] }
    }

    /// Creates a bitset with the first `n` bits set (0-indexed).
    ///
    /// # Panics
    ///
    /// Panics if `n > W * 64`.
    #[inline]
    pub fn full(n: usize) -> Self {
        assert!(n <= Self::MAX_BITS, "n must be <= {}", Self::MAX_BITS);
        let mut words = [0u64; W];
        let full_words = n / 64;
        let remaining_bits = n % 64;

        let mut i = 0;
        while i < full_words {
            words[i] = u64::MAX;
            i += 1;
        }
        if remaining_bits > 0 && full_words < W {
            words[full_words] = (1u64 << remaining_bits) - 1;
        }

        Self { words }
    }

    /// Sets the bit at position `bit`.
    ///
    /// # Panics
    ///
    /// Panics if `bit >= W * 64`.
    #[inline]
    pub fn set(&mut self, bit: usize) {
        assert!(bit < Self::MAX_BITS, "bit index out of range");
        let word = bit >> 6; // bit / 64
        let pos = bit & 63; // bit % 64
        self.words[word] |= 1u64 << pos;
    }

    /// Clears the bit at position `bit`.
    ///
    /// # Panics
    ///
    /// Panics if `bit >= W * 64`.
    #[inline]
    pub fn clear(&mut self, bit: usize) {
        assert!(bit < Self::MAX_BITS, "bit index out of range");
        let word = bit >> 6;
        let pos = bit & 63;
        self.words[word] &= !(1u64 << pos);
    }

    /// Tests whether the bit at position `bit` is set.
    ///
    /// # Panics
    ///
    /// Panics if `bit >= W * 64`.
    #[inline]
    pub fn test(&self, bit: usize) -> bool {
        assert!(bit < Self::MAX_BITS, "bit index out of range");
        let word = bit >> 6;
        let pos = bit & 63;
        (self.words[word] >> pos) & 1 != 0
    }

    /// Returns the number of set bits (population count).
    #[inline]
    pub fn count_ones(&self) -> u32 {
        let mut count = 0u32;
        let mut i = 0;
        while i < W {
            count += self.words[i].count_ones();
            i += 1;
        }
        count
    }

    /// Returns `true` if no bits are set.
    #[inline]
    pub fn is_empty(&self) -> bool {
        let mut i = 0;
        while i < W {
            if self.words[i] != 0 {
                return false;
            }
            i += 1;
        }
        true
    }

    /// Returns `true` if exactly one bit is set.
    #[inline]
    pub fn is_singleton(&self) -> bool {
        self.count_ones() == 1
    }

    /// Returns the index of the lowest set bit, or `None` if empty.
    #[inline]
    pub fn first_one(&self) -> Option<usize> {
        let mut i = 0;
        while i < W {
            if self.words[i] != 0 {
                return Some(i * 64 + self.words[i].trailing_zeros() as usize);
            }
            i += 1;
        }
        None
    }

    /// Returns an iterator over the indices of all set bits, from lowest to highest.
    #[inline]
    pub fn iter_ones(&self) -> IterOnes<W> {
        IterOnes {
            words: self.words,
            word_index: 0,
        }
    }

    /// Clears all bits.
    #[inline]
    pub fn clear_all(&mut self) {
        self.words = [0; W];
    }

    /// Returns `self & !other` (bits set in self but not in other).
    #[inline]
    pub fn and_not(&self, other: &Self) -> Self {
        let mut words = [0u64; W];
        let mut i = 0;
        while i < W {
            words[i] = self.words[i] & !other.words[i];
            i += 1;
        }
        Self { words }
    }
}

impl<const W: usize> BitOrAssign for BitSet<W> {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        let mut i = 0;
        while i < W {
            self.words[i] |= rhs.words[i];
            i += 1;
        }
    }
}

impl<const W: usize> BitAndAssign for BitSet<W> {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        let mut i = 0;
        while i < W {
            self.words[i] &= rhs.words[i];
            i += 1;
        }
    }
}

impl<const W: usize> Not for BitSet<W> {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        let mut words = [0u64; W];
        let mut i = 0;
        while i < W {
            words[i] = !self.words[i];
            i += 1;
        }
        Self { words }
    }
}

/// Iterator over the set bits of a [`BitSet`].
pub struct IterOnes<const W: usize> {
    words: [u64; W],
    word_index: usize,
}

impl<const W: usize> Iterator for IterOnes<W> {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<usize> {
        loop {
            if self.word_index >= W {
                return None;
            }
            let w = self.words[self.word_index];
            if w == 0 {
                self.word_index += 1;
                continue;
            }
            let tz = w.trailing_zeros() as usize;
            let bit_index = self.word_index * 64 + tz;
            // Clear the lowest set bit
            self.words[self.word_index] = w & (w - 1);
            return Some(bit_index);
        }
    }
}
