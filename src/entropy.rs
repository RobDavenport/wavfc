//! Cell selection heuristics for the WFC solver.
//!
//! Provides strategies for choosing which uncollapsed cell to collapse next.
//! The choice of heuristic affects both solve speed and output quality.

use crate::bitset::BitSet;
use rand_core::RngCore;

/// Cell selection heuristic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Heuristic {
    /// Select the cell with the fewest remaining states. Fast, good results.
    #[default]
    MinCount,
    /// Select the cell with lowest Shannon entropy (accounts for weights).
    /// Better distribution, slower.
    ShannonEntropy,
}

/// Selects the uncollapsed cell with the fewest remaining states.
///
/// Ties are broken by random selection among all cells sharing the minimum count.
/// Returns `None` if all cells are collapsed (entropy <= 1).
pub fn select_min_count(entropy_cache: &[u32], rng: &mut impl RngCore) -> Option<usize> {
    let mut min_entropy: u32 = u32::MAX;
    let mut count: usize = 0;

    // First pass: find the minimum entropy > 1
    for &e in entropy_cache {
        if e > 1 && e < min_entropy {
            min_entropy = e;
        }
    }

    if min_entropy == u32::MAX {
        return None;
    }

    // Second pass: count cells with that minimum
    for &e in entropy_cache {
        if e == min_entropy {
            count += 1;
        }
    }

    // Pick a random one among the ties
    let target = (rng.next_u32() as usize) % count;
    let mut seen = 0;
    for (i, &e) in entropy_cache.iter().enumerate() {
        if e == min_entropy {
            if seen == target {
                return Some(i);
            }
            seen += 1;
        }
    }

    // Should not be reached, but just in case
    None
}

/// Selects the uncollapsed cell with the lowest Shannon entropy.
///
/// Shannon entropy is `-sum(p_i * ln(p_i))` where `p_i = weight_i / sum_weights`
/// for each remaining state. This accounts for state weights and produces
/// better-distributed results than simple minimum count, at higher computational cost.
///
/// Returns `None` if all cells are collapsed.
pub fn select_shannon<const W: usize>(
    possibilities: &[BitSet<W>],
    weights: &[f64],
    rng: &mut impl RngCore,
) -> Option<usize> {
    let mut min_entropy = f64::MAX;
    let mut best_count: usize = 0;

    // First pass: find the minimum Shannon entropy among uncollapsed cells
    for poss in possibilities {
        let cnt = poss.count_ones();
        if cnt <= 1 {
            continue;
        }
        let entropy = shannon_entropy(poss, weights);
        if entropy < min_entropy - 1e-12 {
            min_entropy = entropy;
            best_count = 1;
        } else if (entropy - min_entropy).abs() < 1e-12 {
            best_count += 1;
        }
    }

    if best_count == 0 {
        return None;
    }

    // Pick a random one among ties
    let target = (rng.next_u32() as usize) % best_count;
    let mut seen = 0;
    for (i, poss) in possibilities.iter().enumerate() {
        let cnt = poss.count_ones();
        if cnt <= 1 {
            continue;
        }
        let entropy = shannon_entropy(poss, weights);
        if (entropy - min_entropy).abs() < 1e-12 {
            if seen == target {
                return Some(i);
            }
            seen += 1;
        }
    }

    None
}

/// Computes the Shannon entropy for a single cell given its possibilities and weights.
fn shannon_entropy<const W: usize>(poss: &BitSet<W>, weights: &[f64]) -> f64 {
    let mut sum_weights = 0.0;
    let mut sum_w_log_w = 0.0;

    for s in poss.iter_ones() {
        let w = weights[s];
        if w > 0.0 {
            sum_weights += w;
            sum_w_log_w += w * ln_approx(w);
        }
    }

    if sum_weights <= 0.0 {
        return 0.0;
    }

    ln_approx(sum_weights) - sum_w_log_w / sum_weights
}

/// Approximate natural logarithm for no_std environments.
///
/// Uses a simple series expansion. Accurate enough for entropy calculations.
fn ln_approx(x: f64) -> f64 {
    // We use the identity: ln(x) = ln(2) * log2(x)
    // and f64::log2 is not available in no_std, so we use bit manipulation.
    //
    // For a positive f64, we can extract the exponent and mantissa:
    // x = mantissa * 2^exponent, where 1.0 <= mantissa < 2.0
    // ln(x) = ln(mantissa) + exponent * ln(2)
    //
    // For ln(mantissa) where 1.0 <= m < 2.0, we use a polynomial approximation.
    if x <= 0.0 {
        return f64::NEG_INFINITY;
    }

    let bits = x.to_bits();
    let exponent = ((bits >> 52) & 0x7FF) as i64 - 1023;
    // Extract mantissa in [1.0, 2.0)
    let mantissa_bits = (bits & 0x000F_FFFF_FFFF_FFFF) | 0x3FF0_0000_0000_0000;
    let m = f64::from_bits(mantissa_bits);

    // Approximate ln(m) for m in [1.0, 2.0) using a minimax polynomial
    // ln(m) ~ (m-1) - (m-1)^2/2 + (m-1)^3/3 - ...
    // Using a Pade-like approximation for better accuracy:
    let t = m - 1.0;
    let ln_m = t * (1.0 + t * (-0.5 + t * (1.0 / 3.0 + t * (-0.25 + t * 0.2))));

    ln_m + (exponent as f64) * core::f64::consts::LN_2
}
