const exactly_one = (htmlcol) => {
    console.assert(htmlcol.length===1);
    return htmlcol.item(0)
}

const worker_main = (glue, wasm, n_delay_ms) => {
    importScripts(glue);
    const ready = wasm_bindgen(wasm);

    // New DB name: old entries hold full analysis results, not raw JSON
    const db = new Promise((resolve, reject) => {
        const req = indexedDB.open("sauspiel_json_cache", 1);
        req.onupgradeneeded = () => req.result.createObjectStore("json");
        req.onsuccess = () => resolve(req.result);
        req.onerror = () => reject(req.error);
    });
    const idb = async (mode, fn) => {
        const store = (await db).transaction("json", mode).objectStore("json");
        return new Promise((resolve, reject) => {
            const req = fn(store);
            req.onsuccess = () => resolve(req.result);
            req.onerror = () => reject(req.error);
        });
    };

    // Analysis runs on every request (cheap, depends on username); never throttled
    const analyze_and_reply = async (port, str_json, str_username) => {
        try {
            await ready;
            const r = await wasm_bindgen.analyze_sauspiel_json_from_js(str_json, str_username);
            port.postMessage({ result: {
                str_html_simple_game_protocol: r.str_html_simple_game_protocol,
                ostr_color_suboptimal_quality: r.ostr_color_suboptimal_quality,
            }});
            r.free();
        } catch (e) {
            port.postMessage({ error: String(e) });
        }
    };

    let chain = ready.catch(() => {});
    self.onmessage = async ({ data: { str_url, str_username }, ports: [port] }) => {
        const cached = await idb("readonly", (s) => s.get(str_url)).catch(() => undefined);
        if (cached !== undefined) {
            await analyze_and_reply(port, cached, str_username);
            return;
        }
        // Cache miss: only the server request is throttled
        chain = chain.then(async () => {
            try {
                console.log(str_url);
                const str_json = await wasm_bindgen.request_sauspiel_json(str_url + ".json");
                await idb("readwrite", (s) => s.put(str_json, str_url)).catch(console.error);
                await analyze_and_reply(port, str_json, str_username);
            } catch (e) {
                port.postMessage({ error: String(e) });
            }
            await new Promise((r) => setTimeout(r, n_delay_ms));
        });
    };
};

const worker = new Worker(URL.createObjectURL(new Blob(
    [`
        (${worker_main})(
            ${JSON.stringify(browser.runtime.getURL("sauspiel_webext.js"))},
            ${JSON.stringify(browser.runtime.getURL("sauspiel_webext_bg.wasm"))},
            /*n_delay_ms*/3000 + Math.random()*1000
        )
    `],
    { type: "text/javascript" }
)));



const add_details_to_games_item_header = async (div_games_item) => {
    const div_games_item_header = exactly_one(div_games_item.getElementsByClassName("games-item-header"));
    //console.log(div_games_item_header);
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
    //console.log("Outer: " + str_url);
    worker.postMessage({str_url: new URL(str_url, location.href).href, str_username: "smile4free"}, [port2]);
    const { result, error } = await reply;
    // console.assert(!error);
    //console.log(result);
    details.innerHTML = error ? "Error: " + error : result.str_html_simple_game_protocol;
    if (result.ostr_color_suboptimal_quality!==undefined) {
        div_games_item.style.borderLeft = "10px solid " + result.ostr_color_suboptimal_quality;
    }
};

const div_games_results = document.getElementById("games_results");

for (const div_games_item of div_games_results.getElementsByClassName("games-item")) {
    add_details_to_games_item_header(div_games_item);
}

new MutationObserver((mutationList) => {
    for (const mutation of mutationList) {
        for (const node of mutation.addedNodes) {
            if (node.tagName==="DIV") {
                console.assert(node.className==="card games-item");
                //console.log(node);
                add_details_to_games_item_header(/*div_games_item*/node);
            }
        }
    }
}).observe(div_games_results, /*config*/{ childList: true });

// TODO should we somewhen observer_games_results.disconnect()?
