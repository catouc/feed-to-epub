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
        pname = "feed-to-epub";
        version = "0.2.0";

        nativeBuildInputs = with pkgs; [
          pkg-config
        ];

        buildInputs = with pkgs; [
          sqlite
          openssl
        ];

        src = ./.;
        cargoSha256 = "sha256-igZhahS1/ueCwYwlKUl+WM4dqeiP4t3t3DoOwyPWVII=";
      };

      overlay = final: prev: {
        feed-to-epub = self.packages.x86_64-linux.default;
      };

      nixosModules.default = {config, lib, pkgs, ... }:
      let
        cfg = config.feed-to-epub;
      in {
        options.feed-to-epub = {
          enable = lib.mkEnableOption "Enable the feed to epub service";
          downloadDir = lib.mkOption {
            type = lib.types.str;
            default = "/var/feed-to-epub/";
            description = "The location of the working directory, not created by the binary.";
          };

          user = lib.mkOption {
            type = lib.types.str;
            default = "feed-to-epub";
            description = "Name of the user and group that are used for the service";
          };

          settings = lib.mkOption {
            type = lib.types.attrs;
            default = {};
            description = "List of feeds we want to pull";
          };
        };

        config = lib.mkIf cfg.enable {
          users.groups."${cfg.user}" = {};
          users.users."${cfg.user}" = {
            isSystemUser = true;
            group = cfg.user;
          };

          environment.etc."/feed-to-epub/config.toml" = {
            source = pkgs.writers.writeTOML "config.toml" cfg.settings;
          };

          systemd.services.feed-to-epub = {
            serviceConfig = {
              Type = "simple";
              ExecStart = "${pkgs.feed-to-epub}/bin/feed-to-epub --config /etc/feed-to-epub/config.toml";
              User = cfg.user;
              WorkingDirectory = cfg.downloadDir;
            };
          };
        };
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
