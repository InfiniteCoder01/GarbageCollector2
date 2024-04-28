{ pkgs ? import <nixpkgs> { } }:
with pkgs; mkShell {
  inputsFrom = [];
  buildInputs = [ (python3.withPackages (ps: with ps [pillow])) ];
}
