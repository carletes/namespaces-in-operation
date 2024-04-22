{
  description = "UNIX course";

  inputs.nixpkgs.url = "nixpkgs/nixos-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            mdbook
            rustup

            # Keep this line if you use bash.
            bashInteractive
          ] ++ lib.optionals stdenv.isDarwin [
            iconv
          ];

          shellHook = ''
            export CARGO_HOME="$(pwd)/.cargo"
            export RUSTUP_HOME="$(pwd)/.rustup"
            export PATH="$CARGO_HOME/bin:$PATH"

            rustup toolchain install --component rust-analyzer,rust-src --no-self-update stable
            rustup default stable
          '';
        };
      });
}
