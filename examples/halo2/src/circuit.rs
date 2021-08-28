use halo2::{
    pasta::pallas,
    plonk::{Advice, Column, Instance as InstanceColumn, Selector},
};

use halo2_ecc::chip::{EccChip, EccConfig};
use halo2_poseidon::pow5t3::{Pow5T3Chip as PoseidonChip, Pow5T3Config as PoseidonConfig};
use orchard::constants::{OrchardCommitDomains, OrchardFixedBases, OrchardHashDomains};
use sinsemilla::chip::{SinsemillaChip, SinsemillaConfig};

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
pub struct BurnConfig {
    pub primary: Column<InstanceColumn>,
    pub q_add: Selector,
    pub advices: [Column<Advice>; 10],
    pub ecc_config: EccConfig,
    pub sinsemilla_config:
        SinsemillaConfig<OrchardHashDomains, OrchardCommitDomains, OrchardFixedBases>,
    pub poseidon_config: PoseidonConfig<pallas::Base>,
}

impl BurnConfig {
    pub fn ecc_chip(&self) -> EccChip<OrchardFixedBases> {
        EccChip::construct(self.ecc_config.clone())
    }

    pub fn sinsemilla_chip(
        &self,
    ) -> SinsemillaChip<OrchardHashDomains, OrchardCommitDomains, OrchardFixedBases> {
        SinsemillaChip::construct(self.sinsemilla_config.clone())
    }

    pub fn poseidon_chip(&self) -> PoseidonChip<pallas::Base> {
        PoseidonChip::construct(self.poseidon_config.clone())
    }
}
