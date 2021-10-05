use std::io;

use crate::impl_vec;
use crate::error::{Error, Result};
use crate::vm2::{ZkType, ZkBinary, ZkContract};
use crate::serial::{Decodable, Encodable, ReadExt, VarInt};

impl_vec!(ZkType);
impl_vec!((String, ZkContract));

impl Encodable for ZkType {
    fn encode<S: io::Write>(&self, _s: S) -> Result<usize> {
        unimplemented!();
        //Ok(0)
    }
}

impl Decodable for ZkType {
    fn decode<D: io::Read>(mut d: D) -> Result<Self> {
        let op_type = ReadExt::read_u8(&mut d)?;
        match op_type {
            0 => Ok(Self::Base),
            1 => Ok(Self::Scalar),
            2 => Ok(Self::EcPoint),
            3 => Ok(Self::EcConstPointShort),
            4 => Ok(Self::EcConstPoint),
            _i => Err(Error::BadOperationType),
        }
    }
}

impl Encodable for ZkBinary {
    fn encode<S: io::Write>(&self, _s: S) -> Result<usize> {
        unimplemented!();
        //Ok(0)
    }
}

impl Decodable for ZkBinary {
    fn decode<D: io::Read>(mut d: D) -> Result<Self> {
        Ok(Self {
            constants: Decodable::decode(&mut d)?,
            contracts: Vec::<(String, ZkContract)>::decode(&mut d)?
                .into_iter()
                .collect(),
        })
    }
}

impl Encodable for ZkContract {
    fn encode<S: io::Write>(&self, _s: S) -> Result<usize> {
        unimplemented!();
        //Ok(0)
    }
}

impl Decodable for ZkContract {
    fn decode<D: io::Read>(mut d: D) -> Result<Self> {
        Ok(Self {
        })
    }
}

