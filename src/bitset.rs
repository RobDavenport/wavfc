//! Fixed-size 128-bit bitset for tracking possible states.
//!
//! `BitSet128` is implemented as `[u64; 2]` and supports up to 128 states.
//! All operations are branchless where practical.

use core::ops::{BitAndAssign, BitOrAssign, Not};

/// A fixed-size bitset supporting up to 128 bits.
///
/// Internally stored as `[u64; 2]`. Bits 0..63 are in `words[0]`,
/// bits 64..127 are in `words[1]`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct BitSet128 {
    words: [u64; 2],
}

impl BitSet128 {
    /// Maximum number of bits supported.
    pub const MAX_BITS: usize = 128;

    /// Creates a new empty bitset with all bits cleared.
    #[inline]
    pub fn new() -> Self {
        Self { words: [0, 0] }
    }

    /// Creates a bitset with the first `n` bits set (0-indexed).
    ///
    /// # Panics
    ///
    /// Panics if `n > 128`.
    #[inline]
    pub fn full(n: usize) -> Self {
        assert!(n <= Self::MAX_BITS, "n must be <= 128");
        if n == 0 {
            return Self::new();
        }
        let lo = if n >= 64 { u64::MAX } else { (1u64 << n) - 1 };
        let hi = if n <= 64 {
            0
        } else if n == 128 {
            u64::MAX
        } else {
            (1u64 << (n - 64)) - 1
        };
        Self { words: [lo, hi] }
    }

    /// Sets the bit at position `bit`.
    ///
    /// # Panics
    ///
    /// Panics if `bit >= 128`.
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
    /// Panics if `bit >= 128`.
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
    /// Panics if `bit >= 128`.
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
        self.words[0].count_ones() + self.words[1].count_ones()
    }

    /// Returns `true` if no bits are set.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.words[0] == 0 && self.words[1] == 0
    }

    /// Returns `true` if exactly one bit is set.
    #[inline]
    pub fn is_singleton(&self) -> bool {
        self.count_ones() == 1
    }

    /// Returns the index of the lowest set bit, or `None` if empty.
    #[inline]
    pub fn first_one(&self) -> Option<usize> {
        if self.words[0] != 0 {
            Some(self.words[0].trailing_zeros() as usize)
        } else if self.words[1] != 0 {
            Some(64 + self.words[1].trailing_zeros() as usize)
        } else {
            None
        }
    }

    /// Returns an iterator over the indices of all set bits, from lowest to highest.
    #[inline]
    pub fn iter_ones(&self) -> IterOnes {
        IterOnes {
            words: self.words,
            word_index: 0,
        }
    }

    /// Clears all bits.
    #[inline]
    pub fn clear_all(&mut self) {
        self.words = [0, 0];
    }

    /// Returns `self & !other` (bits set in self but not in other).
    #[inline]
    pub fn and_not(&self, other: &Self) -> Self {
        Self {
            words: [
                self.words[0] & !other.words[0],
                self.words[1] & !other.words[1],
            ],
        }
    }
}

impl BitOrAssign for BitSet128 {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.words[0] |= rhs.words[0];
        self.words[1] |= rhs.words[1];
    }
}

impl BitAndAssign for BitSet128 {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        self.words[0] &= rhs.words[0];
        self.words[1] &= rhs.words[1];
    }
}

impl Not for BitSet128 {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        Self {
            words: [!self.words[0], !self.words[1]],
        }
    }
}

/// Iterator over the set bits of a [`BitSet128`].
pub struct IterOnes {
    words: [u64; 2],
    word_index: usize,
}

impl Iterator for IterOnes {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<usize> {
        loop {
            if self.word_index >= 2 {
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
