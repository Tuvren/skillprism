{ pkgs, lib, config, inputs, ... }:
{
  packages = with pkgs; [ just ];

  languages.rust.enable = true;

  git-hooks.hooks = {
    rustfmt.enable = true;
    clippy.enable = true;
    clippy.settings.denyWarnings = true;
  };

  enterShell = ''
    echo "skillprism dev environment ready"
    rustc --version
    cargo --version
  '';
}
