use std::convert::TryInto;
use std::iter;

use bitvec::prelude::Lsb0;
use bitvec::view::AsBits;
use group::{
    ff::{PrimeField, PrimeFieldBits},
    Curve,
};
use halo2::{
    arithmetic::{CurveAffine, Field},
    circuit::{Layouter, SimpleFloorPlanner},
    dev::MockProver,
    pasta::{Fp, Fq},
    plonk::{Circuit, ConstraintSystem, Error},
};
use halo2_ecc::{
    chip::EccChip,
    gadget::{FixedPoint, FixedPoints},
};
use halo2_poseidon::{
    pow5t3::Pow5T3Chip as PoseidonChip,
    primitive::{ConstantLength, Hash, P128Pow5T3 as OrchardNullifier},
};
use halo2_utilities::{
    lookup_range_check::LookupRangeCheckConfig, CellValue, UtilitiesInstructions, Var,
};
#[allow(unused_imports)]
use orchard::constants::{
    fixed_bases::OrchardFixedBases, sinsemilla::MERKLE_CRH_PERSONALIZATION, OrchardHashDomains,
};
use rand::rngs::OsRng;
#[allow(unused_imports)]
use sinsemilla::{
    chip::SinsemillaChip,
    gadget::{
        HashDomain as SinsemillaHashDomain, Message as SinsemillaMessage,
        MessagePiece as SinsemillaMessagePiece,
    },
    primitive::{CommitDomain, HashDomain},
};

use halo2_examples::{circuit::BurnConfig, i2lebsp_k, pedersen_commitment, MERKLE_DEPTH};

#[derive(Default, Debug)]
struct BurnCircuit {
    secret_key: Option<Fq>,
    serial: Option<Fp>,
    value: Option<Fp>,
    asset: Option<Fp>,
    coin_blind: Option<Fp>,
    value_blind: Option<Fq>,
    asset_blind: Option<Fq>,
    merkle_path: Option<Vec<(Fp, bool)>>,
    sig_secret: Option<Fq>,
}

impl UtilitiesInstructions<Fp> for BurnCircuit {
    type Var = CellValue<Fp>;
}

impl Circuit<Fp> for BurnCircuit {
    type Config = BurnConfig;
    //type FloorPlanner = floor_planner::V1;
    type FloorPlanner = SimpleFloorPlanner;

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
        let lookup = (
            table_idx,
            meta.lookup_table_column(),
            meta.lookup_table_column(),
        );

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
            // We place the state columns after the partial_sbox column so that the
            // pad-and-add region can be layed out more efficiently.
            advices[6..9].try_into().unwrap(),
            advices[5],
            rc_a,
            rc_b,
        );

        let sinsemilla_config = SinsemillaChip::configure(
            meta,
            advices[..5].try_into().unwrap(),
            advices[6],
            lagrange_coeffs[0],
            lookup,
            range_check.clone(),
        );

        BurnConfig {
            primary,
            q_add,
            advices,
            ecc_config,
            poseidon_config,
            sinsemilla_config,
        }
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<Fp>,
    ) -> Result<(), Error> {
        // Load the Sinsemilla generator lookup table used by the whole circuit.
        SinsemillaChip::load(config.sinsemilla_config.clone(), &mut layouter)?;

        let sinsemilla_chip = config.sinsemilla_chip();
        let ecc_chip = config.ecc_chip();

        // TODO, get this out of the if clause
        if self.secret_key.is_some() {
            let mut bits: Vec<Option<bool>> = self
                .secret_key
                .unwrap()
                .to_le_bits()
                .iter()
                .by_val()
                .map(Some)
                .collect();

            // To 260 bits
            bits.extend(&[Some(false), Some(false), Some(false), Some(false)]);

            let bits0 = SinsemillaMessagePiece::from_bitstring(
                sinsemilla_chip.clone(),
                layouter.namespace(|| "some"),
                &bits[..250],
            )?;

            let bits1 = SinsemillaMessagePiece::from_bitstring(
                sinsemilla_chip.clone(),
                layouter.namespace(|| "bits1"),
                &bits[250..],
            )?;

            let mut serialbits: Vec<Option<bool>> = self
                .serial
                .unwrap()
                .to_le_bits()
                .iter()
                .by_val()
                .map(Some)
                .collect();

            serialbits.extend(&[Some(false), Some(false), Some(false), Some(false)]);

            let bits2 = SinsemillaMessagePiece::from_bitstring(
                sinsemilla_chip.clone(),
                layouter.namespace(|| "bits2"),
                &serialbits[..250],
            )?;

            let bits3 = SinsemillaMessagePiece::from_bitstring(
                sinsemilla_chip.clone(),
                layouter.namespace(|| "bits3"),
                &serialbits[250..],
            )?;

            let message = SinsemillaMessage::from_pieces(
                sinsemilla_chip.clone(),
                vec![bits0, bits1, bits2, bits3],
            );

            let domain = SinsemillaHashDomain::new(
                sinsemilla_chip.clone(),
                ecc_chip.clone(),
                &OrchardHashDomains::MerkleCrh,
            );

            let hash = domain.hash(layouter.namespace(|| "hash nullifier"), message);
            // Nullifier
            layouter.constrain_instance(hash.unwrap().0.inner().cell(), config.primary, 0)?;
        }

        let value = self.load_private(
            layouter.namespace(|| "load value"),
            config.advices[0],
            self.value,
        )?;

        let asset = self.load_private(
            layouter.namespace(|| "load asset"),
            config.advices[0],
            self.asset,
        )?;

        // ================
        // Value commitment
        // ================

        // This constant one is used for multiplication
        let one = self.load_constant(
            layouter.namespace(|| "constant one"),
            config.advices[0],
            Fp::one(),
        )?;

        // v*G_1
        let (commitment, _) = {
            let value_commit_v = OrchardFixedBases::ValueCommitV;
            let value_commit_v = FixedPoint::from_inner(ecc_chip.clone(), value_commit_v);
            value_commit_v.mul_short(layouter.namespace(|| "[value] ValueCommitV"), (value, one))?
        };

        // r_V*G_2
        let (blind, _rcv) = {
            let rcv = self.value_blind;
            let value_commit_r = OrchardFixedBases::ValueCommitR;
            let value_commit_r = FixedPoint::from_inner(ecc_chip.clone(), value_commit_r);
            value_commit_r.mul(layouter.namespace(|| "[value_blind] ValueCommitR"), rcv)?
        };

        let value_commit = commitment.add(layouter.namespace(|| "valuecommit"), &blind)?;
        layouter.constrain_instance(value_commit.inner().x().cell(), config.primary, 1)?;
        layouter.constrain_instance(value_commit.inner().y().cell(), config.primary, 2)?;

        // ================
        // Asset commitment
        // ================

        // v*G_1
        let (commitment, _) = {
            let asset_commit_v = OrchardFixedBases::ValueCommitV;
            let asset_commit_v = FixedPoint::from_inner(ecc_chip.clone(), asset_commit_v);
            asset_commit_v.mul_short(layouter.namespace(|| "[asset] ValueCommitV"), (asset, one))?
        };

        // r_V*G_2
        let (blind, _rca) = {
            let rca = self.asset_blind;
            let asset_commit_r = OrchardFixedBases::ValueCommitR;
            let asset_commit_r = FixedPoint::from_inner(ecc_chip.clone(), asset_commit_r);
            asset_commit_r.mul(layouter.namespace(|| "[asset_blind] ValueCommitR"), rca)?
        };

        let asset_commit = commitment.add(layouter.namespace(|| "assetcommit"), &blind)?;
        layouter.constrain_instance(asset_commit.inner().x().cell(), config.primary, 3)?;
        layouter.constrain_instance(asset_commit.inner().y().cell(), config.primary, 4)?;

        // =========================
        // Signature key derivation
        // =========================
        let (sig_pub, _) = {
            let spend_auth_g = OrchardFixedBases::SpendAuthG;
            let spend_auth_g = FixedPoint::from_inner(ecc_chip.clone(), spend_auth_g);
            spend_auth_g.mul(layouter.namespace(|| "[x_s] SpendAuthG"), self.sig_secret)?
        };

        layouter.constrain_instance(sig_pub.inner().x().cell(), config.primary, 6)?;
        layouter.constrain_instance(sig_pub.inner().y().cell(), config.primary, 7)?;

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
    //let merkletree: Vec<bool> = (0..6).map(|i| (depth >> i) & 1 == 1).collect();
    //println!("{:?}", merkletree);

    let domain = HashDomain::new(MERKLE_CRH_PERSONALIZATION);
    domain
        .hash(
            iter::empty()
                .chain(i2lebsp_k(depth).iter().copied())
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
    let mut bits0: Vec<bool> = secret_key.to_le_bits().iter().by_val().collect();
    let mut bits1: Vec<bool> = serial.to_le_bits().iter().by_val().collect();
    // We extend these because sinsemilla wants modulo 10, and the above are 256 bits.
    bits0.extend(&[false, false, false, false]);
    bits1.extend(&[false, false, false, false]);

    let nullifier = domain
        .hash(iter::empty().chain(bits0).chain(bits1))
        .unwrap();

    println!("{:?}", nullifier);

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
    merkle_path.resize(MERKLE_DEPTH, true);

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
    let public_inputs = vec![
        nullifier,
        *value_coords.x(),
        *value_coords.y(),
        *asset_coords.x(),
        *asset_coords.y(),
        merkle_root,
        *sig_coords.x(),
        *sig_coords.y(),
    ];

    // println!("{:?}", asset_coords.x());
    // println!("{:?}", asset_coords.y());

    // ========
    // ZK Proof
    // ========
    let circuit = BurnCircuit {
        secret_key: Some(secret_key),
        serial: Some(serial),
        value: Some(Fp::from(value)),
        asset: Some(Fp::from(asset)),
        coin_blind: Some(coin_blind),
        value_blind: Some(value_blind),
        asset_blind: Some(asset_blind),
        merkle_path: Some(merkle_path),
        sig_secret: Some(sig_secret),
    };

    // Valid MockProver
    let prover = MockProver::run(12, &circuit, vec![public_inputs.clone()]).unwrap();
    assert_eq!(prover.verify(), Ok(()));
}
