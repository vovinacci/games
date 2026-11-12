# NixOS: miniquad loads libGL, libX11, libXi and libxkbcommon at run time (dlopen), so they must be on
# LD_LIBRARY_PATH. Use: `nix-shell --run "cargo run -p <game>"`; the Makefile does this by itself on NixOS.
#
# Copied from Scorch and not yet run on NixOS from this repository.
{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  # The commit message check (tools/verify_commit.sh), and Python for the tools in requirements.txt (`make tools`).
  packages = with pkgs; [ cocogitto python315 ];
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [
    libGL
    libx11
    libxi
    libxkbcommon
  ]);
}
