async function run() {
    await wasm_bindgen(browser.runtime.getURL("sauspiel_webext_bg.wasm"));
    wasm_bindgen.analyze_spielbericht();
}
run();
