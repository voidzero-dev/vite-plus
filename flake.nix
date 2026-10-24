{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    mr-boxington.url = "github:jdx/mr-boxington";
    mr-boxington.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      mr-boxington,
      ...
    }:
    let
      systems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ];
      forAllSystems =
        f:
        nixpkgs.lib.genAttrs systems (
          system:
          f (
            import nixpkgs {
              inherit system;
              overlays = [ rust-overlay.overlays.default ];
            }
          )
        );
    in
    {
      formatter = forAllSystems (pkgs: pkgs.nixfmt-tree);

      devShells = forAllSystems (
        pkgs:
        let
          mbx = mr-boxington.packages.${pkgs.stdenv.hostPlatform.system}.default;
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              nodejs_22
              pnpm_12
              just
              cmake
              git
              curl
              mbx

              (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
              # tools that `just init` would install via cargo-binstall
              watchexec
              cargo-insta
              typos
              cargo-shear
              dprint
              taplo
              # no-op shim: the tools above are provided by this devShell
              (writeShellScriptBin "cargo-binstall" ''
                echo "cargo-binstall: skipped (tools are provided by the nix devShell)" >&2
              '')
            ];
            shellHook = ''
              export PATH="${mbx}/libexec/mbx:$PATH"
              # mbx manages its own rustc wrapper; do not stack it with sccache.
              export RUSTC_WRAPPER=""
              export CARGO_BUILD_RUSTC_WRAPPER=""
            '';
          };
        }
      );
    };
}
