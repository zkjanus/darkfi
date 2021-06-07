use autocxx::include_cpp;
use cxx::UniquePtr;
use std::{
    pin::Pin,
    path::PathBuf,
};


include_cpp! {
    #include "bitcoin/system.hpp"

    safety!(unsafe_ffi)

    generate!("libbitcoin::system::to_chunk")
    generate!("libbitcoin::system::data_chunk")
    generate!("libbitcoin::system::pseudo_random_fill")
    //generate!("libbitcoin::system::ec_secret")
    //generate!("libbitcoin::system::byte_array")
    //generate!("libbitcoin::system::wallet::hd_key")
    //generate!("libbitcoin::system::wallet::hd_private")
    generate!("libbitcoin::byte_bits")
}

#[cxx::bridge]
mod bcffi {

    unsafe extern "C++" {
        include!("bitcoin/system.hpp");
        include!("helpers.hpp");
        #[namespace = "libbitcoin::system::wallet"]
        type hd_private;

        #[namespace = "darkfi"]
        fn new_private_key() -> UniquePtr<hd_private>;
    }

}

fn new_seed(bit_length: u8) -> UniquePtr<ffi::libbitcoin::system::data_chunk> {
    let fill_seed_size: u8 = bit_length / ffi::libbitcoin::byte_bits as u8;
    let mut seed = ffi::libbitcoin::system::to_chunk(fill_seed_size);
    let pin = seed.pin_mut();

    ffi::libbitcoin::system::pseudo_random_fill(pin);
    // Return the UniquePtr or the Pin?
    seed
}


fn main() {

    let seed = new_seed(192);

}
