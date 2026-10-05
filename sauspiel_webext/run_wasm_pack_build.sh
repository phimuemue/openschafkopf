RUSTFLAGS='--cfg getrandom_backend="wasm_js"' wasm-pack build --out-dir webext --target no-modules --no-typescript
