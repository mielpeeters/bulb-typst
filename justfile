pkg := "~/.local/share/typst/packages/local/bulb/0.1.0"

toolchain:
    rustup target add wasm32-unknown-unknown --toolchain nightly

install:
    RUSTFLAGS="-Zlocation-detail=none" cargo +nightly build --release --target wasm32-unknown-unknown
    mkdir -p {{pkg}}
    cp target/wasm32-unknown-unknown/release/bulb_typst.wasm {{pkg}}/
    cp typst/typst.toml typst/lib.typ {{pkg}}/
