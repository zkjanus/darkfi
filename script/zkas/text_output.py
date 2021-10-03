from .types import type_id_to_name, func_id_to_name

def output(output, schema):
    for name, witness, code in schema:
        output.write(f"{name}:\n")

        output.write(f"  Witness:\n")
        for type_id, variable, _ in witness:
            type_name = type_id_to_name[type_id]
            output.write(f"    {type_name} {variable}\n")

        output.write(f"  Code:\n")
        for func_fmt, return_vals, args, _ in code:
            func_name = func_id_to_name[func_fmt.func_id]
            output.write(f"    {func_name} {return_vals}\n")

