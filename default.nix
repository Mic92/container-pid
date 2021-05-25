with import <nixpkgs> {};
mkShell {
  nativeBuildInputs = [
    bashInteractive
    cargo
    cargo-watch
    rustc
  ];
}
