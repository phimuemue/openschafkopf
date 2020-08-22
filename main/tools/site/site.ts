enum EPlayerIndex { EPI0=0, EPI1, EPI2, EPI3, } // TODO can we simplify enum interop with serde?
let EPlayerIndex_SIZE = 4; // TODO derive this from enum

enum SCard {
    E7, E8, E9, EZ, EU, EO, EK, EA,
    G7, G8, G9, GZ, GU, GO, GK, GA,
    H7, H8, H9, HZ, HU, HO, HK, HA,
    S7, S8, S9, SZ, SU, SO, SK, SA,
}

interface Cards {
    veccard : Array<SCard>,
}

class Ask {
    str_question: string;
    vecstrgamephaseaction: Array<[string, any]>;
}
class Ask_ {
    Ask: Ask;
}

function isAsk(msg: string | Ask_) : msg is Ask_ {
    return (msg as Ask_).Ask !== undefined;
}
function getAsk(msg: string | Ask_) : Ask | null {
    if (isAsk(msg)) {
        return msg.Ask;
    } else {
        return null;
    }
}

class SDisplayedStichPrev {
    mapepistr_card: Array<string>;
}
class SDisplayedStichCurrent {
    epi_first: EPlayerIndex; // also denotes winner index of ostichprev
    vecstr_card: Array<string>;
}
class SDisplayedStichs {
    stichcurrent: SDisplayedStichCurrent;
    ostichprev: null | SDisplayedStichPrev;
}
class SSiteState {
    readonly vectplstrstr_caption_message_zugeben: Array<[string, string]>;
    readonly msg: string | Ask_;
    readonly odisplayedstichs: null | SDisplayedStichs;
    readonly mapepistr: Array<string>;
    readonly otplepistr_rules: null | [EPlayerIndex, string]
    readonly oepi_timeout: null | EPlayerIndex;
}

function assert(b) {
    if (!b) {
        throw {};
    }
}

function dbg(t) {
    console.log(t);
    return t;
}

function new_div_with_id(str_id: string) {
    let div = document.createElement("DIV");
    div.id = str_id;
    return div;
}

function new_div_card_in_stich(epi: EPlayerIndex, str_card: string) {
    let div_card = document.createElement("DIV");
    div_card.className = "card_stich card_stich_" + epi + " card";
    div_card.className += " card_" + str_card;
    return div_card;
}

function new_div_card_in_hand(str_card: string) {
    let div_card = document.createElement("DIV");
    div_card.className = "card card_hand card_" + str_card;
    return div_card;
}

function set_animationDuration_if(div: HTMLElement, b: boolean) {
    if (b) {
        div.style.animationDuration = "250ms";
    } else {
        div.style.animationDuration = "0s";
    }
}

function replace_div_with(div_old: HTMLElement, div_new: HTMLElement) {
    div_old.parentNode.replaceChild(div_new, div_old);
}

let str_player_name = prompt("Name:");
let ws = new WebSocket("ws://localhost:8080");
ws.onopen = function(event) {
    ws.send(JSON.stringify({"PlayerLogin": {"str_player_name": str_player_name}}));
};
ws.onmessage = function(msg) {
    let sitestate = dbg(JSON.parse(msg.data) as SSiteState); // assume that server sends valid SSiteState // TODO? assert/check
    {
        let div_hand_new = new_div_with_id("hand");
        for (let tplstrstr of sitestate.vectplstrstr_caption_message_zugeben) {
            let div_card = new_div_card_in_hand(tplstrstr[0]);
            div_hand_new.appendChild(div_card);
            (<HTMLElement>div_card).onclick = function () {
                // TODO if (!player is active) { check } else
                ws.send(JSON.stringify({"GamePhaseAction": dbg(tplstrstr[1])}));
            };
        }
        replace_div_with(document.getElementById("hand"), div_hand_new);
    }
    let div_askpanel = document.getElementById("askpanel");
    let oask = getAsk(sitestate.msg);
    if (oask) {
        dbg("ASK: " + oask.vecstrgamephaseaction[0]);
    }
    if (oask && oask.vecstrgamephaseaction) { // TODO is this the canonical emptiness check?
        dbg("ASK: " + oask);
        let div_askpanel_new = new_div_with_id("askpanel");
        let paragraph_title = document.createElement("p");
        paragraph_title.appendChild(document.createTextNode(oask.str_question));
        div_askpanel_new.appendChild(paragraph_title);
        let paragraph_btns = document.createElement("p");
        for (let x of oask.vecstrgamephaseaction) {
            dbg(x);
            let btn = document.createElement("BUTTON");
            btn.appendChild(document.createTextNode(JSON.stringify(x[0])));
            btn.onclick = function () {
                ws.send(JSON.stringify({"GamePhaseAction": dbg(x[1])}));
            };
            paragraph_btns.appendChild(btn);
            div_askpanel_new.appendChild(paragraph_btns);
            //window.scrollTo(0, document.body.scrollHeight);
        }
        replace_div_with(div_askpanel, div_askpanel_new);
    } else {
        div_askpanel.hidden = true;
    }
    if (dbg(sitestate.odisplayedstichs)) {
        let displayedstichs = sitestate.odisplayedstichs;
        // current stich
        let stichcurrent = displayedstichs.stichcurrent;
        let epi_animate_card = (stichcurrent.epi_first + stichcurrent.vecstr_card.length - 1) % EPlayerIndex_SIZE;
        dbg("Most recent card: " + epi_animate_card);
        let div_stich_new = new_div_with_id("stich");
        for (let i = 0; i<4; i++) {
            if (stichcurrent.vecstr_card[i]) {
                let epi = (stichcurrent.epi_first + i) % EPlayerIndex_SIZE;
                let div_card = new_div_card_in_stich(epi, stichcurrent.vecstr_card[i]);
                set_animationDuration_if(div_card, epi_animate_card==epi);
                div_stich_new.appendChild(div_card);
            }
        }
        replace_div_with(document.getElementById("stich"), div_stich_new);
        // previous stich
        let div_stich_prev = new_div_with_id("stich_old");
        if (displayedstichs.ostichprev) {
            let stichprev = displayedstichs.ostichprev;
            for (let epi = 0; epi<4; epi++) {
                div_stich_prev.appendChild(
                    new_div_card_in_stich(epi, stichprev.mapepistr_card[epi])
                );
            }
            set_animationDuration_if(div_stich_prev, 0==displayedstichs.stichcurrent.vecstr_card.length);
            div_stich_prev.className = "stich_old_" + dbg(displayedstichs.stichcurrent.epi_first);
        }
        replace_div_with(document.getElementById("stich_old"), div_stich_prev);
    }
    {
        dbg(sitestate.mapepistr);
        dbg(sitestate.oepi_timeout);
        for (let i_epi = 0; i_epi<4; i_epi++) {
            let div_player = document.getElementById("playerpanel_player_" + i_epi);
            div_player.textContent = sitestate.mapepistr[i_epi];
            if (dbg(sitestate.otplepistr_rules) && i_epi==sitestate.otplepistr_rules[0]) {
                div_player.textContent += ": " + sitestate.otplepistr_rules[1];
            }
            if (sitestate.oepi_timeout===i_epi) {
                div_player.className = "playerpanel_active";
            } else {
                div_player.className = "";
            }
        }
    }
    {
    }
};
