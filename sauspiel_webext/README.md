# What's this

This code can be compiled to a web extension, that - when enabled while browsing [`sauspiel.de/spiele/`](https://www.sauspiel.de/spiele) - automatically runs a (limited) analysis of the game.

## Compiling/running

*Note:* I only tested this on a recent Firefox (version >= 120), using [`web-ext`](https://github.com/mozilla/web-ext).

```
# starting in the repositories root folder

# convert Rust code to JavaScript/WebAssembly that will be included in the extension
cd sauspiel_webext
./run_wasm_pack_build.sh

# run the web extension
cd webext
./webext/run_webext_in_browser.sh
```
