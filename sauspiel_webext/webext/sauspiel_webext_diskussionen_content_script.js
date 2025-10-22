async function run() {
    let x = await wasm_bindgen(browser.runtime.getURL("sauspiel_webext_bg.wasm"));
    x.parse_and_extend_diskussionen_start_worker();
}
run();
