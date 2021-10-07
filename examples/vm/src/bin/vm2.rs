use std::fs::File;
use std::time::Instant;
use rand::rngs::OsRng;
use halo2::{
    arithmetic::Field,
    pasta::pallas
};

use drk::serial::Decodable;

type Result<T> = std::result::Result<T, failure::Error>;

fn main() -> Result<()> {
    let start = Instant::now();
    let file = File::open("../../proof/mint.zk.bin")?;
    let zkbin = drk::vm2::ZkBinary::decode(file)?;
    for contract_name in zkbin.contracts.keys() {
        println!("Loaded '{}' contract.", contract_name);
    }
    println!("Load time: [{:?}]", start.elapsed());

    let contract = &zkbin.contracts["Mint"];

    //contract.witness_base(...);
    //contract.witness_base(...);
    //contract.witness_base(...);

    let pub_x = pallas::Base::random(&mut OsRng);
    let pub_y = pallas::Base::random(&mut OsRng);
    let value = pallas::Base::random(&mut OsRng);
    let asset = pallas::Base::random(&mut OsRng);
    let serial = pallas::Base::random(&mut OsRng);
    let coin_blind = pallas::Base::random(&mut OsRng); 
    let value_blind = pallas::Scalar::random(&mut OsRng);  
    let asset_blind = pallas::Scalar::random(&mut OsRng); 

    let mut circuit = drk::vm2::ZkCircuit::new(contract);
    circuit.witness_base("pub_x", pub_x)?;
    circuit.witness_base("pub_y", pub_y)?;
    circuit.witness_base("value", value)?;
    circuit.witness_base("asset", asset)?;
    circuit.witness_base("serial", serial)?;
    circuit.witness_base("coin_blind", coin_blind)?;
    circuit.witness_scalar("value_blind", value_blind)?;
    circuit.witness_scalar("asset_blind", asset_blind)?;

    Ok(())
}
