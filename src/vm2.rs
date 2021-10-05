use std::collections::HashMap;

pub enum ZkType {
    Base,
    Scalar,
    EcPoint,
    EcConstPointShort,
    EcConstPoint,
}

pub struct ZkBinary {
    pub constants: Vec<ZkType>,
    pub contracts: HashMap<String, ZkContract>
}

pub struct ZkContract {
}

