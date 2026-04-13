pkg_local := x"~/.local/share/typst"
pkg_contrib := "packages/preview/bulb"

toolchain:
  rustup target add wasm32-unknown-unknown --toolchain nightly

install version: (contribute version pkg_local "local")

# Build and copy all necessary files to local typst/pakcages fork
contribute version typst-packages scope="preview":
  #!/usr/bin/env bash
  set -euo pipefail
  
  target_dir="{{typst-packages}}/packages/{{scope}}/bulb/{{version}}"
  
  printf "\x1b[33mChecking if manifests list version {{version}}...\x1b[0m\n"
  head -5 Cargo.toml |  grep 'version = "{{version}}"' > /dev/null
  head -5 typst/typst.toml |  grep 'version = "{{version}}"' > /dev/null
  printf "\x1b[34mCheck!\x1b[0m\n\n"

  printf "\x1b[33mBuilding WASM target with nightly and -Zlocation-detail=none...\x1b[0m\n"
  RUSTFLAGS="-Zlocation-detail=none" cargo +nightly build --release --target wasm32-unknown-unknown
  printf "\x1b[34mBuilt!\x1b[0m\n\n"

  test -d {{typst-packages}} || { echo -e "\x1b[31m{{typst-packages}} isn't a directory\x1b[0m"; exit 1; }

  printf "\x1b[33mCopying files to ${target_dir}...\x1b[0m\n"
  mkdir -p "$target_dir"
  cp target/wasm32-unknown-unknown/release/bulb_typst.wasm \
    typst/typst.toml \
    typst/*.typ \
    ./LICENSE \
    README.md \
    "$target_dir"
  printf "\x1b[34mDone!\x1b[0m\n\n"
