cargo build --target wasm32-unknown-unknown --release
~/.cargo/bin/wasm-bindgen target/wasm32-unknown-unknown/debug/garbage_collector2.wasm --out-dir html --target web
