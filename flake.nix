{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
      };
    in
    {
      packages.x86_64-linux.default = pkgs.rustPlatform.buildRustPackage {
        pname = "nyaa-bulk";
        version = "0.2.0";

        nativeBuildInputs = with pkgs; [
          pkg-config
        ];

        buildInputs = with pkgs; [
          openssl
        ];

        src = ./.;
        cargoSha256 = "sha256-OtSbmcxhfPavc8ea1WV6+deFiZi3FYTr9JTkNcxk9dw=";
      };

      devShells.x86_64-linux.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          cargo
          nixd
          openssl
          pkg-config
          rust-analyzer
          rustc
        ];
      };
    };
}
