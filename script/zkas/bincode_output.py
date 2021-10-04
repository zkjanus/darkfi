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
    output.write(varuint(len(contract_name)))
    output.write(contract_name.encode())

def output(output, contracts, constants):
    for contract in contracts:
        output_contract(output, contract)

