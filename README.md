An example `metaview` app.

This application demonstrates the ability to move entities around via input events and view orientation.

Prerequisites:
* [https://rustwasm.github.io/wasm-pack/](wasm-pack): A tool for compiling Rust to WASM

Building:
* Add `.glb` files to `resources/showcase` and make sure to give them a name in the following format:
    * `NAME_(SCALE).glb`
    * for example: `my_frog_(0.01).glb`
* Run the following command from the project root:
```sh
rm -rf pkg; env WASM_INTERFACE_TYPES=1 wasm-pack build --release
```
