{
  pkgs ? import <nixpkgs> { },
}:

pkgs.buildNpmPackage {
  pname = "ents-admin-ui";
  version = "0.1.0";

  src = ./.;

  npmDepsHash = "sha256-sXktjMWXp1uBdc2OEAuXMwdSD3YHzD6aGn3JE76v20A=";

  buildPhase = ''
    runHook preBuild
    npm run build
    runHook postBuild
  '';

  installPhase = ''
    runHook preInstall
    mkdir -p $out
    cp -r dist/* $out/
    runHook postInstall
  '';

  meta = {
    description = "Ents Admin UI";
  };
}
