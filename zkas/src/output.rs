use anyhow::Result;
use std::io::{stdout, Write};

use crate::compiler::CompiledContract;
use crate::state::Constants;

pub fn text_output(contracts: Vec<CompiledContract>, constants: Constants) -> Result<()> {
    let mut f = stdout();
    f.write_all(b"Constants\n")?;

    for variable in constants.variables() {
        let type_id = constants.lookup(variable.to_string());
        f.write_all(format!("  {:#?} {}\n", type_id, variable).as_bytes())?;
    }

    for contract in contracts {
        f.write_all(format!("{}:\n", contract.name).as_bytes())?;

        f.write_all(b"  Witness:\n")?;
        for (type_id, variable, _) in contract.witness {
            f.write_all(format!("    {:#?} {}\n", type_id, variable).as_bytes())?;
        }

        f.write_all(b"  Code:\n")?;
        for code in contract.code {
            f.write_all(format!("    # args = {:?}\n", code.args).as_bytes())?;
            f.write_all(
                format!(
                    "    {:?} {:?} {:?}\n",
                    code.func_format.func_id, code.return_values, code.arg_idxs
                )
                .as_bytes(),
            )?;
        }
    }

    Ok(())
}

pub fn bincode_output(
    _filename: &str,
    _contracts: Vec<CompiledContract>,
    _constants: Constants,
) -> Result<()> {
    Ok(())
}
