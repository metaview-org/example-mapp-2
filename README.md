An example `metaview` app.

Prerequisites:
* [https://rustwasm.github.io/wasm-pack/](wasm-pack): A tool for compiling Rust to WASM

Building:
```sh
rm -rf pkg; env WASM_INTERFACE_TYPES=1 wasm-pack build --release
```
