{
  description = "Development environment for zOS";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      llvmPkgs = pkgs.llvmPackages_18;
    in
    {
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = [
          # Rust toolchain
          pkgs.rustup

          # C/C++ toolchain for linking
          pkgs.clang
          llvmPkgs.libclang
          llvmPkgs.llvm
          pkgs.binutils
          pkgs.gcc

          # Frontend dependencies
          pkgs.nodejs_20 # For Svelte/npm

          # Project runner
          pkgs.just

          # System libraries for backend crates
          pkgs.linux-pam # For the `pam` crate
        ];

        # Environment variables for Rust bindings
        LIBCLANG_PATH = "${llvmPkgs.libclang.lib}/lib";
        LLVM_CONFIG_PATH = "${llvmPkgs.llvm.dev}/bin/llvm-config";
      };
    };
}
