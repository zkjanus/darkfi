use std::{convert::TryInto, time::Instant};

use group::{ff::Field, Curve, Group};
use halo2::{
    arithmetic::CurveAffine,
    circuit::{Layouter, SimpleFloorPlanner},
    dev::MockProver,
    pasta::{pallas, vesta},
    plonk,
    plonk::{Circuit, ConstraintSystem, Error},
    poly::commitment,
    transcript::{Blake2bRead, Blake2bWrite},
};
use halo2_ecc::{chip::EccChip, gadget::FixedPoint};
use halo2_poseidon::{
    gadget::{Hash as PoseidonHash, Word},
    pow5t3::{Pow5T3Chip as PoseidonChip, StateWord},
    primitive::{ConstantLength, Hash, P128Pow5T3 as OrchardNullifier},
};
use halo2_utilities::{
    lookup_range_check::LookupRangeCheckConfig, CellValue, UtilitiesInstructions, Var,
};
use orchard::constants::fixed_bases::OrchardFixedBases;
use rand::rngs::OsRng;

use halo2_examples::{circuit::MintConfig, pedersen_commitment};

// The number of rows in our circuit cannot exceed 2^k
const K: u32 = 9;

// This struct defines our circuit and cointains its private inputs.
#[derive(Default, Debug)]
struct MintCircuit {
    pub_x: Option<pallas::Base>,         // x coordinate for pubkey
    pub_y: Option<pallas::Base>,         // y coordinate for pubkey
    value: Option<pallas::Base>,         // The value of this coin
    asset: Option<pallas::Base>,         // The asset ID
    serial: Option<pallas::Base>,        // Unique serial number corresponding to this coin
    coin_blind: Option<pallas::Base>,    // Random blinding factor for coin
    value_blind: Option<pallas::Scalar>, // Random blinding factor for value commitment
    asset_blind: Option<pallas::Scalar>, // Random blinding factor for the asset ID
}

// The public input array offsets
const COIN_OFFSET: usize = 0;
const VALCOMX_OFFSET: usize = 1;
const VALCOMY_OFFSET: usize = 2;
const ASSCOMX_OFFSET: usize = 3;
const ASSCOMY_OFFSET: usize = 4;

impl UtilitiesInstructions<pallas::Base> for MintCircuit {
    type Var = CellValue<pallas::Base>;
}

impl Circuit<pallas::Base> for MintCircuit {
    type Config = MintConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<pallas::Base>) -> Self::Config {
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

        MintConfig {
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
        mut layouter: impl Layouter<pallas::Base>,
    ) -> Result<(), Error> {
        let ecc_chip = config.ecc_chip();

        let pub_x = self.load_private(
            layouter.namespace(|| "load pubkey x"),
            config.advices[0],
            self.pub_x,
        )?;
        let pub_y = self.load_private(
            layouter.namespace(|| "load pubkey y"),
            config.advices[0],
            self.pub_y,
        )?;
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
        let serial = self.load_private(
            layouter.namespace(|| "load serial"),
            config.advices[0],
            self.serial,
        )?;
        let coin_blind = self.load_private(
            layouter.namespace(|| "load coin_blind"),
            config.advices[0],
            self.coin_blind,
        )?;

        // =========
        // Coin hash
        // =========
        // TODO: This is a hack, but may work.
        // At the moment, we make three hashes and then the Coin C is their sum.
        // C = Poseidon(pub_x, pub_y) + Poseidon(value, asset) + Poseidon(serial, coin_blind)
        let mut coin = pallas::Base::zero();
        let messages = [[pub_x, pub_y], [value, asset], [serial, coin_blind]];
        for msg in messages.iter() {
            let poseidon_message = layouter.assign_region(
                || "load message",
                |mut region| {
                    let mut message_word = |i: usize| {
                        let val = msg[i].value();
                        let var = region.assign_advice(
                            || format!("load message_{}", i),
                            config.poseidon_config.state()[i],
                            0,
                            || val.ok_or(Error::SynthesisError),
                        )?;
                        region.constrain_equal(var, msg[i].cell())?;
                        Ok(Word::<_, _, OrchardNullifier, 3, 2>::from_inner(
                            StateWord::new(var, val),
                        ))
                    };
                    Ok([message_word(0)?, message_word(1)?])
                },
            )?;

            let poseidon_hasher = PoseidonHash::init(
                config.poseidon_chip(),
                layouter.namespace(|| "Poseidon init"),
                ConstantLength::<2>,
            )?;

            let poseidon_output =
                poseidon_hasher.hash(layouter.namespace(|| "Poseidon hash"), poseidon_message)?;

            let poseidon_output: CellValue<pallas::Base> = poseidon_output.inner().into();

            if !poseidon_output.value().is_none() {
                coin += poseidon_output.value().unwrap();
            }
        }

        let hash = self.load_private(
            layouter.namespace(|| "load hash"),
            config.advices[0],
            Some(coin),
        )?;

        // Constrain the coin C
        layouter.constrain_instance(hash.cell(), config.primary, COIN_OFFSET)?;

        // ================
        // Value commitment
        // ================

        // This constant one is used for multiplication
        let one = self.load_constant(
            layouter.namespace(|| "constant one"),
            config.advices[0],
            pallas::Base::one(),
        )?;

        // v * G_1
        let (commitment, _) = {
            let value_commit_v = OrchardFixedBases::ValueCommitV;
            let value_commit_v = FixedPoint::from_inner(ecc_chip.clone(), value_commit_v);
            value_commit_v.mul_short(layouter.namespace(|| "[value] ValueCommitV"), (value, one))?
        };

        // r_V * G_2
        let (blind, _rcv) = {
            let rcv = self.value_blind;
            let value_commit_r = OrchardFixedBases::ValueCommitR;
            let value_commit_r = FixedPoint::from_inner(ecc_chip.clone(), value_commit_r);
            value_commit_r.mul(layouter.namespace(|| "[value_blind] ValueCommitR"), rcv)?
        };

        // Constrain the value commitment coordinates
        let value_commit = commitment.add(layouter.namespace(|| "valuecommit"), &blind)?;
        layouter.constrain_instance(
            value_commit.inner().x().cell(),
            config.primary,
            VALCOMX_OFFSET,
        )?;
        layouter.constrain_instance(
            value_commit.inner().y().cell(),
            config.primary,
            VALCOMY_OFFSET,
        )?;

        // ================
        // Asset commitment
        // ================

        // a * G_1
        let (commitment, _) = {
            let asset_commit_v = OrchardFixedBases::ValueCommitV;
            let asset_commit_v = FixedPoint::from_inner(ecc_chip.clone(), asset_commit_v);
            asset_commit_v.mul_short(layouter.namespace(|| "[asset] ValueCommitV"), (asset, one))?
        };

        // r_A * G_2
        let (blind, _rca) = {
            let rca = self.asset_blind;
            let asset_commit_r = OrchardFixedBases::ValueCommitR;
            let asset_commit_r = FixedPoint::from_inner(ecc_chip.clone(), asset_commit_r);
            asset_commit_r.mul(layouter.namespace(|| "[asset_blind] ValueCommitR"), rca)?
        };

        // Constrain the asset commitment coordinates
        let asset_commit = commitment.add(layouter.namespace(|| "assetcommit"), &blind)?;
        layouter.constrain_instance(
            asset_commit.inner().x().cell(),
            config.primary,
            ASSCOMX_OFFSET,
        )?;
        layouter.constrain_instance(
            asset_commit.inner().y().cell(),
            config.primary,
            ASSCOMY_OFFSET,
        )?;

        // At this point we've enforced all of our public inputs.
        Ok(())
    }
}

#[derive(Debug)]
struct VerifyingKey {
    params: commitment::Params<vesta::Affine>,
    vk: plonk::VerifyingKey<vesta::Affine>,
}

impl VerifyingKey {
    fn build() -> Self {
        let params = commitment::Params::new(K);
        let circuit: MintCircuit = Default::default();

        let vk = plonk::keygen_vk(&params, &circuit).unwrap();

        VerifyingKey { params, vk }
    }
}

#[derive(Debug)]
struct ProvingKey {
    params: commitment::Params<vesta::Affine>,
    pk: plonk::ProvingKey<vesta::Affine>,
}

impl ProvingKey {
    fn build() -> Self {
        let params = commitment::Params::new(K);
        let circuit: MintCircuit = Default::default();

        let vk = plonk::keygen_vk(&params, &circuit).unwrap();
        let pk = plonk::keygen_pk(&params, vk, &circuit).unwrap();

        ProvingKey { params, pk }
    }
}

#[derive(Clone, Debug)]
struct Proof(Vec<u8>);

impl AsRef<[u8]> for Proof {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Proof {
    fn create(
        pk: &ProvingKey,
        circuits: &[MintCircuit],
        pubinputs: &[pallas::Base],
    ) -> Result<Self, Error> {
        let mut transcript = Blake2bWrite::<_, vesta::Affine, _>::init(vec![]);
        plonk::create_proof(
            &pk.params,
            &pk.pk,
            circuits,
            &[&[pubinputs]],
            &mut transcript,
        )?;
        Ok(Proof(transcript.finalize()))
    }

    fn verify(&self, vk: &VerifyingKey, pubinputs: &[pallas::Base]) -> Result<(), plonk::Error> {
        let msm = vk.params.empty_msm();
        let mut transcript = Blake2bRead::init(&self.0[..]);
        let guard = plonk::verify_proof(&vk.params, &vk.vk, msm, &[&[pubinputs]], &mut transcript)?;
        let msm = guard.clone().use_challenges();
        if msm.eval() {
            Ok(())
        } else {
            Err(Error::ConstraintSystemFailure)
        }
    }

    // fn new(bytes: Vec<u8>) -> Self {
    // Proof(bytes)
    // }
}

fn main() {
    let pubkey = pallas::Point::random(&mut OsRng);
    let coords = pubkey.to_affine().coordinates().unwrap();

    let value = 110;
    let asset = 1;

    let value_blind = pallas::Scalar::random(&mut OsRng);
    let asset_blind = pallas::Scalar::random(&mut OsRng);

    let serial = pallas::Base::random(&mut OsRng);
    let coin_blind = pallas::Base::random(&mut OsRng);

    let mut coin = pallas::Base::zero();

    let messages = [
        [*coords.x(), *coords.y()],
        [pallas::Base::from(value), pallas::Base::from(asset)],
        [serial, coin_blind],
    ];

    for msg in messages.iter() {
        coin += Hash::init(OrchardNullifier, ConstantLength::<2>).hash(*msg);
    }

    let value_commit = pedersen_commitment(value, value_blind);
    let value_coords = value_commit.to_affine().coordinates().unwrap();

    let asset_commit = pedersen_commitment(asset, asset_blind);
    let asset_coords = asset_commit.to_affine().coordinates().unwrap();

    let mut public_inputs = vec![
        coin,
        *value_coords.x(),
        *value_coords.y(),
        *asset_coords.x(),
        *asset_coords.y(),
    ];

    let circuit = MintCircuit {
        pub_x: Some(*coords.x()),
        pub_y: Some(*coords.y()),
        value: Some(vesta::Scalar::from(value)),
        asset: Some(vesta::Scalar::from(asset)),
        serial: Some(serial),
        coin_blind: Some(coin_blind),
        value_blind: Some(value_blind),
        asset_blind: Some(asset_blind),
    };

    // Valid MockProver
    let prover = MockProver::run(K, &circuit, vec![public_inputs.clone()]).unwrap();
    assert_eq!(prover.verify(), Ok(()));

    // Break the public inputs by adding 0xdeadbeef to the Coin C
    public_inputs[0] += pallas::Base::from(0xdeadbeef);

    // Invalid MockProver
    let prover = MockProver::run(K, &circuit, vec![public_inputs.clone()]).unwrap();
    assert!(prover.verify().is_err());

    // Make the public inputs valid again.
    public_inputs[0] -= pallas::Base::from(0xdeadbeef);

    // Actual ZK proof
    let start = Instant::now();
    let vk = VerifyingKey::build();
    let pk = ProvingKey::build();
    println!("\nSetup: [{:?}]", start.elapsed());

    let start = Instant::now();
    let proof = Proof::create(&pk, &[circuit], &public_inputs).unwrap();
    println!("Prove: [{:?}]", start.elapsed());

    let start = Instant::now();
    assert!(proof.verify(&vk, &public_inputs).is_ok());
    println!("Verify: [{:?}]", start.elapsed());
}
