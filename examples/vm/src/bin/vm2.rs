use std::fs::File;
use std::time::Instant;

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
    Ok(())
}
