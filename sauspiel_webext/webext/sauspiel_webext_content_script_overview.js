const exactly_one = (htmlcol) => {
    console.assert(htmlcol.length===1);
    return htmlcol.item(0)
}

const worker_main = (glue, wasm) => {
    importScripts(glue);
    const ready = wasm_bindgen(wasm);
    self.onmessage = async ({ data: arg, ports: [port] }) => {
        try {
            await ready;
            // replace `analyze` with your exported wasm function
            console.log("Before postMessage " + arg);
            port.postMessage({ result: wasm_bindgen.analyze(arg) });
        } catch (e) {
            port.postMessage({ error: String(e) });
        }
    };
};

const worker = new Worker(URL.createObjectURL(new Blob(
    [`
        (${worker_main})(
            ${JSON.stringify(browser.runtime.getURL("sauspiel_webext.js"))},
            ${JSON.stringify(browser.runtime.getURL("sauspiel_webext_bg.wasm"))}
        )
    `],
    { type: "text/javascript" }
)));


const add_details_to_games_item_header = async (div_games_item) => {
    const div_games_item_header = exactly_one(div_games_item.getElementsByClassName("games-item-header"));
    console.log(div_games_item_header);
    const str_url = 
        exactly_one(
            exactly_one(div_games_item_header.getElementsByTagName("h4"))
                .getElementsByTagName("a")
        ).getAttribute("href");
    const details = document.createElement("details");
    details.textContent = "Loading…";
    div_games_item_header.appendChild(details);

    const { port1, port2 } = new MessageChannel();
    const reply = new Promise((resolve) => { port1.onmessage = ({ data }) => resolve(data); });
    console.log("Outer: " + str_url);
    worker.postMessage(str_url, [port2]);
    const { result, error } = await reply;
    details.textContent = error ? "Error: " + error : result;
};

/*const add_details_to_games_item_header = (div_games_item) => {
    const div_games_item_header = exactly_one(div_games_item.getElementsByClassName("games-item-header"));
    const str_url = exactly_one(div_games_item_header.getElementsByTagName("h4")).getAttribute("href");
    let details = document.createElement("details");
    details.innerHtml = "TODO: Fill in details for " + str_url;
    // TODO implement analysis in spiel overview
    div_games_item_header.appendChild(details);
}*/

const div_games_results = document.getElementById("games_results");

for (const div_games_item of div_games_results.getElementsByClassName("games-item")) {
    add_details_to_games_item_header(div_games_item);
}

new MutationObserver((mutationList) => {
    for (const mutation of mutationList) {
        for (const node of mutation.addedNodes) {
            if (node.tagName==="DIV") {
                console.assert(node.className==="card games-item");
                console.log(node);
                add_details_to_games_item_header(/*div_games_item*/node);
            }
        }
    }
}).observe(div_games_results, /*config*/{ childList: true });

// TODO should we somewhen observer_games_results.disconnect()?
