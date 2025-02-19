cargo build --release --target wasm32-unknown-unknown --package icrc7

candid-extractor target/wasm32-unknown-unknown/release/icrc7.wasm > src/icrc7/icrc7.did


# ic-wasm target/wasm32-unknown-unknown/release/icrc7.wasm -o target/wasm32-unknown-unknown/release/icrc7.wasm shrink
# gzip -f -c target/wasm32-unknown-unknown/release/icrc7.wasm > wasm/icrc7.wasm.gz