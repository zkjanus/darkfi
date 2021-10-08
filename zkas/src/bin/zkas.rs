use anyhow::Result;
use clap::clap_app;
use std::fs::File;
use std::io::{BufRead, BufReader};

use zkas::parser::load_lines;

fn main() -> Result<()> {
    let args = clap_app!(zkas =>
        (@arg INPUT: +required "ZK script to compile")
        (@arg DISPLAY: -f --format +takes_value "Output format (bincode/readable)")
        (@arg OUTPUT: -o --output +takes_value "Output file")
    )
    .get_matches();

    let file = File::open(args.value_of("INPUT").unwrap())?;
    let loaded = load_lines(BufReader::new(file).lines());

    println!("{:#?}", loaded);

    Ok(())
}
