{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    { nixpkgs, rust-overlay, ... }:
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

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            nodejs_22
            pnpm_11
            just
            cmake

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
        };
      });
    };
}
