use lazy_static::lazy_static;
use std::collections::HashMap;

const TYPE_ID_BASE: usize = 0;
const TYPE_ID_SCALAR: usize = 1;
const TYPE_ID_EC_POINT: usize = 2;
const TYPE_ID_EC_FIXED_POINT: usize = 3;
const TYPE_ID_LAST: usize = 4;

#[derive(Debug)]
pub enum AllowedTypes {
    Base,
    Scalar,
    EcFixedPoint,
}

const FUNC_ID_POSEIDON_HASH: usize = 0;
const FUNC_ID_ADD: usize = 1;
const FUNC_ID_CONSTRAIN_INSTANCE: usize = 2;
const FUNC_ID_EC_MUL_SHORT: usize = 3;
const FUNC_ID_EC_MUL: usize = 4;
const FUNC_ID_EC_ADD: usize = 4;
const FUNC_ID_EC_GET_X: usize = 4;
const FUNC_ID_EC_GET_Y: usize = 4;

#[derive(Debug)]
pub struct FuncFormat {
    func_id: usize,
    return_type_ids: Vec<usize>,
    param_types: Vec<usize>,
}

impl FuncFormat {
    pub fn new(func_id: usize, return_type_ids: Vec<usize>, param_types: Vec<usize>) -> Self {
        FuncFormat {
            func_id,
            return_type_ids,
            param_types,
        }
    }

    pub fn total_arguments(&self) -> usize {
        self.return_type_ids.len() + self.param_types.len()
    }
}

lazy_static! {
    static ref FUNCTION_FORMATS: HashMap<&'static str, FuncFormat> = {
        let mut map = HashMap::new();

        map.insert(
            "poseidon_hash",
            FuncFormat::new(
                FUNC_ID_POSEIDON_HASH,
                vec![TYPE_ID_BASE],
                vec![TYPE_ID_BASE, TYPE_ID_BASE],
            ),
        );

        map.insert(
            "add",
            FuncFormat::new(
                FUNC_ID_ADD,
                vec![TYPE_ID_BASE],
                vec![TYPE_ID_BASE, TYPE_ID_BASE],
            ),
        );

        map.insert(
            "constrain_instance",
            FuncFormat::new(
                FUNC_ID_CONSTRAIN_INSTANCE,
                vec![],
                vec![TYPE_ID_BASE, TYPE_ID_BASE],
            ),
        );

        map.insert(
            "ec_mul_short",
            FuncFormat::new(
                FUNC_ID_EC_MUL_SHORT,
                vec![TYPE_ID_EC_POINT],
                vec![TYPE_ID_BASE, TYPE_ID_EC_FIXED_POINT],
            ),
        );

        map.insert(
            "ec_mul",
            FuncFormat::new(
                FUNC_ID_EC_MUL,
                vec![TYPE_ID_EC_POINT],
                vec![TYPE_ID_EC_POINT, TYPE_ID_EC_POINT],
            ),
        );

        map.insert(
            "ec_add",
            FuncFormat::new(
                FUNC_ID_EC_ADD,
                vec![TYPE_ID_EC_POINT],
                vec![TYPE_ID_EC_POINT, TYPE_ID_EC_POINT],
            ),
        );

        map.insert(
            "ec_get_x",
            FuncFormat::new(FUNC_ID_EC_GET_X, vec![TYPE_ID_BASE], vec![TYPE_ID_EC_POINT]),
        );

        map.insert(
            "ec_get_y",
            FuncFormat::new(FUNC_ID_EC_GET_Y, vec![TYPE_ID_BASE], vec![TYPE_ID_EC_POINT]),
        );

        map
    };
}
