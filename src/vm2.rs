use std::collections::HashMap;

pub enum ZkType {
    Base,
    Scalar,
    EcPoint,
    EcConstPointShort,
    EcConstPoint,
}

type RetValIdx = usize;
type ArgIdx = usize;

pub enum ZkFunctionCall {
    PoseidonHash(RetValIdx, (ArgIdx, ArgIdx)),
    Add(RetValIdx, (ArgIdx, ArgIdx)),
    ConstrainInstance(ArgIdx),
    EcMulShort(RetValIdx, (ArgIdx, ArgIdx)),
    EcMul(RetValIdx, (ArgIdx, ArgIdx)),
    EcAdd(RetValIdx, (ArgIdx, ArgIdx)),
    EcGetX(RetValIdx, ArgIdx),
    EcGetY(RetValIdx, ArgIdx),
}

pub struct ZkBinary {
    pub constants: Vec<ZkType>,
    pub contracts: HashMap<String, ZkContract>,
}

pub struct ZkContract {
    pub witness: Vec<ZkType>,
    pub code: Vec<ZkFunctionCall>,
}
