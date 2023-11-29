document.body.style.border = "5px solid red";

async function run() {
    await wasm_bindgen(browser.runtime.getURL("sauspiel_webext_bg.wasm"));
}
run();
