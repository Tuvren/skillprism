{ pkgs, lib, config, inputs, ... }:
{
  packages = with pkgs; [ just hugo ];

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
    hugo version
  '';
}
