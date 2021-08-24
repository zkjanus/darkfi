use std::convert::TryInto;
use std::iter;

use bitvec::prelude::Lsb0;
use bitvec::view::AsBits;
use group::{
    ff::{PrimeField, PrimeFieldBits},
    Curve,
};
use halo2::{
    arithmetic::{CurveAffine, Field, FieldExt},
    circuit::{floor_planner, Layouter},
    pasta::{Fp, Fq},
    plonk::{Circuit, ConstraintSystem, Error},
};
use halo2_ecc::{chip::EccChip, gadget::FixedPoints};
use halo2_poseidon::{
    pow5t3::Pow5T3Chip as PoseidonChip,
    primitive::{ConstantLength, Hash, P128Pow5T3 as OrchardNullifier},
};
use halo2_utilities::{
    lookup_range_check::LookupRangeCheckConfig, CellValue, UtilitiesInstructions,
};
use orchard::constants::{fixed_bases::OrchardFixedBases, sinsemilla::MERKLE_CRH_PERSONALIZATION};
use rand::rngs::OsRng;
use sinsemilla::{
    gadget::HashDomains,
    primitive::{CommitDomain, HashDomain},
};

use halo2_examples::{circuit::Config, pedersen_commitment};

pub const SAPLING_COMMITMENT_TREE_DEPTH: usize = 32;

#[derive(Default, Debug)]
struct BurnCircuit {
    secret_key: Option<Fq>,
    serial: Option<Fp>,
    value: Option<Fp>,
    asset: Option<Fp>,
    coin_blind: Option<Fp>,
    value_blind: Option<Fq>,
    asset_blind: Option<Fq>,
    branch: u8,
    isright: u8,
    sig_secret: Option<Fq>,
}

impl UtilitiesInstructions<Fp> for BurnCircuit {
    type Var = CellValue<Fp>;
}

impl Circuit<Fp> for BurnCircuit {
    type Config = Config;
    type FloorPlanner = floor_planner::V1;
    //type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<Fp>) -> Self::Config {
        let advices = [
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
        ];

        let q_add = meta.selector();

        let table_idx = meta.lookup_table_column();

        let primary = meta.instance_column();

        meta.enable_equality(primary.into());

        for advice in advices.iter() {
            meta.enable_equality((*advice).into());
        }

        let lagrange_coeffs = [
            meta.fixed_column(),
            meta.fixed_column(),
            meta.fixed_column(),
            meta.fixed_column(),
            meta.fixed_column(),
            meta.fixed_column(),
            meta.fixed_column(),
            meta.fixed_column(),
        ];

        let rc_a = lagrange_coeffs[2..5].try_into().unwrap();
        let rc_b = lagrange_coeffs[5..8].try_into().unwrap();

        meta.enable_constant(lagrange_coeffs[0]);

        let range_check = LookupRangeCheckConfig::configure(meta, advices[9], table_idx);

        let ecc_config = EccChip::<OrchardFixedBases>::configure(
            meta,
            advices,
            lagrange_coeffs,
            range_check.clone(),
        );

        let poseidon_config = PoseidonChip::configure(
            meta,
            OrchardNullifier,
            advices[6..9].try_into().unwrap(),
            advices[5],
            rc_a,
            rc_b,
        );

        Config {
            primary,
            q_add,
            advices,
            ecc_config,
            poseidon_config,
        }
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<Fp>,
    ) -> Result<(), Error> {
        Ok(())
    }
}

fn merkle_hash(depth: usize, lhs: &[u8; 32], rhs: &[u8; 32]) -> Fp {
    // This thing is nasty lol
    let lhs = {
        let mut tmp = [false; 256];
        for (a, b) in tmp.iter_mut().zip(lhs.as_bits::<Lsb0>()) {
            *a = *b;
        }
        tmp
    };

    let rhs = {
        let mut tmp = [false; 256];
        for (a, b) in tmp.iter_mut().zip(rhs.as_bits::<Lsb0>()) {
            *a = *b;
        }
        tmp
    };

    // TODO: Review this
    let merkletree: Vec<bool> = (0..6).map(|i| (depth >> i) & 1 == 1).collect();

    let domain = HashDomain::new(MERKLE_CRH_PERSONALIZATION);
    domain
        .hash(
            iter::empty()
                .chain(lhs.iter().copied())
                .chain(rhs.iter().copied()),
        )
        .unwrap()
}

fn main() {
    let secret_key = Fq::random(&mut OsRng);
    let serial = Fp::random(&mut OsRng);
    let value = 110;
    let asset = 1;

    // =========
    // Nullifier
    // =========
    // N = SinsemillaHash(secret_key, serial)
    //
    let domain = HashDomain::new(MERKLE_CRH_PERSONALIZATION);
    let nullifier = domain
        .hash(
            iter::empty()
                .chain(secret_key.to_le_bits().iter().by_val())
                .chain(serial.to_le_bits().iter().by_val()),
        )
        .unwrap();

    // =====================
    // Public key derivation
    // =====================
    // P = secret_key * G
    //
    let public_key = OrchardFixedBases::SpendAuthG.generator() * secret_key;
    let coords = public_key.to_affine().coordinates().unwrap();

    // ==============
    // Construct Coin
    // ==============
    // C = PoseidonHash(public_key, value, asset, serial, coin_blind)
    //
    // FIXME:
    let coin_blind = Fp::random(&mut OsRng);
    let messages = [
        [*coords.x(), *coords.y()],
        [Fp::from(value), Fp::from(asset)],
        [serial, coin_blind],
    ];
    let mut coin = Fp::zero();
    for msg in messages.iter() {
        coin += Hash::init(OrchardNullifier, ConstantLength::<2>).hash(*msg);
    }

    // ===========
    // Merkle Root
    // ===========
    // Hash the coin C, then for each branch of the tree:
    // 1) Conditionally reverse:
    //    (left, right) = if is_right{(branch_i, current)} else {(current, branch_i)}
    // 2) Hash left and right values:
    //    current = Hash(left, right)
    // 3) Final hash value is the merkle root R
    //
    // TODO: Review this
    let mut merkle_path = vec![true, false];
    merkle_path.resize(32, true);

    let merkle_path: Vec<(Fp, bool)> = merkle_path
        .into_iter()
        .map(|x| (Fp::random(&mut OsRng), x))
        .collect();

    let mut merkle_root = coin.clone();

    for (i, (right, is_right)) in merkle_path.iter().enumerate() {
        if *is_right {
            merkle_root = merkle_hash(i, &right.to_repr(), &merkle_root.to_repr());
        } else {
            merkle_root = merkle_hash(i, &merkle_root.to_repr(), &right.to_repr());
        }
    }

    //let merkle_root = merkle_root.to_repr();

    // ===========================
    // Value and asset commitments
    // ============================
    // V = value * G_1 + value_blind * G_2
    // A = asset * G_1 + asset_blind * G_2
    let value_blind = Fq::random(&mut OsRng);
    let asset_blind = Fq::random(&mut OsRng);
    let value_commit = pedersen_commitment(value, value_blind);
    let asset_commit = pedersen_commitment(asset, asset_blind);

    let value_coords = value_commit.to_affine().coordinates().unwrap();
    let asset_coords = asset_commit.to_affine().coordinates().unwrap();

    // =====================================================
    // Derive signature public key from signature secret key
    // =====================================================
    // sig_pubkey = sig_secret * G_sig
    let sig_secret = Fq::random(&mut OsRng);
    let sig_pubkey = OrchardFixedBases::SpendAuthG.generator() * sig_secret;
    let sig_coords = sig_pubkey.to_affine().coordinates().unwrap();

    // =======================================================================
    // Return (nullifier, value_commit, asset_commit, merkle_root, sig_pubkey)
    // =======================================================================
    // (N, V, A, R, P_s)
    let mut public_inputs = vec![
        nullifier,
        *value_coords.x(),
        *value_coords.y(),
        *asset_coords.x(),
        *asset_coords.y(),
        merkle_root,
        *sig_coords.x(),
        *sig_coords.y(),
    ];
}
