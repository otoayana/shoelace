{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
  };

  outputs = { self, flake-utils, naersk, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = (import nixpkgs) {
          inherit system;
        };

        naersk' = pkgs.callPackage naersk { };
      in
      {
        # For `nix build` & `nix run`:
        defaultPackage = naersk'.buildPackage {
          src = ./.;

          nativeBuildInputs = [ pkgs.makeWrapper ];
          buildInputs = with pkgs; [ cacert llvmPackages.libstdcxxClang openssl pkg-config ];

          # Includes LLVM for bundling RocksDB in
          stdenv = pkgs.clangStdenv;
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          
          # Add CA roots, since otherwise Shoelace can't send requests
          postInstall = ''
            wrapProgram $out/bin/shoelace --set SSL_CERT_FILE "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
          '';
        };

        # For `nix develop`:
        devShell = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            rustc
            cargo
            rustfmt
            rust-analyzer
            clippy
            pkg-config
            dbus
            openssl
            clangStdenv
            llvmPackages.libstdcxxClang
          ];

          stdenv = pkgs.clangStdenv;
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
        };
      });
}
