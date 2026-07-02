{ pkgs, lib, config, inputs, ... }:

let
  # Pin the same Hugo Extended version used by the deploy workflow so local
  # builds are reproducible with CI. Prebuilt binary; update the hash when
  # bumping HUGO_VERSION in .github/workflows/deploy-site.yml.
  hugo-extended = pkgs.stdenv.mkDerivation rec {
    pname = "hugo";
    version = "0.163.3";
    src = pkgs.fetchurl {
      url = "https://github.com/gohugoio/hugo/releases/download/v${version}/hugo_extended_${version}_linux-amd64.tar.gz";
      sha256 = "0ymz42r1785shpnq9rc14jfxf34fi10l1gn5l7xjlnd4x5crjwr5";
    };
    sourceRoot = ".";
    installPhase = ''
      mkdir -p $out/bin
      cp hugo $out/bin/
    '';
  };
in
{
  packages = with pkgs; [ just hugo-extended ];

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
