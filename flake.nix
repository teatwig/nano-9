{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
  };
  outputs = {
    self,
    nixpkgs,
    utils,
  }:
    utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs { inherit system; };
        # inheriting the inputs from the package massively slows down rust-analyzer, so specify them separately
        nativeBuildInputs' = with pkgs; [
          pkg-config
        ];
        buildInputs' = with pkgs; [
          alsa-lib
          libudev-zero
          # runtime
          libxkbcommon
          vulkan-loader
          # runtime x11
          # xorg.libX11
          # xorg.libXcursor
          # xorg.libXi
          # runtime wayland
          wayland
        ];
      in rec {
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [] ++ nativeBuildInputs' ++ buildInputs';

          shellHook = ''
            export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${builtins.toString (pkgs.lib.makeLibraryPath buildInputs')}";
          '';
        };
      }
    );
}
