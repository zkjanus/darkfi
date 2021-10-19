# default.nix
{ lib
, pkgs
, naersk
, stdenv
, clangStdenv
, hostPlatform
, targetPlatform
, pkg-config
, libiconv
, rustfmt
, cargo
, rustc
, llvmPackages
}:

let
  cargoToml = (builtins.fromTOML (builtins.readFile ./Cargo.toml));
in

naersk.lib."${targetPlatform.system}".buildPackage rec {
  src = ./.;

  nativeBuildInputs = with pkgs; [
    rustfmt
    llvm
    clang
    protobuf
    pkg-config
  ];
  buildInputs = with pkgs; [
    rustfmt
    pkg-config
    cargo
    rustc
    libiconv
    libclang
    openssl.dev
    libudev
  ];
  checkInputs = [ cargo rustc ];

  doCheck = false;
  CARGO_BUILD_INCREMENTAL = "false";
  RUST_BACKTRACE = "full";
  copyLibs = true;

  # https://hoverbear.org/blog/rust-bindgen-in-nix/
  preBuild = with pkgs; ''
    export BINDGEN_EXTRA_CLANG_ARGS="$(< ${stdenv.cc}/nix-support/libc-crt1-cflags) \
    $(< ${stdenv.cc}/nix-support/libc-cflags) \
    $(< ${stdenv.cc}/nix-support/cc-cflags) \
    $(< ${stdenv.cc}/nix-support/libcxx-cxxflags) \
    ${lib.optionalString stdenv.cc.isClang "-idirafter ${stdenv.cc.cc}/lib/clang/${lib.getVersion stdenv.cc.cc}/include"} \
    ${lib.optionalString stdenv.cc.isGNU "-isystem ${stdenv.cc.cc}/include/c++/${lib.getVersion stdenv.cc.cc} -isystem ${stdenv.cc.cc}/include/c++/${lib.getVersion stdenv.cc.cc}/${stdenv.hostPlatform.config} -idirafter ${stdenv.cc.cc}/lib/gcc/${stdenv.hostPlatform.config}/${lib.getVersion stdenv.cc.cc}/include"} \
    "
  '';
  LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
  LLVM_CONFIG_PATH = "${pkgs.llvm}/bin/llvm-config";

  #cargoBuildFlags = builtins.map (binName: "--bin=${binName}") endUserBins;

  name = cargoToml.package.name;
  version = cargoToml.package.version;

  meta = with lib; {
    description = cargoToml.package.description;
    homepage = cargoToml.package.homepage;
    license = with licenses; [ mit ];
    maintainers = with maintainers; [ ];
  };
}
