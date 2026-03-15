{
  description: "auto-cpufreq-rust: A high-performance Linux CPU optimizer in Rust/Tauri";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustVersion = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        nativeBuildInputs = with pkgs; [
          pkg-config
          rustVersion
          nodejs
          nodePackages.npm
          wrapGAppsHook3
        ];

        buildInputs = with pkgs; [
          webkitgtk_4_1
          gtk3
          libayatana-appindicator
          libsoup_3
          openssl
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs buildInputs;

          shellHook = ''
            export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath buildInputs}:$LD_LIBRARY_PATH
            echo "auto-cpufreq-rust dev environment ready"
          '';
        };
      }
    );
}
