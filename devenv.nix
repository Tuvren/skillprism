{ pkgs, lib, config, inputs, ... }:
{
  packages = with pkgs; [ just ];

  languages.rust.enable = true;

  enterShell = ''
    echo "skillprism dev environment ready"
    rustc --version
    cargo --version
  '';
}
