use wavfc::BitSet128;

// ---- new() ----

#[test]
fn new_creates_empty_set() {
    let bs = BitSet128::new();
    assert!(bs.is_empty());
    assert_eq!(bs.count_ones(), 0);
}

// ---- full() ----

#[test]
fn full_zero_is_empty() {
    let bs = BitSet128::full(0);
    assert!(bs.is_empty());
    assert_eq!(bs.count_ones(), 0);
}

#[test]
fn full_one_has_bit_zero() {
    let bs = BitSet128::full(1);
    assert!(bs.test(0));
    assert_eq!(bs.count_ones(), 1);
    assert!(!bs.test(1));
}

#[test]
fn full_64_has_first_word_full() {
    let bs = BitSet128::full(64);
    assert_eq!(bs.count_ones(), 64);
    for i in 0..64 {
        assert!(bs.test(i), "bit {i} should be set");
    }
    for i in 64..128 {
        assert!(!bs.test(i), "bit {i} should not be set");
    }
}

#[test]
fn full_128_is_all_set() {
    let bs = BitSet128::full(128);
    assert_eq!(bs.count_ones(), 128);
    for i in 0..128 {
        assert!(bs.test(i), "bit {i} should be set");
    }
}

#[test]
fn full_intermediate_values() {
    let bs = BitSet128::full(65);
    assert_eq!(bs.count_ones(), 65);
    assert!(bs.test(64));
    assert!(!bs.test(65));

    let bs = BitSet128::full(100);
    assert_eq!(bs.count_ones(), 100);
    assert!(bs.test(99));
    assert!(!bs.test(100));
}

// ---- set / clear / test ----

#[test]
fn set_and_test_bit_zero() {
    let mut bs = BitSet128::new();
    assert!(!bs.test(0));
    bs.set(0);
    assert!(bs.test(0));
}

#[test]
fn set_and_test_bit_63_boundary() {
    let mut bs = BitSet128::new();
    bs.set(63);
    assert!(bs.test(63));
    assert!(!bs.test(62));
    assert!(!bs.test(64));
}

#[test]
fn set_and_test_bit_64_boundary() {
    let mut bs = BitSet128::new();
    bs.set(64);
    assert!(bs.test(64));
    assert!(!bs.test(63));
    assert!(!bs.test(65));
}

#[test]
fn set_and_test_bit_127() {
    let mut bs = BitSet128::new();
    bs.set(127);
    assert!(bs.test(127));
    assert!(!bs.test(126));
}

#[test]
fn clear_bit() {
    let mut bs = BitSet128::new();
    bs.set(10);
    assert!(bs.test(10));
    bs.clear(10);
    assert!(!bs.test(10));
}

#[test]
fn clear_bit_at_boundary_63() {
    let mut bs = BitSet128::full(128);
    bs.clear(63);
    assert!(!bs.test(63));
    assert!(bs.test(62));
    assert!(bs.test(64));
}

#[test]
fn clear_bit_at_boundary_64() {
    let mut bs = BitSet128::full(128);
    bs.clear(64);
    assert!(!bs.test(64));
    assert!(bs.test(63));
    assert!(bs.test(65));
}

// ---- count_ones ----

#[test]
fn count_ones_empty() {
    let bs = BitSet128::new();
    assert_eq!(bs.count_ones(), 0);
}

#[test]
fn count_ones_single() {
    let mut bs = BitSet128::new();
    bs.set(42);
    assert_eq!(bs.count_ones(), 1);
}

#[test]
fn count_ones_full() {
    let bs = BitSet128::full(128);
    assert_eq!(bs.count_ones(), 128);
}

#[test]
fn count_ones_across_words() {
    let mut bs = BitSet128::new();
    bs.set(0);
    bs.set(63);
    bs.set(64);
    bs.set(127);
    assert_eq!(bs.count_ones(), 4);
}

// ---- is_empty / is_singleton ----

#[test]
fn is_empty_on_empty() {
    assert!(BitSet128::new().is_empty());
}

#[test]
fn is_empty_on_non_empty() {
    let mut bs = BitSet128::new();
    bs.set(0);
    assert!(!bs.is_empty());
}

#[test]
fn is_singleton_on_single_bit() {
    let mut bs = BitSet128::new();
    bs.set(99);
    assert!(bs.is_singleton());
}

#[test]
fn is_singleton_on_empty() {
    assert!(!BitSet128::new().is_singleton());
}

#[test]
fn is_singleton_on_two_bits() {
    let mut bs = BitSet128::new();
    bs.set(0);
    bs.set(1);
    assert!(!bs.is_singleton());
}

// ---- first_one ----

#[test]
fn first_one_empty_returns_none() {
    assert_eq!(BitSet128::new().first_one(), None);
}

#[test]
fn first_one_single_bit() {
    let mut bs = BitSet128::new();
    bs.set(42);
    assert_eq!(bs.first_one(), Some(42));
}

#[test]
fn first_one_multiple_bits_returns_lowest() {
    let mut bs = BitSet128::new();
    bs.set(100);
    bs.set(50);
    bs.set(75);
    assert_eq!(bs.first_one(), Some(50));
}

#[test]
fn first_one_bit_in_second_word() {
    let mut bs = BitSet128::new();
    bs.set(64);
    assert_eq!(bs.first_one(), Some(64));
}

#[test]
fn first_one_bit_zero() {
    let mut bs = BitSet128::new();
    bs.set(0);
    bs.set(127);
    assert_eq!(bs.first_one(), Some(0));
}

// ---- iter_ones ----

#[test]
fn iter_ones_empty() {
    let bs = BitSet128::new();
    let bits: Vec<usize> = bs.iter_ones().collect();
    assert!(bits.is_empty());
}

#[test]
fn iter_ones_single() {
    let mut bs = BitSet128::new();
    bs.set(42);
    let bits: Vec<usize> = bs.iter_ones().collect();
    assert_eq!(bits, vec![42]);
}

#[test]
fn iter_ones_multiple_in_order() {
    let mut bs = BitSet128::new();
    bs.set(100);
    bs.set(5);
    bs.set(64);
    bs.set(0);
    bs.set(127);
    let bits: Vec<usize> = bs.iter_ones().collect();
    assert_eq!(bits, vec![0, 5, 64, 100, 127]);
}

#[test]
fn iter_ones_across_word_boundary() {
    let mut bs = BitSet128::new();
    bs.set(63);
    bs.set(64);
    let bits: Vec<usize> = bs.iter_ones().collect();
    assert_eq!(bits, vec![63, 64]);
}

#[test]
fn iter_ones_full_128() {
    let bs = BitSet128::full(128);
    let bits: Vec<usize> = bs.iter_ones().collect();
    assert_eq!(bits.len(), 128);
    for (i, &bit) in bits.iter().enumerate() {
        assert_eq!(bit, i);
    }
}

// ---- BitOrAssign ----

#[test]
fn bitor_assign_disjoint() {
    let mut a = BitSet128::new();
    a.set(0);
    a.set(2);

    let mut b = BitSet128::new();
    b.set(1);
    b.set(3);

    a |= b;

    let bits: Vec<usize> = a.iter_ones().collect();
    assert_eq!(bits, vec![0, 1, 2, 3]);
}

#[test]
fn bitor_assign_overlapping() {
    let mut a = BitSet128::new();
    a.set(5);
    a.set(10);

    let mut b = BitSet128::new();
    b.set(5);
    b.set(15);

    a |= b;

    let bits: Vec<usize> = a.iter_ones().collect();
    assert_eq!(bits, vec![5, 10, 15]);
}

#[test]
fn bitor_assign_across_words() {
    let mut a = BitSet128::new();
    a.set(0);

    let mut b = BitSet128::new();
    b.set(127);

    a |= b;

    assert!(a.test(0));
    assert!(a.test(127));
    assert_eq!(a.count_ones(), 2);
}

// ---- BitAndAssign ----

#[test]
fn bitand_assign_intersection() {
    let mut a = BitSet128::new();
    a.set(1);
    a.set(2);
    a.set(3);

    let mut b = BitSet128::new();
    b.set(2);
    b.set(3);
    b.set(4);

    a &= b;

    let bits: Vec<usize> = a.iter_ones().collect();
    assert_eq!(bits, vec![2, 3]);
}

#[test]
fn bitand_assign_disjoint_yields_empty() {
    let mut a = BitSet128::new();
    a.set(0);

    let mut b = BitSet128::new();
    b.set(1);

    a &= b;

    assert!(a.is_empty());
}

#[test]
fn bitand_assign_with_full() {
    let mut a = BitSet128::new();
    a.set(50);
    a.set(100);

    let full = BitSet128::full(128);
    a &= full;

    assert_eq!(a.count_ones(), 2);
    assert!(a.test(50));
    assert!(a.test(100));
}

// ---- Not ----

#[test]
fn not_empty_is_full() {
    let bs = BitSet128::new();
    let inv = !bs;
    // All 128 bits should be set
    assert_eq!(inv.count_ones(), 128);
}

#[test]
fn not_full_is_empty() {
    let bs = BitSet128::full(128);
    let inv = !bs;
    assert!(inv.is_empty());
}

#[test]
fn not_inverts_specific_bits() {
    let mut bs = BitSet128::new();
    bs.set(0);
    bs.set(127);
    let inv = !bs;
    assert!(!inv.test(0));
    assert!(!inv.test(127));
    assert!(inv.test(1));
    assert!(inv.test(126));
    assert_eq!(inv.count_ones(), 126);
}

// ---- and_not (diff) ----

#[test]
fn and_not_removes_common_bits() {
    let mut a = BitSet128::new();
    a.set(1);
    a.set(2);
    a.set(3);

    let mut b = BitSet128::new();
    b.set(2);
    b.set(4);

    let diff = a.and_not(&b);

    let bits: Vec<usize> = diff.iter_ones().collect();
    assert_eq!(bits, vec![1, 3]);
}

#[test]
fn and_not_with_empty_is_identity() {
    let mut a = BitSet128::new();
    a.set(10);
    a.set(20);

    let b = BitSet128::new();
    let diff = a.and_not(&b);

    assert_eq!(diff, a);
}

#[test]
fn and_not_with_superset_is_empty() {
    let a = BitSet128::full(10);
    let b = BitSet128::full(128);

    let diff = a.and_not(&b);
    assert!(diff.is_empty());
}

#[test]
fn and_not_across_words() {
    let mut a = BitSet128::new();
    a.set(63);
    a.set(64);

    let mut b = BitSet128::new();
    b.set(63);

    let diff = a.and_not(&b);
    let bits: Vec<usize> = diff.iter_ones().collect();
    assert_eq!(bits, vec![64]);
}

// ---- Edge cases ----

#[test]
fn set_same_bit_twice_is_idempotent() {
    let mut bs = BitSet128::new();
    bs.set(50);
    bs.set(50);
    assert_eq!(bs.count_ones(), 1);
    assert!(bs.test(50));
}

#[test]
fn clear_already_cleared_bit_is_noop() {
    let mut bs = BitSet128::new();
    bs.clear(50); // already clear
    assert!(!bs.test(50));
    assert!(bs.is_empty());
}

#[test]
fn clear_all_resets_everything() {
    let mut bs = BitSet128::full(128);
    assert_eq!(bs.count_ones(), 128);
    bs.clear_all();
    assert!(bs.is_empty());
    assert_eq!(bs.count_ones(), 0);
}

#[test]
fn default_is_empty() {
    let bs: BitSet128 = Default::default();
    assert!(bs.is_empty());
}

#[test]
fn copy_and_clone() {
    let mut a = BitSet128::new();
    a.set(5);
    a.set(100);

    let b = a; // Copy
    let c = a; // Clone

    assert_eq!(a, b);
    assert_eq!(a, c);
}

#[test]
fn eq_and_ne() {
    let mut a = BitSet128::new();
    a.set(1);

    let mut b = BitSet128::new();
    b.set(1);

    let mut c = BitSet128::new();
    c.set(2);

    assert_eq!(a, b);
    assert_ne!(a, c);
}

// ===========================================================================
// BitSet256 tests
// ===========================================================================

use wavfc::BitSet256;

#[test]
fn bitset256_full_200() {
    let bs = BitSet256::full(200);
    assert_eq!(bs.count_ones(), 200);
    for i in 0..200 {
        assert!(bs.test(i), "bit {i} should be set");
    }
    for i in 200..256 {
        assert!(!bs.test(i), "bit {i} should be clear");
    }
}

#[test]
fn bitset256_set_test_above_128() {
    let mut bs = BitSet256::new();
    bs.set(130);
    bs.set(200);
    bs.set(255);
    assert!(bs.test(130));
    assert!(bs.test(200));
    assert!(bs.test(255));
    assert!(!bs.test(129));
    assert!(!bs.test(131));
    assert_eq!(bs.count_ones(), 3);
}

#[test]
fn bitset256_iter_ones_across_four_words() {
    let mut bs = BitSet256::new();
    // Set one bit in each of the 4 words
    bs.set(10);   // word 0
    bs.set(70);   // word 1
    bs.set(140);  // word 2
    bs.set(210);  // word 3

    let bits: Vec<usize> = bs.iter_ones().collect();
    assert_eq!(bits, vec![10, 70, 140, 210]);
}

#[test]
fn bitset256_full_256() {
    let bs = BitSet256::full(256);
    assert_eq!(bs.count_ones(), 256);
    assert!(bs.test(0));
    assert!(bs.test(127));
    assert!(bs.test(128));
    assert!(bs.test(255));
}

#[test]
fn bitset256_and_not() {
    let a = BitSet256::full(200);
    let b = BitSet256::full(100);
    let c = a.and_not(&b);
    assert_eq!(c.count_ones(), 100);
    assert!(!c.test(50));
    assert!(c.test(150));
}

#[test]
fn bitset256_first_one_high_word() {
    let mut bs = BitSet256::new();
    bs.set(200);
    assert_eq!(bs.first_one(), Some(200));
}

#[test]
fn bitset256_clear_and_singleton() {
    let mut bs = BitSet256::new();
    bs.set(250);
    assert!(bs.is_singleton());
    bs.clear(250);
    assert!(bs.is_empty());
}

// Test BitSet<1> (64-bit)
#[test]
fn bitset_1_word() {
    use wavfc::BitSet;
    let mut bs = BitSet::<1>::new();
    assert_eq!(BitSet::<1>::MAX_BITS, 64);
    bs.set(63);
    assert!(bs.test(63));
    assert_eq!(bs.count_ones(), 1);
    let full = BitSet::<1>::full(64);
    assert_eq!(full.count_ones(), 64);
}
