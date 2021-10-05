use std::fs::File;
use std::time::Instant;

use drk::serial::Decodable;

type Result<T> = std::result::Result<T, failure::Error>;

fn main() -> Result<()> {
    let start = Instant::now();
    let file = File::open("proof/mint.zk.bin")?;
    let zkbin = drk::vm2::ZkBinary::decode(file)?;
    for contract_name in zkbin.contracts.keys() {
        println!("{}", contract_name);
    }
    println!("Loaded contract: [{:?}]", start.elapsed());
    Ok(())
}

