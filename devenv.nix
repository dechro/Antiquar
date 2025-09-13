{
  pkgs,
  lib,
  config,
  ...
}:
{
  languages.rust.enable = true;
  # languages.c.enable = true;
  packages = [
    pkgs.cmake
    pkgs.pkg-config
    pkgs.mesa-gl-headers
    pkgs.mesa
    pkgs.libGL
    pkgs.libGLU
    pkgs.libxkbcommon
    pkgs.fontconfig
    pkgs.slint-lsp
    # pkgs.slint-viewer
  ];
  env = {
    LD_LIBRARY_PATH = "/run/current-system/sw/lib:/usr/lib";
  };
}
