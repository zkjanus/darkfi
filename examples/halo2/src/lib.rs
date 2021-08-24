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
