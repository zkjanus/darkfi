use std::collections::HashMap;
use std::{convert::TryInto, time::Instant};
use halo2::{
    pasta::pallas,
    circuit::{Layouter, SimpleFloorPlanner},
    plonk::{Advice, Circuit, Column, ConstraintSystem, Instance as InstanceColumn, Selector, Error as PlonkError},
};

use halo2_ecc::chip::{EccPoint, EccChip, EccConfig};
use halo2_poseidon::{
    primitive::{ConstantLength, Hash, P128Pow5T3 as OrchardNullifier},
    pow5t3::{Pow5T3Chip as PoseidonChip, Pow5T3Config as PoseidonConfig}};
use halo2_utilities::{
    lookup_range_check::LookupRangeCheckConfig, CellValue, UtilitiesInstructions, Var,
};
use orchard::constants::{OrchardCommitDomains, OrchardFixedBases, OrchardHashDomains};
use sinsemilla::chip::{SinsemillaChip, SinsemillaConfig};

use crate::error::{Result, Error};

#[derive(Clone, Debug, PartialEq)]
pub enum ZkType {
    Base,
    Scalar,
    EcPoint,
    EcFixedPoint,
}

type RetValIdx = usize;
type ArgIdx = usize;

#[derive(Clone, Debug)]
pub enum ZkFunctionCall {
    PoseidonHash(RetValIdx, (ArgIdx, ArgIdx)),
    Add(RetValIdx, (ArgIdx, ArgIdx)),
    ConstrainInstance(ArgIdx),
    EcMulShort(RetValIdx, (ArgIdx, ArgIdx)),
    EcMul(RetValIdx, (ArgIdx, ArgIdx)),
    EcAdd(RetValIdx, (ArgIdx, ArgIdx)),
    EcGetX(RetValIdx, ArgIdx),
    EcGetY(RetValIdx, ArgIdx),
}

pub struct ZkBinary {
    pub constants: Vec<(String, ZkType)>,
    pub contracts: HashMap<String, ZkContract>,
}

#[derive(Clone, Debug)]
pub struct ZkContract {
    pub witness: Vec<(String, ZkType)>,
    pub code: Vec<ZkFunctionCall>,
}

// These is the actual structures below which interpret the structures
// deserialized above.

#[derive(Clone, Debug)]
pub struct MintConfig {
    pub primary: Column<InstanceColumn>,
    pub q_add: Selector,
    pub advices: [Column<Advice>; 10],
    pub ecc_config: EccConfig,
    pub poseidon_config: PoseidonConfig<pallas::Base>,
}

impl MintConfig {
    pub fn ecc_chip(&self) -> EccChip<OrchardFixedBases> {
        EccChip::construct(self.ecc_config.clone())
    }

    pub fn poseidon_chip(&self) -> PoseidonChip<pallas::Base> {
        PoseidonChip::construct(self.poseidon_config.clone())
    }
}

#[derive(Clone, Debug)]
pub struct ZkCircuit<'a> {
    pub const_fixed_points: HashMap<String, OrchardFixedBases>,
    pub constants: &'a Vec<(String, ZkType)>,
    pub contract: &'a ZkContract,
    // For each type create a separate stack
    pub witness_base: HashMap<String, Option<pallas::Base>>,
    pub witness_scalar: HashMap<String, Option<pallas::Scalar>>,
}

impl<'a> ZkCircuit<'a> {
    pub fn new(const_fixed_points: HashMap<String, OrchardFixedBases>, constants: &'a Vec<(String, ZkType)>, contract: &'a ZkContract) -> Self {
        let mut witness_base = HashMap::new();
        let mut witness_scalar = HashMap::new();
        for (name, type_id) in contract.witness.iter() {
            match type_id {
                ZkType::Base => { witness_base.insert(name.clone(), None); },
                ZkType::Scalar => { witness_scalar.insert(name.clone(), None); },
                _ => { unimplemented!(); }
            }
        }

        Self {
            const_fixed_points,
            constants,
            contract,
            witness_base,
            witness_scalar,
        }
    }

    pub fn witness_base(&mut self, name: &str, value: pallas::Base) -> Result<()> {
        for (variable, type_id) in self.contract.witness.iter() {
            if name != variable {
                continue;
            }
            if *type_id != ZkType::Base {
                return Err(Error::InvalidParamType);
            }
            *self.witness_base.get_mut(name).unwrap() = Some(value);
            return Ok(());
        }
        return Err(Error::InvalidParamName);
    }

    pub fn witness_scalar(&mut self, name: &str, value: pallas::Scalar) -> Result<()> {
        for (variable, type_id) in self.contract.witness.iter() {
            if name != variable {
                continue;
            }
            if *type_id != ZkType::Scalar {
                return Err(Error::InvalidParamType);
            }
            *self.witness_scalar.get_mut(name).unwrap() = Some(value);
            return Ok(());
        }
        return Err(Error::InvalidParamName);
    }
}

impl<'a> Circuit<pallas::Base> for ZkCircuit<'a> {
    type Config = MintConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self {
            const_fixed_points: self.const_fixed_points.clone(),
            constants: self.constants,
            contract: &self.contract,
            witness_base: self.witness_base.keys().map(|key| (key.clone(), None)).collect(),
            witness_scalar: self.witness_scalar.keys().map(|key| (key.clone(), None)).collect(),
        }
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
    ) -> std::result::Result<(), PlonkError> {
        let mut stack_base = Vec::new();
        let mut stack_scalar = Vec::new();
        //let mut stack_ec_point = Vec::new();
        let mut stack_ec_fixed_point = Vec::new();

        // Load constants first onto the stacks
        for (variable, fixed_point) in self.const_fixed_points.iter() {
            stack_ec_fixed_point.push(*fixed_point);
        }

        // Push the witnesses onto the stacks in order
        for (variable, type_id) in self.contract.witness.iter() {
            match *type_id {
                ZkType::Base => {
                    let value = self.witness_base.get(variable).expect("witness base set");
                    stack_base.push(value.clone());
                },
                ZkType::Scalar => {
                    let value = self.witness_scalar.get(variable).expect("witness base set");
                    stack_scalar.push(value.clone());
                },
                ZkType::EcPoint => {
                    unimplemented!();
                },
                ZkType::EcFixedPoint => {
                    unimplemented!();
                }
            }
        }

        for func_call in self.contract.code.iter() {
            println!("{:?}", func_call);
        }

        // At this point we've enforced all of our public inputs.
        Ok(())
    }
}

