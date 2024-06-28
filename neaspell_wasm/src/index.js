import init, {load_language, spell_text} from '../pkg/neaspell_wasm.js';
import {testData} from "./test_data.js"
// html element ID attrbute values
const LOAD_LANGUAGE_BUTTONS_ID = "dicts_id";
const LOAD_RESULT_ID = "load_result_id";
const TEXT_PARA_ID = "text_para_id";

let DICT_URL_PREFIX = "https://raw.githubusercontent.com/LibreOffice/dictionaries/master";
let LANG_ARRAY = [
    // [0] a part of URL past DICT_URL_PREFIX, before extension ".dic" or ".aff"
    ["es/es_ES"],
    ["en/en_US"],
];

async function run() {
    await init();
}
run();

async function loadLangExample(event) {
    let buttonElem = event.srcElement;
    let langIx = parseInt (buttonElem.getAttribute("dict_ix"), 10);
    let [dictRelS] = LANG_ARRAY[langIx];
    let dictId = buttonElem.getAttribute("id").substring("dict_".length);
    let urlBaseS = `${DICT_URL_PREFIX}/${dictRelS}`;
    let loadSummary = "";
    try {
        let dicFetch = await fetch (urlBaseS+".dic");
        let affFetch = await fetch (urlBaseS+".aff");
        if (dicFetch.ok && affFetch.ok) {
            let dicText = await dicFetch.text();
            let dicLineA = dicText.split("\n");
            let dicLineCount = dicLineA.length;
            let affText = await affFetch.text();
            let affLineA = affText.split("\n");
            let affLineCount = affLineA.length;
            let notes = load_language (dictId, affLineA, dicLineA);
            loadSummary = `Language ${dictId}: ${affLineCount} aff lines, ${dicLineCount} dic lines, ${notes.length} notes`;
        }
        let langTestdata = testData[dictId];
        if (langTestdata) {
            let [creditUrl, testHtmlS] = langTestdata;
            let paraElem = document.getElementById(TEXT_PARA_ID);
            paraElem.innerText = testHtmlS;
            loadSummary += `, test words from ${creditUrl}`
        }
    } catch (e) {
        loadSummary = `Language ${dictId}: problem while loading: ${e.message}`;
    }
    let loadResultElem = document.getElementById(LOAD_RESULT_ID);
    loadResultElem.innerText = loadSummary;

}

async function createLoadDictionaryButtons (langA) {
    let container = document.getElementById(LOAD_LANGUAGE_BUTTONS_ID); 
    for (let langIx=0; langIx < langA.length; langIx++) {
        // e.g. dictRelS="es/es_ES"
        let [dictRelS, testUrl] = langA[langIx];
        let button = document.createElement("button");
        let dictIdS = dictRelS.split("/").pop();
        button.setAttribute("id", "dict_"+dictIdS);
        button.setAttribute("dict_ix", `${langIx}`);
        button.innerText = dictIdS;
        button.onclick = loadLangExample;
        container.appendChild(button);
    }    
}

async function spellCheckWords () {
    let checkText = document.getElementById(TEXT_PARA_ID).innerText.trim();
    checkText = checkText.replaceAll ("+", ""). replaceAll("#", "");
    let wasmResult = spell_text (checkText);
    document.getElementById(TEXT_PARA_ID).innerText = wasmResult.join("");
}

document.getElementById("check_btn").addEventListener("click", spellCheckWords, false);
createLoadDictionaryButtons (LANG_ARRAY);
