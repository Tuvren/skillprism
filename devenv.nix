{ pkgs, lib, config, inputs, ... }:
{
  packages = with pkgs; [ just ];

  languages.rust = {
    enable = true;
    channel = "stable";
  };

  enterShell = ''
    echo "skillprism dev environment ready"
    rustc --version
    cargo --version
  '';
}
