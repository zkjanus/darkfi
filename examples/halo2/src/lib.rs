pub mod circuit;
//pub mod tree;

use halo2::{
    arithmetic::{CurveExt, FieldExt},
    pasta::{Ep, Fq},
};
use orchard::constants::fixed_bases::{
    VALUE_COMMITMENT_PERSONALIZATION, VALUE_COMMITMENT_R_BYTES, VALUE_COMMITMENT_V_BYTES,
};

pub const MERKLE_DEPTH: usize = 32;

pub const L_ORCHARD_MERKLE: usize = 255;

#[allow(non_snake_case)]
pub fn pedersen_commitment(value: u64, blind: Fq) -> Ep {
    let hasher = Ep::hash_to_curve(VALUE_COMMITMENT_PERSONALIZATION);
    let V = hasher(&VALUE_COMMITMENT_V_BYTES);
    let R = hasher(&VALUE_COMMITMENT_R_BYTES);
    let value = Fq::from_u64(value);

    V * value + R * blind
}

pub const K: usize = 10;

pub fn gen_const_array<Output: Copy + Default, const LEN: usize>(
    mut closure: impl FnMut(usize) -> Output,
) -> [Output; LEN] {
    let mut ret: [Output; LEN] = [Default::default(); LEN];
    for (bit, val) in ret.iter_mut().zip((0..LEN).map(|idx| closure(idx))) {
        *bit = val;
    }
    ret
}

pub fn i2lebsp<const NUM_BITS: usize>(int: u64) -> [bool; NUM_BITS] {
    assert!(NUM_BITS <= 64);
    gen_const_array(|mask: usize| (int & (1 << mask)) != 0)
}

pub fn i2lebsp_k(int: usize) -> [bool; K] {
    assert!(int < (1 << K));
    i2lebsp(int as u64)
}
