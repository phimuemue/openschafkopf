importScripts('sauspiel_webext.js');
console.log("after import");
//console.log("worker got", msg.data);
//self.postMessage("Msg back " + msg.data);

/*
async function run() {
    let x = await wasm_bindgen("sauspiel_webext_bg.wasm");
    x.worker_function();
}
run();

//wasm_bindgen.worker_function();
console.log("after worker_function");
*/

onmessage = (e) => {
    let data = e.data;
    console.log("Worker: Message received from main script");

    console.log(data);

    postMessage(data);
    console.log("Worker: Sent back to main script");
};
