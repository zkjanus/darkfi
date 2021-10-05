import struct

def varuint(value):
    if value <= 0xfc:
        return struct.pack("<B", value)
    elif value <= 0xffff:
        return struct.pack("<BH", 0xfd, value)
    elif value <= 0xffffffff:
        return struct.pack("<BI", 0xfe, value)
    else:
        return struct.pack("<BQ", 0xff, value)

def output_contract(output, contract):
    output.write(varuint(len(contract.name)))
    output.write(contract.name.encode())

def output(output, contracts, constants):
    output.write(varuint(len(constants.variables())))
    for variable in constants.variables():
        type_id = constants.lookup(variable)
        type_id_bytes = struct.pack("<B", type_id)
        assert len(type_id_bytes) == 1
        output.write(type_id_bytes)
    output.write(varuint(len(contracts)))
    for contract in contracts:
        output_contract(output, contract)

