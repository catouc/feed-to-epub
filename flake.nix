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
        pname = "rss-to-epub";
        version = "0.2.0";

        nativeBuildInputs = with pkgs; [
          pkg-config
        ];

        buildInputs = with pkgs; [
	  sqlite
          openssl
        ];

        src = ./.;
        cargoSha256 = "sha256-lEl2JuFHsYxLAoGMLZVzAESFrqJvVulk78URyMG37hE=";
      };

      devShells.x86_64-linux.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          cargo
	  rustPackages.clippy
	  rustPackages.rustfmt
          nixd
          openssl
          pkg-config
	  sqlite
	  litecli
          rust-analyzer
          rustc
        ];
      };
    };
}
