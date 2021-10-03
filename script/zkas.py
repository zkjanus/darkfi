import sys

TYPE_ID_BASE                    = 0
TYPE_ID_SCALAR                  = 1
TYPE_ID_EC_POINT                = 2
TYPE_ID_EC_CONSTANT_POINT_SHORT = 3
TYPE_ID_EC_CONSTANT_POINT       = 4

allowed_types = {
    "Base":                 TYPE_ID_BASE,
    "Scalar":               TYPE_ID_SCALAR,
    "EcFixedPointShort":    TYPE_ID_EC_CONSTANT_POINT_SHORT,
    "EcFixedPoint":         TYPE_ID_EC_CONSTANT_POINT
}
# Used for debug and error messages
types_to_string = dict((value, key) for key, value in allowed_types.items())

FUNC_ID_POSEIDON_HASH           = 0
FUNC_ID_ADD                     = 1
FUNC_ID_CONSTRAIN_INSTANCE      = 2
FUNC_ID_EC_MUL_SHORT            = 3
FUNC_ID_EC_MUL                  = 4
FUNC_ID_EC_ADD                  = 5
FUNC_ID_EC_GET_X                = 6
FUNC_ID_EC_GET_Y                = 7

class FuncFormat:

    def __init__(self, func_id, return_types, param_types):
        self.func_id = func_id
        self.return_types = return_types
        self.param_types = param_types

    def total_arguments(self):
        if self.return_types:
            return 1 + len(self.param_types)
        return len(self.param_types)

function_formats = {
    "poseidon_hash": FuncFormat(
        # Funcion ID            Type ID             Parameter types
        FUNC_ID_POSEIDON_HASH,  [TYPE_ID_BASE],     [TYPE_ID_BASE,
                                                     TYPE_ID_BASE]
    ),
    "add": FuncFormat(
        FUNC_ID_ADD,            [TYPE_ID_BASE],     [TYPE_ID_BASE,
                                                     TYPE_ID_BASE]
    ),
    "constrain_instance": FuncFormat(
        FUNC_ID_CONSTRAIN_INSTANCE, [],             [TYPE_ID_BASE]
    ),
    "ec_mul_short": FuncFormat(
        FUNC_ID_EC_MUL_SHORT,   [TYPE_ID_EC_POINT], [TYPE_ID_BASE,
                                                TYPE_ID_EC_CONSTANT_POINT_SHORT]
    ),
    "ec_mul": FuncFormat(
        FUNC_ID_EC_MUL,         [TYPE_ID_EC_POINT], [TYPE_ID_SCALAR,
                                                     TYPE_ID_EC_CONSTANT_POINT]
    ),
    "ec_add": FuncFormat(
        FUNC_ID_EC_ADD,         [TYPE_ID_EC_POINT], [TYPE_ID_EC_POINT,
                                                     TYPE_ID_EC_POINT]
    ),
    "ec_get_x": FuncFormat(
        FUNC_ID_EC_GET_X,       [TYPE_ID_BASE],     [TYPE_ID_EC_POINT]
    ),
    "ec_get_y": FuncFormat(
        FUNC_ID_EC_GET_Y,       [TYPE_ID_BASE],     [TYPE_ID_EC_POINT]
    ),
}

class CompileException(Exception):

    def __init__(self, error_message, line):
        super().__init__(error_message)
        self.error_message = error_message
        self.line = line

class SyntaxStruct:

    def __init__(self):
        self.contracts = {}
        self.circuits = {}
        self.constants = {}

    def parse_contract(self, line, it):
        assert line.tokens[0] == "contract"
        if len(line.tokens) != 3 or line.tokens[2] != "{":
            raise CompileException("malformed contract opening", line)
        name = line.tokens[1]
        if name in self.contracts:
            raise CompileException(f"duplicate contract {name}", line)
        lines = []

        while True:
            try:
                line = next(it)
            except StopIteration:
                raise CompileException(
    f"premature end of file while parsing {name} contract", line)

            assert len(line.tokens) > 0
            if line.tokens[0] == "}":
                break

            lines.append(line)

        self.contracts[name] = lines

    def parse_circuit(self, line, it):
        assert line.tokens[0] == "circuit"
        if len(line.tokens) != 3 or line.tokens[2] != "{":
            raise CompileException("malformed circuit opening", line)
        name = line.tokens[1]
        if name in self.circuits:
            raise CompileException(f"duplicate contract {name}", line)
        lines = []

        while True:
            try:
                line = next(it)
            except StopIteration:
                raise CompileException(
    f"premature end of file while parsing {name} circuit", line)

            assert len(line.tokens) > 0
            if line.tokens[0] == "}":
                break

            lines.append(line)

        self.circuits[name] = lines

    def parse_constant(self, line):
        assert line.tokens[0] == "constant"
        if len(line.tokens) != 3:
            raise CompileException("malformed constant line", line)
        _, type, name = line.tokens
        if type not in allowed_types:
            raise CompileException("unknown type '{type}'", line)
        type_id = allowed_types[type]
        self.constants[name] = type_id

    def verify(self):
        self.static_checks()
        schema = self.format_data()
        self.trace_circuits(schema)
        return schema

    def static_checks(self):
        for name, lines in self.contracts.items():
            for line in lines:
                if len(line.tokens) != 2:
                    raise CompileException("incorrect number of tokens", line)
                type, variable = line.tokens
                if type not in allowed_types:
                    raise CompileException(
                        f"unknown type specifier for variable {variable}", line)

        for name, lines in self.circuits.items():
            for line in lines:
                assert len(line.tokens) > 0
                func_name, args = line.tokens[0], line.tokens[1:]
                if func_name not in function_formats:
                    raise CompileException(f"unknown function call {func_name}",
                                         line)
                func_format = function_formats[func_name]
                if len(args) != func_format.total_arguments():
                    raise CompileException(
        f"incorrect number of arguments for function call {func_name}", line)

        # Finally check there are matching circuits and contracts
        all_names = set(self.circuits.keys()) | set(self.contracts.keys())

        for name in all_names:
            if name not in self.contracts:
                raise CompileException(f"missing contract for {name}", None)
            if name not in self.circuits:
                raise CompileException(f"missing circuit for {name}", None)

    def format_data(self):
        schema = []
        for name, circuit in self.circuits.items():
            assert name in self.contracts
            contract = self.contracts[name]

            witness = []
            for line in contract:
                assert len(line.tokens) == 2
                type, variable = line.tokens
                assert type in allowed_types
                type = allowed_types[type]
                witness.append((type, variable, line))

            code = []
            for line in circuit:
                assert len(line.tokens) > 0
                func_name, args = line.tokens[0], line.tokens[1:]
                assert func_name in function_formats
                func_format = function_formats[func_name]
                assert len(args) == func_format.total_arguments()

                return_values = []
                if func_format.return_types:
                    rv_len = len(func_format.return_types)
                    return_values, args = args[:rv_len], args[rv_len:]

                func_id = func_format.func_id
                code.append((func_format, return_values, args, line))

            schema.append((name, witness, code))
        return schema

    def trace_circuits(self, schema):
        for name, witness, code in schema:
            tracer = DynamicTracer(name, witness, code, self.constants)
            tracer.execute()

class DynamicTracer:

    def __init__(self, name, contract_witness, circuit_code, constants):
        self.name = name
        self.witness = contract_witness
        self.code = circuit_code
        self.constants = constants

    def execute(self):
        stack = {}

        # Preload stack with our witness values
        for type_id, variable, line in self.witness:
            stack[variable] = type_id

        # Load constants
        for variable, type_id in self.constants.items():
            stack[variable] = type_id

        for i, (func_format, return_values, args, code_line) \
            in enumerate(self.code):

            assert len(args) == len(func_format.param_types)
            for variable, type in zip(args, func_format.param_types):
                if variable not in stack:
                    raise CompileException(
                        f"variable '{variable}' is not defined", code_line)

                stack_type = stack[variable]
                if stack_type != type:
                    type_name = types_to_string[type]
                    stack_type_name = types_to_string[stack_type]
                    raise CompileException(
    f"variable '{variable}' has incorrect type. "
    f"Found {type_name} but expected variable of "
    f"type {stack_type_name}", code_line)

                assert len(return_values) == len(func_format.return_types)

                for return_value, return_type \
                    in zip(return_values, func_format.return_types):

                    # Note that later variables shadow earlier ones.
                    # We accept this.

                    stack[return_value] = return_type

class Line:

    def __init__(self, tokens, original_line, number):
        self.tokens = tokens
        self.orig = original_line
        self.number = number

    def __repr__(self):
        return f"Line({self.number}: {str(self.tokens)})"

def load(src_file):
    source = []
    for i, original_line in enumerate(src_file):
        # Remove whitespace on both sides
        line = original_line.strip().split()
        if not line:
            continue
        line_number = i + 1
        source.append(Line(line, original_line, line_number))
    return source

def parse(source):
    syntax = SyntaxStruct()
    it = iter(source)
    while True:
        try:
            line = next(it)
        except StopIteration:
            break

        assert len(line.tokens) > 0
        if line.tokens[0] == "contract":
            syntax.parse_contract(line, it)
        elif line.tokens[0] == "circuit":
            syntax.parse_circuit(line, it)
        elif line.tokens[0] == "constant":
            syntax.parse_constant(line)
        elif line.tokens[0] == "}":
            raise CompileException("unmatched delimiter '}'", line)
    return syntax

def main(argv):
    with open("mint.zk", "r") as src_file:
        source = load(src_file)
    try:
        syntax = parse(source)
        syntax.verify()
        print("Successful compilation.")
        # todo: serialize data out
    except CompileException as ex:
        print(f"Error: {ex.error_message}", file=sys.stderr)
        if ex.line is not None:
            print(f"Line {ex.line.number}: {ex.line.orig}", file=sys.stderr)
        #return -1
        raise
    return 0

if __name__ == "__main__":
    sys.exit(main(sys.argv))

# todo: think about extendable payment scheme which
# is like bitcoin soft forks
