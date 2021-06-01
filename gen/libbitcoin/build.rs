// Copyright 2020 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

fn main() {
    let libbitcoin = pkg_config::probe_library("libbitcoin-system").unwrap();
    let libbitcoinc = pkg_config::probe_library("libbitcoin-client").unwrap();

    // It's necessary to use an absolute path here because the
    // C++ codegen and the macro codegen appears to be run from different
    // working directories.
    //let path = std::path::PathBuf::from("s2geometry/src");
    let path = std::path::PathBuf::from("src");
    let mut b = autocxx_build::build("src/main.rs", &[&path], &[]).unwrap();
    b.flag_if_supported("-std=c++14")
        .includes(libbitcoin.include_paths)
        .includes(libbitcoinc.include_paths)
        .compile("libbitcoin-rs");

    for link_path in libbitcoin.link_paths {
        println!("cargo:rustc-link-search={}", link_path.to_str().unwrap());
    }
    for lib in libbitcoin.libs {
        println!("cargo:rustc-link-lib={}", lib);
    }
    for link_path in libbitcoinc.link_paths {
        println!("cargo:rustc-link-search={}", link_path.to_str().unwrap());
    }
    for lib in libbitcoinc.libs {
        println!("cargo:rustc-link-lib={}", lib);
    }

    println!("cargo:rerun-if-changed=src/main.rs");
}
