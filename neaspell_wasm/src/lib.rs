use neaspell_core::{core_speller::{Spell, SpellLang, TokenType}, text_parser::{LineReader, TextParser}};
use wasm_bindgen::prelude::*;
use std::sync::{Mutex, OnceLock};

struct WasmLineReader {
    pub wlr_base_name: String,
    pub wlr_extension: String,
    wlr_reader: Vec<String>,
    wlr_next_line_index: usize,
}

impl WasmLineReader {
    pub fn new(slr_base_name: &str, slr_extension:&str, text:Vec<String>) -> WasmLineReader {
        WasmLineReader {
            wlr_base_name: slr_base_name.to_string(),
            wlr_extension: slr_extension.to_string(),
            wlr_reader: text,
            wlr_next_line_index: 0
        }
    }
}

impl LineReader for WasmLineReader {
    fn get_base_name(&self) -> String {
        self.wlr_base_name.clone()
    }
    fn get_extension(&self) -> String {
        self.wlr_extension.clone()
    }
    fn read_line(&mut self) -> Option<Vec::<u8>> {
        if self.wlr_next_line_index >= self.wlr_reader.len() {
            return None;
        }
        let line = &self.wlr_reader[self.wlr_next_line_index];
        self.wlr_next_line_index+= 1;
        let byte_vec: Vec::<u8> = line.as_bytes().into_iter().map(|b| 0+b).collect();
        Some(byte_vec)
    }
}

struct WorkSet {
    ws_spell_lang: SpellLang,
    ws_text_parser: TextParser,
}


impl WorkSet {
    pub fn new() -> WorkSet {
        WorkSet {
            ws_spell_lang: SpellLang::new(""),
            ws_text_parser: TextParser::new(),
        }
    }

    pub fn load_language (&mut self, base_name: &str, aff_text:Vec<String>, dic_text:Vec<String>) -> Vec<String> {
        //let mut ws_spell_lang = SpellLang::new(base_name);
        let mut notes: Vec<String> = vec![];
        // aff file
        let mut aff_line_reader = WasmLineReader::new(base_name, TextParser::EXT_AFF, aff_text);
        self.ws_text_parser.parse_dictionary_text(&mut self.ws_spell_lang, &mut aff_line_reader);
        for line_note in &self.ws_text_parser.tps_line_notes {
            notes.push (line_note.clone());
        }
        self.ws_text_parser.tps_line_notes.clear();
        // dic file
        let mut dic_line_reader = WasmLineReader::new(base_name, TextParser::EXT_DIC, dic_text);
        self.ws_text_parser.parse_dictionary_text(&mut self.ws_spell_lang, &mut dic_line_reader);
        for line_note in &self.ws_text_parser.tps_line_notes {
            notes.push (line_note.clone());
        }
        notes
    }

    pub fn spell_text (&mut self, text:String) -> Vec<(String, TokenType)> {
        Spell::check_text (&self.ws_spell_lang, &text)
    }
}

fn get_work_set() -> &'static Mutex<WorkSet> {
    static WORK_SET: OnceLock<Mutex<WorkSet>> = OnceLock::new();
    WORK_SET.get_or_init(|| Mutex::new(WorkSet::new()))
}

#[wasm_bindgen]
pub fn load_language(base_name: &str, aff_text:Vec<String>, dic_text:Vec<String>) -> Vec<String> {
    let notes = get_work_set().lock().unwrap().load_language(base_name, aff_text, dic_text);
    notes
}

#[wasm_bindgen]
pub fn spell_text(text:String) -> Vec<String> {
    let spelled_tokens: Vec<(String, TokenType)> = get_work_set().lock().unwrap().spell_text(text);
    // wasm currently doesn't allow returning vector of tuples
    // so let's encode tuple into string
    let wasm_result: Vec<String> = spelled_tokens.iter().
        map(|it|
            {if it.1 == TokenType::IsGoodWord {"+"}
            else if it.1 == TokenType::IsBadWord {"#"}
            else{""}}
            .to_string() + &it.0 ).collect();
    wasm_result
}

#[wasm_bindgen(start)]
fn main() -> Result<(), JsValue> {
    Ok(())
}
