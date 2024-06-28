use crate::core_speller::{
    HashMap, HashSet,AffixEntry, AffixClass, DicEntry, FlagFormat, FlagNameAndType, FlagType, FlaggedWord, SpellLang,
};
use std::str::SplitWhitespace;

// The text dictionary files are read with such a trait.
pub trait LineReader {
    fn get_base_name(&self) -> String;
    fn get_extension(&self) -> String;
    fn get_full_name(&self) -> String {self.get_base_name() + "." + &self.get_extension()}
    fn read_line(&mut self, ) -> Option<Vec::<u8>>;
}

/// Comment on a single line or a problem.
pub struct ParseNote {
    pub psn_line_no: u32, // 0 no data; when given > 0
    pub psn_desc: &'static str,
    pub psn_details: Option<String>, // displayed on a separate line, after description's line
}

#[derive(PartialEq, Clone, Copy)]
pub enum ParseStatus {
    /// line is correctly encoded and non-empty
    LineReady,
    // line empty, commented or incorrectly encoded
    EncodingErrorOrEmpty,
    /// end of file or reading error
    FileEnded,
}

/// An example with text line and its tokenization together.
#[allow(dead_code)]
pub struct ParsedLine<'a> {
    pln_line: &'a str,
    pln_tokens: SplitWhitespace<'a>,
}

impl<'a> ParsedLine<'a> {
    #[allow(dead_code)]
    pub fn new(pln_line: &'a str) -> ParsedLine<'a> {
        ParsedLine::<'a> {
            pln_line,
            pln_tokens: pln_line.split_whitespace(),
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum ParseMode {
    /// the line starts with a tag, as is in the .aff file
    Toplevel,
    /// the line contains word(s) of the .dic file or the initial count of the words
    WordDic,
    /// grammar descriptions causing errors
    TestBadGram,
    /// words passing the spelling rules
    TestGoodWords,
    /// words failing the spelling rules
    TestBadWords,
}

/// While parsing a single dictionary line.
pub struct LineParseState<'a> {
    /// line number in the file, starting with 1
    lps_line_no: u32,
    /// remaining tokens in the line
    lps_tokens: &'a mut SplitWhitespace<'a>,
    /// the first token in the line is often used as keyword
    lps_first_token: Option<&'a str>,
    /// warnings and explanations of error handling
    lps_notes: Vec<ParseNote>,
}

impl<'a> LineParseState<'a> {
    pub fn new(pst_line_no: u32, pst_tokens: &'a mut SplitWhitespace<'a>) -> LineParseState<'a> {
        LineParseState::<'a> {
            lps_line_no: pst_line_no,
            lps_tokens: pst_tokens,
            lps_first_token: None,
            lps_notes: vec![],
        }
    }

    pub fn add_note(&mut self, desc: &'static str) {
        self.lps_notes.push(ParseNote {
            psn_line_no: self.lps_line_no,
            psn_desc: desc,
            psn_details: None,
        })
    }

    pub fn add_note2(&mut self, desc: &'static str, detail: &String) {
        self.lps_notes.push(ParseNote {
            psn_line_no: self.lps_line_no,
            psn_desc: desc,
            psn_details: Some(detail.clone()),
        })
    }

    pub fn get_next_token(&mut self) -> Option<&str> {
        self.lps_tokens.next()
    }

    /// The function is expected to be called when the token is known to be present.
    /// It returns the token
    pub fn get_first_token(&mut self) -> &str {
        if let None = self.lps_first_token {
            self.lps_first_token = self.lps_tokens.next();
            if let None = self.lps_first_token {
                self.lps_first_token = Some("");
            }
        }
        if let Some(token) = self.lps_first_token {
            return token;
        }
        "" // not reachable, but compiler doesn't know it and needs something
    }

    pub fn get_notes(&self) -> &Vec<ParseNote> {
        &self.lps_notes
    }

    pub fn get_note_length(&self) -> usize {
        self.lps_notes.len()
    }
}

pub struct Parser {}
impl Parser {
    /// Parses string with multiple flags.
    /// With FLAG UTF-8, each flag is one character, multiple flags are not separated.
    /// With FLAG long, each flag is two characters, multiple flags are not separated
    /// With FLAG num, each flag is an unsigned number, multiple flags are separated by commas
    fn parse_flags(spell_lang: &SpellLang, flags: &str) -> Vec<String> {
        if flags.len() == 0 {
            return vec![];
        }
        if spell_lang.slg_flag == FlagFormat::SingleUni {
            // one-character flags
            return flags.chars().map(|c| c.to_string()).collect();
        }
        if spell_lang.slg_flag == FlagFormat::DoubleChar {
            // two-character flags
            let mut flag_vec: Vec<String> = vec![];
            let mut flag_chars = "".to_string();
            for c in flags.chars() {
                if flag_chars.is_empty() {
                    flag_chars = c.to_string();
                } else {
                    flag_chars.push(c);
                    flag_vec.push(flag_chars.clone());
                    flag_chars = "".to_string();
                }
            }
            return flag_vec;
        }
        if spell_lang.slg_flag == FlagFormat::Numeric {
            return flags.split(",").map(|s| s.to_string()).collect();
        }
        vec![]
    }

    /// Parses COMPOUNDRULE string with multiple flags.
    /// Asterisk, question mark and parenthesis are regex characters.
    /// SingleChar and SingleUni flags are all the remaining characters: mn*t,
    /// DoubleChar and Numeric flags are enclosed in parentheses.
    /// Returns the vector of flags.
    fn parse_compoundrule_flags(spell_lang: &SpellLang, flags: &str) -> Vec<String> {
        if spell_lang.slg_flag == FlagFormat::SingleUni {
            // one-character flags
            return flags
                .chars()
                .map(|fl| fl.to_string())
                .filter(|fl| fl != "*" && fl != "?")
                .collect();
        }
        vec![]
    }

    /// Parses the tag without value, acting as bool.
    /// If no errors, it updates "variab" to "value".
    /// The "note" is set to Some if a message is to be issued.
    /// It returns true if the tag was procesed.
    fn parse_bool(spell_lang: &mut SpellLang, parse_state: &mut LineParseState) -> bool {
        let parse_table= [
            ("COMPLEXPREFIXES", &mut spell_lang.slg_cplx_pref, false, true),
            ("NOSPLITSUGS", &mut spell_lang.slg_sug_split, false, false),
            ("SUGSWITHDOTS", &mut spell_lang.slg_sug_dots, true, false),
            ("CHECKCOMPOUNDDUP", &mut spell_lang.slg_comp_check_dup, true, false),
            ("CHECKCOMPOUNDREP", &mut spell_lang.slg_comp_check_rep, true, false),
            ("CHECKCOMPOUNDCASE", &mut spell_lang.slg_comp_check_case, true, false),
            ("CHECKSHARPS", &mut spell_lang.slg_check_sharp_s, true, false),
            ("CHECKCOMPOUNDTRIPLE", &mut spell_lang.slg_check_comp_triple, true, false),
            ("SIMPLIFIEDTRIPLE", &mut spell_lang.slg_simplified_triple, true, false),
            ("ONLYMAXDIFF", &mut spell_lang.slg_only_max_diff, true, false),
            ("FULLSTRIP", &mut spell_lang.slg_full_string, true, false),
            ("COMPOUNDMORESUFFIXES", &mut spell_lang.slg_comp_more_suffixes, true, false),
                    ];
        let mut result = false;
        let mut is_complex_prefixes = false;
        for (tag, variab, value, arg3_complex_prefixes) in parse_table {
            if tag == parse_state.get_first_token() {
                let tokens: Vec<&str> = parse_state.lps_tokens.collect();
                if tokens.len() > 0 && !tokens[0].starts_with("#") {
                    parse_state.add_note("Unexpected argument");
                }
                *variab = value;
                result = true;
                is_complex_prefixes = arg3_complex_prefixes;
                break;
            }
        }
        if is_complex_prefixes {
            spell_lang.slg_prefix_max = 2;
            spell_lang.slg_suffix_max = 1;
        }
        result
    }

    /// Parses the tag and its string value.
    /// If no errors, it updates "variab".
    /// The "parse_state.note" is set to Some if a message is to be issued.
    /// It returns true if the tag was procesed.
    fn parse_string(spell_lang: &mut SpellLang, parse_state: &mut LineParseState) -> bool {
        let parse_table = [
            ("TRY", &mut spell_lang.slg_try, false),
            ("LANG", &mut spell_lang.slg_code, false),
            ("KEY", &mut spell_lang.slg_key, false),
            ("WORDCHARS", &mut spell_lang.tag_wordchars, true),
            ("IGNORE", &mut spell_lang.slg_ignore, false),
            ("NAME", &mut spell_lang.slg_name, false),
            ("HOME", &mut spell_lang.slg_home, false),
            ("VERSION", &mut spell_lang.slg_version, false),
        ];
        let mut result = false;
        let mut is_wordchars = false;
        for (tag, variab, arg2_wordchars) in parse_table {
            if tag == parse_state.get_first_token() {
                if let Some(try_value) = parse_state.lps_tokens.next() {
                    *variab = try_value.to_string();
                } else {
                    parse_state.add_note("Missing value");
                }
                result = true;
                is_wordchars = arg2_wordchars;
                break;
            }
        }
        if is_wordchars {
            Parser::parse_wordchars(spell_lang);
        }
        result
    }

    /// Parses the tag and its unsigned numeric value.
    /// If no errors, it updates "variab".
    /// The "parse_state.note" is set to Some if a message is to be issued.
    /// It returns true if the tag was procesed.
    fn parse_number(spell_lang: &mut SpellLang, parse_state: &mut LineParseState) -> bool {
        let parse_table = [
            ("COMPOUNDMIN", &mut spell_lang.slg_comp_min),
            ("COMPOUNDWORDMAX", &mut spell_lang.slg_comp_word_max),
            ("MAXCPDSUGS", &mut spell_lang.slg_max_cpd_sugs),
            ("MAXNGRAMSUGS", &mut spell_lang.slg_max_ngram_sugs),
            ("MAXDIFF", &mut spell_lang.slg_max_diff),
        ];
        let mut result = false;
        for (tag, variab) in parse_table {
            if tag == parse_state.get_first_token() {
                if let Some(number_value) = parse_state.lps_tokens.next() {
                    let number_value = number_value.parse::<u32>();
                    if let Ok(number_value) = number_value {
                        *variab = number_value;
                    } else {
                        parse_state.add_note("Expected number");
                    }
                } else {
                    parse_state.add_note("Missing value");
                }
                result = true;
                break;
            }
        }
        result
    }

    /// Parses the tag with an array of String values.
    /// If no errors, it updates "select_value".
    /// The "note" is set to Some if a message is to be issued.
    /// It returns true if the tag was procesed.
    fn parse_string_table(
        spell_lang: &mut SpellLang, 
        parse_state: &mut LineParseState,
    ) -> bool {
        let parse_table = [
            ("MAP", &mut spell_lang.slg_map),
            ("BREAK",&mut spell_lang.slg_break),
        ];
        let mut result = false;
        for (tag, variab) in parse_table {
            if tag == parse_state.get_first_token() {
                // MAP 5
                // MAP aáAÁ
                let tokens: Vec<&str> = parse_state.lps_tokens.collect();
                if tokens.len() < 1 {
                    parse_state.add_note("Missing argument");
                    return true;
                }
                if !variab.1 {
                    // header tag
                    let group_size = tokens[0].parse::<u32>();
                    if let Ok(group_size) = group_size {
                        _ = variab.0.try_reserve(group_size as usize);
                    } else {
                        parse_state.add_note("Expected number");
                    }
                    variab.1 = true;
                } else {
                    variab.0.push(tokens[0].to_string());
                }
                if tokens.len() > 1 && !tokens[1].starts_with("#") {
                    parse_state.add_note("Expected one argument");
                }
                result = true;
                break;
            }
        }
        result
    }

    /// Parses the tag with an array of (String,String) values.
    /// If no errors, it updates "select_value".
    /// The "note" is set to Some if a message is to be issued.
    /// It returns true if the tag was procesed.
    fn parse_pair_table(
        spell_lang: &mut SpellLang,
        parse_state: &mut LineParseState,
    ) -> bool {
        let parse_table = [
            ("REP", &mut spell_lang.slg_rep),
            ("PHONE", &mut spell_lang.slg_phone),
            ("ICONV", &mut spell_lang.slg_iconv),
            ("OCONV", &mut spell_lang.slg_oconv),
        ];
        let mut result = false;
        for (tag, variab) in parse_table {
            if tag == parse_state.get_first_token() {
                // REP 20 # replacement table
                // REP ke que
                let tokens: Vec<&str> = parse_state.lps_tokens.collect();
                let is_first = tokens.len() == 1 || tokens.len() >= 2 && tokens[1].starts_with("#");
                if is_first {
                    let group_size = tokens[0].parse::<u32>();
                    if let Ok(group_size) = group_size {
                        _ = variab.try_reserve(group_size as usize);
                    }
                } else {
                    if tokens.len() < 2 {
                        parse_state.add_note("Not enough arguments, expected two");
                    }
                    if tokens.len() >= 2 {
                        variab.push((tokens[0].to_string(), tokens[1].to_string()));
                    }
                    if tokens.len() > 2 && !tokens[2].starts_with("#") {
                        parse_state.add_note("Expected two arguments");
                    }
                }
                result = true;
                break;
            }
        }
        result
    }

    fn parse_simple_flag(
        spell_lang: &mut SpellLang,
        simple_flag_table: &[FlagNameAndType],
        parse_state: &mut LineParseState,
    ) -> bool {
        let simple_ix = simple_flag_table
            .iter()
            .position(|sf| parse_state.get_first_token() == sf.0);
        if let Some(simple_ix) = simple_ix {
            // a name of simple COMPOUND* (COMPOUND_FLAG etc) and similar tag
            let flag_type = &simple_flag_table[simple_ix].1;
            if let Some(comp_flag) = parse_state.lps_tokens.next() {
                spell_lang
                    .slg_flag_hash
                    .insert(String::from(comp_flag), (flag_type.clone(), 0));
            } else {
                parse_state.add_note("No flag value for element");
            }
            return true;
        }
        false
    }

    fn parse_affix(spell_lang: &mut SpellLang, parse_state: &mut LineParseState, is_prefix: bool) {
        let tokens: Vec<&str> = parse_state.lps_tokens.collect();
        if tokens.len() < 3 {
            // any PFX or SFX element, initial or not, should have at least three
            // arguments after the tag name
            parse_state.add_note("Less than 3 tokens for PFX or SFX");
            return;
        }
        let mut is_first = spell_lang.slg_aff_groups.len() == 0
            || spell_lang.slg_aff_groups.last().unwrap().is_complete();
        if !is_first
            && spell_lang.slg_aff_groups.len() != 0
            && spell_lang.slg_aff_groups.last().unwrap().afc_name != tokens[0]
        {
            // the documentation seems to imply that the group_size in the initial
            // element is precise, but let's rely on the name of the affix group
            is_first = true;
            // we'll issue a message
            parse_state.add_note(
                "According to the affix header count, the previous affix class is not yet full",
            );
        }
        if is_first {
            // PFX f Y 6
            // SFX A Y 14
            let group_name = tokens[0].to_string();
            let can_circum = tokens[1] == "Y";
            let group_size = tokens[2].parse::<u32>();
            if let Ok(group_size) = group_size {
                let mut affix_group =
                    AffixClass::build_affix_group(group_name, is_prefix, can_circum, group_size);
                affix_group.afc_ix = spell_lang.slg_aff_groups.len() as u32;
                if is_prefix {
                    spell_lang.slg_pfxes.push(affix_group.afc_ix);
                } else {
                    spell_lang.slg_sfxes.push(affix_group.afc_ix);
                }
                spell_lang.slg_aff_groups.push(affix_group);
            } else {
                parse_state.add_note("Bad class size in the PFX or SFX header");
            }
            if tokens.len() >= 4 {
                if !tokens[3].starts_with("#") {
                    parse_state.add_note("Superfluous tokens in the PFX or SFX header");
                }
            }
        } else {
            // PFX f 0 con [^abehilopru]
            // SFX A r ción/S ar
            let mut sub = tokens[1];
            if sub == "0" {
                // the empty-string element is defined with string "0"
                sub = "";
            }
            let mut add_next = tokens[2];
            if add_next == "0" {
                // the empty-string element is defined with string "0"
                add_next = "";
            }
            // afe_next_flags
            let add_parts: Vec<&str> = add_next.split("/").collect();
            let add = String::from(if add_parts.len() >= 1 {
                add_parts[0]
            } else {
                ""
            });
            let next = String::from(if add_parts.len() >= 2 {
                add_parts[1]
            } else {
                ""
            });
            let mut affix_entry = AffixEntry::new(
                sub.to_string(),
                add,
                Parser::parse_flags(&spell_lang, &next),
                if tokens.len() < 4 {
                    "".to_string()
                } else {
                    tokens[3].to_string()
                },
            );
            if let Some(desc) = affix_entry.afe_cond.rgx_error {
                parse_state.add_note(desc.0); // todo add column number desc.1
                return;
            }
            let aff_groups: &mut Vec<AffixClass> = &mut spell_lang.slg_aff_groups;
            let last_aff_group: &mut AffixClass = aff_groups.last_mut().unwrap();
            affix_entry.afe_ix = last_aff_group.afc_affixes.len() as u32;
            if last_aff_group.afc_affixes.len() < last_aff_group.afc_size as usize {
                last_aff_group.add_entry(affix_entry);
            } else {
                parse_state.add_note("too many affix entries");
            }
        }
    }

    /// The second step in parsing WORDCHARS tag. Set slg_wordchar_digits.
    fn parse_wordchars(spell_lang: &mut SpellLang) {
        spell_lang.slg_wordchar_digits = true;
        for c in '0'..='9' {
            if !spell_lang.tag_wordchars.contains(c) {
                spell_lang.slg_wordchar_digits = false;
                break;
            }
        }
        if spell_lang.slg_wordchar_digits {
            spell_lang.slg_wordchars = spell_lang
                .tag_wordchars
                .chars()
                .filter(|c| !c.is_ascii_digit())
                .collect();
        } else {
            spell_lang.slg_wordchars = spell_lang.tag_wordchars.chars().collect();
        }
    }

    /// Parses most of the tags, except SET that is handled by the caller.
    /// Todo implement the remaining tags.
    /// The line parts are without the initial comment and eol.
    /// Comments after the tag, at the end of line, are still present.
    pub fn parse_aff_line(spell_lang: &mut SpellLang, mut parse_state: &mut LineParseState) {
        if parse_state.get_first_token() == "FLAG" {
            if let Some(flag_value) = parse_state.lps_tokens.next() {
                if flag_value == "UTF-8" {
                    spell_lang.slg_flag = FlagFormat::SingleUni;
                } else if flag_value == "long" {
                    spell_lang.slg_flag = FlagFormat::DoubleChar;
                } else if flag_value == "num" {
                    spell_lang.slg_flag = FlagFormat::Numeric;
                } else {
                    parse_state
                        .add_note("Unknown FLAG value, allowed are 'UTF-8', 'long', and 'num'");
                }
            } else {
                parse_state.add_note("No value for FLAG element");
            }
        } else if Parser::parse_bool(spell_lang, &mut parse_state) {
            // nothing more to do
        } else if Parser::parse_string(spell_lang, &mut parse_state) {
            // nothing more to do
        } else if Parser::parse_number(spell_lang, &mut parse_state,) {
            // parsed, nothing more to do
        } else if Parser::parse_string_table(spell_lang, &mut parse_state) {
            // parsed, nothing more to do
        } else if Parser::parse_pair_table(spell_lang, &mut parse_state) {
            // parsed, nothing more to do
        } else if parse_state.get_first_token() == "COMPOUNDRULE" {
            // COMPOUNDRULE 4
            // COMPOUNDRULE 1np
            // COMPOUNDRULE mn*t
            let tokens: Vec<&str> = parse_state.lps_tokens.collect();
            if !spell_lang.slg_compoundrule_parsed {
                let group_size = tokens[0].parse::<u32>();
                if let Ok(group_size) = group_size {
                    _ = spell_lang.slg_compoundrule.try_reserve(group_size as usize);
                }
                spell_lang.slg_compoundrule_parsed = true;
            } else {
                if tokens.len() != 1 {
                    parse_state.add_note("Expected one argument for COMPOUNDRULE");
                }
                let comp_rule_value: &str = tokens[0];
                for comp_rule_flag in
                Parser::parse_compoundrule_flags(&spell_lang, comp_rule_value)
                {
                    spell_lang.slg_flag_hash.insert(
                        comp_rule_flag.clone(),
                        (
                            FlagType::FlagCompRule,
                            spell_lang.slg_compoundrule.len() as u32,
                        ),
                    );
                }
                spell_lang
                    .slg_compoundrule
                    .push(comp_rule_value.to_string());
            }
        } else if Parser::parse_simple_flag(
            spell_lang,
            &[
                ("COMPOUNDFLAG", FlagType::FlagCompound),
                ("COMPOUNDBEGIN", FlagType::FlagCompBegin),
                ("COMPOUNDLAST", FlagType::FlagCompLast),
                ("COMPOUNDMIDDLE", FlagType::FlagCompMid),
                ("COMPOUNDEND", FlagType::FlagCompEnd),
                ("ONLYINCOMPOUND", FlagType::FlagOnlyComp),
                ("COMPOUNDPERMITFLAG", FlagType::FlagCompPermit),
                ("COMPOUNDFORBIDFLAG", FlagType::FlagCompForbid),
                ("COMPOUNDROOT", FlagType::FlagCompRoot),
                ("NEEDAFFIX", FlagType::FlagNeedAffix),
                ("CIRCUMFIX", FlagType::FlagCircumfix),
                ("FORBIDDENWORD", FlagType::FlagForbidden),
                ("SUBSTANDARD", FlagType::FlagSubstandard),
                ("NOSUGGEST", FlagType::FlagNoSuggest),
                ("KEEPCASE", FlagType::FlagKeepCase),
                ("FORCEUCASE", FlagType::FlagForceUcase),
                ("WARN", FlagType::FlagWarn),
                ("LEMMA_PRESENT", FlagType::FlagLemma),
            ],
            &mut parse_state,
        ) {
        } else if parse_state.get_first_token() == "PFX" || parse_state.get_first_token() == "SFX" {
            let is_prefix = parse_state.get_first_token() == "PFX";
            Parser::parse_affix(spell_lang, parse_state, is_prefix);
        } else if parse_state.get_first_token() == "AF" {
            // AF 333
            // AF TbTc # 1
            // AF TbTcff # 2
            // ...
            let tokens: Vec<&str> = parse_state.lps_tokens.collect();
            if !spell_lang.slg_af_parsed {
                let group_size = tokens[0].parse::<u32>();
                if let Ok(group_size) = group_size {
                    _ = spell_lang.slg_af.try_reserve(group_size as usize);
                }
                spell_lang.slg_af_parsed = true;
            } else {
                if tokens.len() >= 1 {
                    let af_index = spell_lang.slg_af.len();
                    spell_lang.slg_af.push(tokens[0].to_string());
                    let af_number_str = spell_lang.slg_af.len().to_string(); // == af_index plus 1
                    spell_lang
                        .slg_flag_hash
                        .insert(af_number_str, (FlagType::FlagAf, af_index as u32));
                    if tokens.len() >= 2 && !tokens[1].starts_with("#") {
                        parse_state.add_note("Superfluous arguments after AF element");
                    }
                } else {
                    parse_state.add_note("Expected one argument for AF");
                }
            }
        } else {
            spell_lang
                .slg_noparse_tags
                .entry(parse_state.get_first_token().to_string())
                .or_insert(0);
            *spell_lang
                .slg_noparse_tags
                .get_mut(parse_state.get_first_token())
                .unwrap() += 1;
        }
    }

    pub fn parse_dic_entry(
        spell_lang: &mut SpellLang,
        dic_entry: &mut DicEntry,
        parse_state: &mut LineParseState,
        reporting_other: bool,
    ) {
        let flagged_words = dic_entry.den_source.split_whitespace();
        // the last slash starts flags, if not preceeded by backslash
        // "vulcanizar/REDA"
        // "virus"
        // "nitidament/ "
        // "buena/B tarde/B"
        // "ESP/Aprilia/BF" // todo report warning
        // "hab/km²/BF"
        // "km\/h"
        for flagged_word_str in flagged_words {
            let slash_pos = flagged_word_str.rfind("/");
            if let Some(slash_pos) = slash_pos {
                // if the previous character is backslash, again no flags are defined
                if slash_pos != 0 {
                    let before_slash = &flagged_word_str[..slash_pos];
                    let last_ch = before_slash.chars().last().unwrap();
                    if last_ch == '\\' {
                        dic_entry
                            .den_words
                            .push(FlaggedWord::new(flagged_word_str, vec![]));
                        // todo correct the word, unescape the slash, -> flagged_word_str
                        // todo and also all the backslashes
                    } else {
                        let from_slash = &flagged_word_str[slash_pos..];
                        let mut chars = from_slash.chars();
                        chars.next();
                        let fwd_flags = chars.as_str();
                        dic_entry.den_words.push(FlaggedWord::new(
                            before_slash,
                            Parser::parse_flags(&spell_lang, &fwd_flags),
                        ));
                    }
                } else {
                    parse_state.add_note("Incorrect slash at the start of word");
                }
            } else {
                dic_entry
                    .den_words
                    .push(FlaggedWord::new(flagged_word_str, vec![]));
            }
        }
        for flagged_word in &dic_entry.den_words {
            for flag in &flagged_word.flw_flags {
                let present = spell_lang.slg_flag_hash.contains_key(flag);
                if !present {
                    if reporting_other {
                        parse_state.add_note2("Unknown flag", flag);
                    }
                    spell_lang
                        .slg_noparse_flags
                        .entry(flag.to_string())
                        .or_insert(0);
                    *spell_lang
                        .slg_noparse_flags
                        .get_mut(&flag.to_string())
                        .unwrap() += 1;
                }
            }
        }
    }

    pub fn parse_dictionary_count(spell_lang: &mut SpellLang, parse_state: &mut LineParseState) {
        // 57157
        let group_size = parse_state.get_first_token().parse::<u32>();
        if let Ok(group_size) = group_size {
            let result = spell_lang.slg_dic_hash.try_reserve(group_size as usize);
            if let Err(_result) = result {
                parse_state.add_note("Not enough memory for dictionary");
                // todo also prevent processing of the next lines
            }
            spell_lang.slg_dic_count = group_size;
        } else {
            parse_state.add_note("Entry count not recognized as number");
        }
        if let Some(_) = parse_state.lps_tokens.next() {
            parse_state.add_note("Unexpected argument after entry count");
        }
    }

    /// The function returns up to 2 notes
    /// .0 is generic error description
    /// .1 is detail, e.g. older definition being duplicated
    /// The line is without the initial comment and eol.
    /// Comments after the words, at the end of line, are still present.
    pub fn parse_dic_line(
        spell_lang: &mut SpellLang,
        parsed_line: &str,
        parse_state: &mut LineParseState,
        reporting_dupl: bool,
        reporting_other: bool,
    ) {
        let mut dic_entry = DicEntry::new(parse_state.lps_line_no, parsed_line.to_string());
        Parser::parse_dic_entry(spell_lang, &mut dic_entry, parse_state, reporting_other);
        if dic_entry.den_words.len() == 0 {
            // empty or comment line
            return;
        }
        let key = dic_entry.hash_key();
        let existing_entry = spell_lang.slg_dic_hash.get_key_value(&key);
        let mut description: Option<String> = None;
        let mut inserting_ok = true;
        if let Some(existing_entry) = existing_entry {
            spell_lang.slg_dic_duplicated += 1;
            let existing_entry = existing_entry.1;
            if existing_entry.den_source.trim() == dic_entry.den_source.trim() {
                description = Some(format!(
                    "{}: Original entry: {}",
                    existing_entry.den_line_no, existing_entry.den_source
                ));
                inserting_ok = false;
            } else {
                /*
                description = Some(format!(
                    "{}: Similar entry: {}",
                    existing_entry.den_line_no, existing_entry.den_source
                ));
                */
                // todo allow several entries of the same key
            }
        }
        if inserting_ok {
            spell_lang.slg_dic_hash.insert(key, dic_entry);
        }
        if let Some(note) = description {
            if reporting_dupl {
                parse_state.add_note2("Duplicate entry", &note);
            }
        }
    }

    pub fn finalize_parsing(spell_lang: &mut SpellLang) -> Vec<String> {
        let mut notes: Vec<String> = vec![];
        // set up slg_flag_hash, map from affix group names (flags) to their indexes
        // also count the affixes
        spell_lang.slg_affix_ct = 0;
        for affix_group in &spell_lang.slg_aff_groups {
            spell_lang.slg_flag_hash.insert(
                affix_group.afc_name.clone(),
                (FlagType::FlagAffix, affix_group.afc_ix),
            );
            spell_lang.slg_affix_ct += affix_group.afc_affixes.len() as u32;
        }
        // set up prev_hash in order to initialize afg_prev_flags, calculated from afe_next_flags
        let mut prev_hash: HashMap<u32, Vec<u32>> = HashMap::new(); // (key=next_ix, value=Vec<prev_ix>)
        for affix_group in spell_lang.slg_aff_groups.iter_mut() {
            let mut flags_defined = false;
            let mut flags_uniform = true; // true when all afg_affixes members have the same afe_next_flags
            let mut next_flags = &vec![];
            for affix_entry in affix_group.afc_affixes.iter_mut() {
                if !flags_defined {
                    next_flags = &affix_entry.afe_next_flags;
                    flags_defined = true;
                    if next_flags.len() != 0 {
                        //notes.push (format!("Groups_{:?}.prev=Group_{}", next_flags, affix_group.afg_name));
                    }
                } else {
                    if next_flags != &affix_entry.afe_next_flags {
                        flags_uniform = false;
                    }
                }
            }
            if !flags_uniform {
                //notes.push (format!("next flags not uniform in group {}", affix_group.afg_name));
                // happens rather often
            }
            let prev_ix = affix_group.afc_ix;
            for next_flag in next_flags {
                if let Some((_flag_type, next_ix)) = spell_lang.slg_flag_hash.get(next_flag) {
                    // next_ix is the index of the "next" affix group
                    if let Some(prev_vec) = prev_hash.get_mut(next_ix) {
                        prev_vec.push(prev_ix);
                    } else {
                        let prev_vec: Vec<u32> = vec![prev_ix];
                        prev_hash.insert(*next_ix, prev_vec);
                    }
                    continue;
                };
                notes.push(format!(
                    "Unknown continuation flag in group {}: {next_flag}",
                    affix_group.afc_name
                ));
            }
        }
        for (next_ix, prev_vec) in prev_hash {
            let affix_group = &mut spell_lang.slg_aff_groups[next_ix as usize];
            affix_group.afc_prev_flags = prev_vec;
            /*
            let prev_names:Vec<String> = prev_vec.into_iter()
                .map(|prev_ix| self.slg_aff_groups[prev_ix as usize].afg_name.clone())
                .collect();
            notes.push (format!("Groups_{}.prev=Group_{:?}",
                affix_group.afg_name, prev_names));
            */
        }
        notes
    }

    pub fn get_summary(spell_lang: &SpellLang) -> String {
        let mut noparse_tags = String::from("");
        let mut first_tag = true;
        for (key, value) in &spell_lang.slg_noparse_tags {
            noparse_tags += if first_tag { ", other tags " } else { "," };
            noparse_tags += key;
            noparse_tags.push('*');
            noparse_tags += &value.to_string();
            first_tag = false;
        }
        let mut noparse_flags = String::from("");
        let mut first_flag = true;
        for (key, value) in &spell_lang.slg_noparse_flags {
            noparse_flags += if first_flag { ", other flags " } else { "," };
            noparse_flags += key;
            noparse_flags.push('*');
            noparse_flags += &value.to_string();
            first_flag = false;
        }
        let duplicated = if spell_lang.slg_dic_duplicated != 0 {
            format!(", {} duplicated entries", spell_lang.slg_dic_duplicated)
        } else {
            String::from("")
        };
        let summary = format!(
            "encoding {}, affixes {}/{}, word entries {}{duplicated}{noparse_tags}{noparse_flags}.",
            spell_lang.slg_set,
            spell_lang.slg_flag_hash.len(),
            spell_lang.slg_affix_ct,
            spell_lang.slg_dic_hash.len(),
        );
        summary
    }
}

pub struct Encoding {}
impl Encoding {
    const UTF_8: &'static str = "UTF-8";
    const ISO_8859_1: &'static str = "ISO8859-1";
    const ISO_8859_2: &'static str = "ISO8859-2";
    const ISO_8859_7: &'static str = "ISO8859-7";
    const ISO_8859_13: &'static str = "ISO8859-13";
    const ISO_8859_15: &'static str = "ISO8859-15";
    const CHAR_SET_NAME: [&'static str; 6] = [
        Self::UTF_8,
        Self::ISO_8859_1,
        Self::ISO_8859_2,
        Self::ISO_8859_7,
        Self::ISO_8859_13,
        Self::ISO_8859_15,
        // all defined for aff files are below, but some haven't been necessary thus far
        //UTF-8, ISO8859-1 - ISO8859-10, ISO8859-13 - ISO8859-15, KOI8-R, KOI8-U, cp1251, ISCII-DEVANAGARI.
    ];

    const ISO_SET_1: [char; 6 * 16] = [
        '\u{a0}', '\u{a1}', '\u{a2}', '\u{a3}', '\u{a4}', '\u{a5}', '\u{a6}', '\u{a7}', '\u{a8}',
        '\u{a9}', '\u{aa}', '\u{ab}', '\u{ac}', '\u{ad}', '\u{ae}', '\u{af}', '\u{b0}', '\u{b1}',
        '\u{b2}', '\u{b3}', '\u{b4}', '\u{b5}', '\u{b6}', '\u{b7}', '\u{b8}', '\u{b9}', '\u{ba}',
        '\u{bb}', '\u{bc}', '\u{bd}', '\u{be}', '\u{bf}', '\u{c0}', '\u{c1}', '\u{c2}', '\u{c3}',
        '\u{c4}', '\u{c5}', '\u{c6}', '\u{c7}', '\u{c8}', '\u{c9}', '\u{ca}', '\u{cb}', '\u{cc}',
        '\u{cd}', '\u{ce}', '\u{cf}', '\u{d0}', '\u{d1}', '\u{d2}', '\u{d3}', '\u{d4}', '\u{d5}',
        '\u{d6}', '\u{d7}', '\u{d8}', '\u{d9}', '\u{da}', '\u{db}', '\u{dc}', '\u{dd}', '\u{de}',
        '\u{df}', '\u{e0}', '\u{e1}', '\u{e2}', '\u{e3}', '\u{e4}', '\u{e5}', '\u{e6}', '\u{e7}',
        '\u{e8}', '\u{e9}', '\u{ea}', '\u{eb}', '\u{ec}', '\u{ed}', '\u{ee}', '\u{ef}', '\u{f0}',
        '\u{f1}', '\u{f2}', '\u{f3}', '\u{f4}', '\u{f5}', '\u{f6}', '\u{f7}', '\u{f8}', '\u{f9}',
        '\u{fa}', '\u{fb}', '\u{fc}', '\u{fd}', '\u{fe}', '\u{ff}',
    ];

    const ISO_SET_2: [char; 6 * 16] = [
        '\u{00a0}', '\u{0104}', '\u{02d8}', '\u{0141}', '\u{00a4}', '\u{013d}', '\u{015a}',
        '\u{00a7}', '\u{00a8}', '\u{0160}', '\u{015e}', '\u{0164}', '\u{0179}', '\u{00ad}',
        '\u{017d}', '\u{017b}', '\u{00b0}', '\u{0105}', '\u{02db}', '\u{0142}', '\u{00b4}',
        '\u{013e}', '\u{015b}', '\u{02c7}', '\u{00b8}', '\u{0161}', '\u{015f}', '\u{0165}',
        '\u{017a}', '\u{02dd}', '\u{017e}', '\u{017c}', '\u{0154}', '\u{00c1}', '\u{00c2}',
        '\u{0102}', '\u{00c4}', '\u{0139}', '\u{0106}', '\u{00c7}', '\u{010c}', '\u{00c9}',
        '\u{0118}', '\u{00cb}', '\u{011a}', '\u{00cd}', '\u{00ce}', '\u{010e}', '\u{0110}',
        '\u{0143}', '\u{0147}', '\u{00d3}', '\u{00d4}', '\u{0150}', '\u{00d6}', '\u{00d7}',
        '\u{0158}', '\u{016e}', '\u{00da}', '\u{0170}', '\u{00dc}', '\u{00dd}', '\u{0162}',
        '\u{00df}', '\u{0155}', '\u{00e1}', '\u{00e2}', '\u{0103}', '\u{00e4}', '\u{013a}',
        '\u{0107}', '\u{00e7}', '\u{010d}', '\u{00e9}', '\u{0119}', '\u{00eb}', '\u{011b}',
        '\u{00ed}', '\u{00ee}', '\u{010f}', '\u{0111}', '\u{0144}', '\u{0148}', '\u{00f3}',
        '\u{00f4}', '\u{0151}', '\u{00f6}', '\u{00f7}', '\u{0159}', '\u{016f}', '\u{00fa}',
        '\u{0171}', '\u{00fc}', '\u{00fd}', '\u{0163}', '\u{02d9}',
    ];

    const ISO_SET_7: [char; 6 * 16] = [
        // three characters not defined
        '\u{00A0}', '\u{2018}', '\u{2019}', '\u{00A3}', '\u{20AC}', '\u{20AF}', '.', '\u{00A6}',
        '\u{00A7}', '\u{00A8}', '\u{00A9}', '\u{037A}', '\u{00AB}', '\u{00AC}', '\u{00AD}',
        '\u{2015}', '\u{00B0}', '\u{00B1}', '\u{00B2}', '\u{00B3}', '\u{0384}', '\u{0385}',
        '\u{0386}', '\u{00B7}', '\u{0388}', '\u{0389}', '\u{038A}', '\u{00BB}', '\u{038C}',
        '\u{00BD}', '\u{038E}', '\u{038F}', '\u{0390}', '\u{0391}', '\u{0392}', '\u{0393}',
        '\u{0394}', '\u{0395}', '\u{0396}', '\u{0397}', '\u{0398}', '\u{0399}', '\u{039A}',
        '\u{039B}', '\u{039C}', '\u{039D}', '\u{039E}', '\u{039F}', '\u{03A0}', '\u{03A1}', '.',
        '\u{03A3}', '\u{03A4}', '\u{03A5}', '\u{03A6}', '\u{03A7}', '\u{03A8}', '\u{03A9}',
        '\u{03AA}', '\u{03AB}', '\u{03AC}', '\u{03AD}', '\u{03AE}', '\u{03AF}', '\u{03B0}',
        '\u{03B1}', '\u{03B2}', '\u{03B3}', '\u{03B4}', '\u{03B5}', '\u{03B6}', '\u{03B7}',
        '\u{03B8}', '\u{03B9}', '\u{03BA}', '\u{03BB}', '\u{03BC}', '\u{03BD}', '\u{03BE}',
        '\u{03BF}', '\u{03C0}', '\u{03C1}', '\u{03C2}', '\u{03C3}', '\u{03C4}', '\u{03C5}',
        '\u{03C6}', '\u{03C7}', '\u{03C8}', '\u{03C9}', '\u{03CA}', '\u{03CB}', '\u{03CC}',
        '\u{03CD}', '\u{03CE}', '.',
    ];

    const ISO_SET_13: [char; 6 * 16] = [
        '\u{00A0}', '\u{201D}', '\u{00A2}', '\u{00A3}', '\u{00A4}', '\u{201E}', '\u{00A6}',
        '\u{00A7}', '\u{00D8}', '\u{00A9}', '\u{0156}', '\u{00AB}', '\u{00AC}', '\u{00AD}',
        '\u{00AE}', '\u{00C6}', '\u{00B0}', '\u{00B1}', '\u{00B2}', '\u{00B3}', '\u{201C}',
        '\u{00B5}', '\u{00B6}', '\u{00B7}', '\u{00F8}', '\u{00B9}', '\u{0157}', '\u{00BB}',
        '\u{00BC}', '\u{00BD}', '\u{00BE}', '\u{00E6}', '\u{0104}', '\u{012E}', '\u{0100}',
        '\u{0106}', '\u{00C4}', '\u{00C5}', '\u{0118}', '\u{0112}', '\u{010C}', '\u{00C9}',
        '\u{0179}', '\u{0116}', '\u{0122}', '\u{0136}', '\u{012A}', '\u{013B}', '\u{0160}',
        '\u{0143}', '\u{0145}', '\u{00D3}', '\u{014C}', '\u{00D5}', '\u{00D6}', '\u{00D7}',
        '\u{0172}', '\u{0141}', '\u{015A}', '\u{016A}', '\u{00DC}', '\u{017B}', '\u{017D}',
        '\u{00DF}', '\u{0105}', '\u{012F}', '\u{0101}', '\u{0107}', '\u{00E4}', '\u{00E5}',
        '\u{0119}', '\u{0113}', '\u{010D}', '\u{00E9}', '\u{017A}', '\u{0117}', '\u{0123}',
        '\u{0137}', '\u{012B}', '\u{013C}', '\u{0161}', '\u{0144}', '\u{0146}', '\u{00F3}',
        '\u{014D}', '\u{00F5}', '\u{00F6}', '\u{00F7}', '\u{0173}', '\u{0142}', '\u{015B}',
        '\u{016B}', '\u{00FC}', '\u{017C}', '\u{017E}', '\u{2019}',
    ];

    const ISO_SET_15: [char; 6 * 16] = [
        '\u{00A0}', '\u{00A1}', '\u{00A2}', '\u{00A3}', '\u{20AC}', '\u{00A5}', '\u{0160}',
        '\u{00A7}', '\u{0161}', '\u{00A9}', '\u{00AA}', '\u{00AB}', '\u{00AC}', '\u{00AD}',
        '\u{00AE}', '\u{00AF}', '\u{00B0}', '\u{00B1}', '\u{00B2}', '\u{00B3}', '\u{017D}',
        '\u{00B5}', '\u{00B6}', '\u{00B7}', '\u{017E}', '\u{00B9}', '\u{00BA}', '\u{00BB}',
        '\u{0152}', '\u{0153}', '\u{0178}', '\u{00BF}', '\u{00C0}', '\u{00C1}', '\u{00C2}',
        '\u{00C3}', '\u{00C4}', '\u{00C5}', '\u{00C6}', '\u{00C7}', '\u{00C8}', '\u{00C9}',
        '\u{00CA}', '\u{00CB}', '\u{00CC}', '\u{00CD}', '\u{00CE}', '\u{00CF}', '\u{00D0}',
        '\u{00D1}', '\u{00D2}', '\u{00D3}', '\u{00D4}', '\u{00D5}', '\u{00D6}', '\u{00D7}',
        '\u{00D8}', '\u{00D9}', '\u{00DA}', '\u{00DB}', '\u{00DC}', '\u{00DD}', '\u{00DE}',
        '\u{00DF}', '\u{00E0}', '\u{00E1}', '\u{00E2}', '\u{00E3}', '\u{00E4}', '\u{00E5}',
        '\u{00E6}', '\u{00E7}', '\u{00E8}', '\u{00E9}', '\u{00EA}', '\u{00EB}', '\u{00EC}',
        '\u{00ED}', '\u{00EE}', '\u{00EF}', '\u{00F0}', '\u{00F1}', '\u{00F2}', '\u{00F3}',
        '\u{00F4}', '\u{00F5}', '\u{00F6}', '\u{00F7}', '\u{00F8}', '\u{00F9}', '\u{00FA}',
        '\u{00FB}', '\u{00FC}', '\u{00FD}', '\u{00FE}', '\u{00FF}',
    ];

    fn bytes_by_table_to_string(
        bytes: &Vec<u8>,
        conversion_table: [char; 96],
    ) -> Result<String, bool> {
        let mut out = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            if *byte < 0x80_u8 {
                out.push(char::from(*byte));
            } else if *byte >= 0xa0 {
                let table_ix: usize = (*byte - 0xa0) as usize;
                out.push(conversion_table[table_ix]); // get the value from the table
            } else {
                out.push('\u{a0}'); // to report a warning?
            }
        }
        return Ok(out);
    }

    fn bytes_to_string(bytes: &Vec<u8>, encoding: &str) -> Result<String, bool> {
        if encoding == Self::UTF_8 {
            if let Ok(line_utf8) = std::str::from_utf8(&bytes) {
                return Ok(String::from(line_utf8));
            }
        }
        if encoding == Self::ISO_8859_1 {
            return Self::bytes_by_table_to_string(bytes, Self::ISO_SET_1);
        }
        if encoding == Self::ISO_8859_2 {
            return Self::bytes_by_table_to_string(bytes, Self::ISO_SET_2);
        }
        if encoding == Self::ISO_8859_7 {
            return Self::bytes_by_table_to_string(bytes, Self::ISO_SET_7);
        }
        if encoding == Self::ISO_8859_13 {
            return Self::bytes_by_table_to_string(bytes, Self::ISO_SET_13);
        }
        if encoding == Self::ISO_8859_15 {
            return Self::bytes_by_table_to_string(bytes, Self::ISO_SET_15);
        }
        Err(false)
    }
}

/// Used during parsing and modification of text language definition.
/// All the languages that are loaded in memory
pub struct TextParser {
    pub tps_check_level: u32,
    /// flag: don't report problems with -, used for performance testing.
    pub tps_skip_output: bool,
    pub tps_showing_details: bool,
    /// Used for compatible processing, to have external test parity.
    /// There will be perhaps more spelling modes in the future.
    pub tps_mode_flags: u32,
    pub tps_langs: Vec<SpellLang>,
    /// maximal number of notes
    pub tps_max_notes: u32,
    pub tps_warn: HashSet<&'static str>,
    pub tps_line_notes: Vec<String>,

    pub tps_parse_status: ParseStatus,
    pub tps_parsed_line: String,
    pub tps_total_notes: usize,
    /// flag: closing brace "}" will revert the ParseMode to Toplevel
    pub tps_mode_until_brace: bool,
    pub tps_passed_count: u32,
    pub tps_failed_count: u32,
    /// test items, each one or more words, expected to pass
    pub tps_test_good_words: Vec<String>,
    /// test items, each one or more words, expected to fail
    pub tps_test_bad_words: Vec<String>,
    /// true if TESTBADGRAM is present
    pub tps_testing_bad_gram: bool,
    /// number of notes at the start of some section
    pub tps_start_note_count: usize,
    /// false if bad-grammar-test failed
    pub tps_test_bad_gram_passed: bool,

}

impl TextParser {
    /// Option --warn value for duplicate words in dictionary
    pub const SHOW_DUPLICATES: &'static str = "dupl";
    /// Option --warn value for other dictionary problems
    pub const SHOW_DIC_OTHER: &'static str = "dic";

    // file or URL extensions
    pub const EXT_NEADIC: &'static str = "neadic";
    pub const EXT_AFF: &'static str = "aff";
    pub const EXT_DIC: &'static str = "dic";
    pub const EXT_GOOD: &'static str = "good";
    pub const EXT_WRONG: &'static str = "wrong";
    
    pub fn new() -> TextParser {
        TextParser {
            tps_check_level: 0,
            tps_skip_output: false,
            tps_showing_details: false,
            tps_mode_flags: 0,
            tps_langs: vec![],
            tps_max_notes: 10,
            tps_warn: HashSet::new(),
            tps_line_notes: vec![],

            tps_parse_status: ParseStatus::FileEnded,
            tps_parsed_line: String::from(""),
            tps_total_notes: 0,
            tps_mode_until_brace: false,
            tps_passed_count: 0,
            tps_failed_count: 0,
            tps_test_good_words: vec![],
            tps_test_bad_words: vec![],
            tps_testing_bad_gram: false,
            tps_start_note_count: 0,
            tps_test_bad_gram_passed: true,
        }
    }

    /// Outputs the text either to a file or the standard output.
    pub fn store_note(&mut self, s: &str) {
        self.tps_line_notes.push (s.to_string())
    }

    fn store_line_note(
        &mut self,
        file_code: &str,
        file_ext: &str,
        line_no: u32,
        line: &str,
        desc: &str,
    ) {
        if self.tps_showing_details {
            let out_text = if line_no != 0 {
                format!("{}.{}:{}: {}: {}", file_code, file_ext, line_no, desc, line)
            } else {
                format!("{}.{}: {}", file_code, file_ext, desc)
            };
            self.store_note(&out_text);
        }
    }

    pub fn store_noline_note(&mut self, file_code: &str, file_ext: &str, desc: &str) {
        self.store_line_note(file_code, file_ext, 0, "", desc);
    }

    fn store_parse_note(
        &mut self,
        file_code: &str,
        file_ext: &str,
        line: &str,
        parse_note: &ParseNote,
    ) {
        self.store_line_note(
            file_code,
            file_ext,
            parse_note.psn_line_no,
            line,
            parse_note.psn_desc,
        );
    }

    /// Reads bytes until the end of line (byte 0x0a, LF)
    /// and converts them to a string (if encoding is ok) and stores the line into "lang".
    fn read_line_bytes(&mut self, spell_lang: &mut SpellLang, line_reader: &mut impl LineReader, line_no: u32) {
        let line_buf_opt = line_reader.read_line();
        if line_buf_opt.is_none() {
            // io error, stop loop
            self.tps_parse_status = ParseStatus::FileEnded;
            self.tps_parsed_line = String::from("");
            return;
        }
        let mut line_buf = line_buf_opt.unwrap();
        if line_buf.len() == 0 {
            // nothing more to read, not even end of line
            self.tps_parse_status = ParseStatus::FileEnded;
            self.tps_parsed_line = String::from("");
            return;
        }
        // truncate UTF-8 BOM in the first line
        if line_no == 1 && line_buf.starts_with(&[0xef_u8, 0xbb_u8, 0xbf_u8]) {
            line_buf.splice(0..3, []);
        }
        // Truncate before initial "#" as comments can be before SET tag, in any encoding.
        // The '#' after tag can be start of comment (eo.aff:807) or not (eo.aff:807),
        // these are processed later.
        // an_ES.aff:187: SFX A Y 311		# FLEXION VERBAL
        // eo.aff:807: SFX # Y 20
        let mut is_non_empty = false;
        for ci in 0..line_buf.len() {
            if line_buf[ci] == 35 {
                // 35 is '#', comment-start character
                line_buf.truncate(ci);
                break; // break "for" after the comment has been removed
            }
            if line_buf[ci] != 32 && line_buf[ci] != 9 {
                // space or tab characters
                is_non_empty = true;
                break; // don't treat '#' as comment if non-space is before it
            }
        }
        // bytes_to_string
        if let Ok(line_as_string) = Encoding::bytes_to_string(&line_buf, &spell_lang.slg_set) {
            let mut line_as_string = line_as_string;
            if line_as_string.ends_with("\r\n") {
                line_as_string.pop();
                line_as_string.pop();
            } else if line_as_string.ends_with("\n") {
                line_as_string.pop();
            };
            self.tps_parse_status = if is_non_empty {
                ParseStatus::LineReady
            } else {
                ParseStatus::EncodingErrorOrEmpty
            };
            self.tps_parsed_line = line_as_string;
        } else {
            self.tps_parse_status = ParseStatus::EncodingErrorOrEmpty;
            self.tps_parsed_line = String::from("");
        }
    }

    fn store_summary_note(
        &mut self,
        extension: &str,
        lang_code: &str,
        bad_encoding: u32,
        note_count: u32,
    ) {
        if bad_encoding != 0 {
            self.store_noline_note(
                &lang_code,
                extension,
                &format!(
                    "Lines with bad character encoding: {}",
                    &bad_encoding.to_string()
                ),
            );
        }
        if note_count != 0 {
            self.store_noline_note(
                &lang_code,
                extension,
                &format!("Parse errors: {}", &note_count.to_string()),
            );
        }
    }

    fn parse_charset(spell_lang: &mut SpellLang, parse_state: &mut LineParseState) {
        // the SET tag
        if let Some(set_value) = parse_state.get_next_token() {
            let mut name_valid = false;
            for set_name in Encoding::CHAR_SET_NAME {
                if set_value == set_name {
                    name_valid = true;
                    spell_lang.slg_set = set_value.to_string();
                    break;
                }
            }
            if !name_valid {
                parse_state
                    .add_note("SET element *limitation*: this encoding is not yet implemented");
            }
        } else {
            parse_state.add_note("No value for SET element");
        }
    }

    fn store_line_notes(
        &mut self,
        file_code: &str,
        file_ext: &str,
        parse_state: &LineParseState,
        line_as_string: &String,
        note_count: &mut u32,
    ) {
        for parse_note in parse_state.get_notes() {
            if *note_count < self.tps_max_notes {
                self.store_parse_note(&file_code, file_ext, &line_as_string, &parse_note);
            } else if *note_count == self.tps_max_notes {
                self.store_noline_note(&file_code, file_ext, "Next parse errors not shown");
            }
            *note_count += 1;
        }
    }

    pub fn finalize_description_part(&mut self, spell_lang: &mut SpellLang, file_ext: &str) {
        let notes = Parser::finalize_parsing(spell_lang);
        if self.tps_showing_details {
            let mut note_count = 0;
            for note in notes {
                if note_count < self.tps_max_notes {
                    self.store_noline_note(&spell_lang.slg_code, file_ext, &note);
                }
                note_count += 1;
            }
            if note_count != 0 {
                self.store_noline_note(
                    &spell_lang.slg_code,
                    file_ext,
                    &format!("Total final notes: {}", &note_count.to_string()),
                );
            }
        }
    }

    pub fn parse_nea_token(parse_lang: &mut TextParser, parse_state: &mut LineParseState) -> ParseMode {
        // NEA DIC {
        // NEA TESTBADGRAM {
        // NEA TESTGOODWORDS {
        // NEA TESTBADWORDS {
        let mut next_mode = ParseMode::Toplevel;
        if let Some(nea2) = parse_state.get_next_token() {
            if nea2 == "DIC" {
                next_mode = ParseMode::WordDic;
            } else if nea2 == "TESTBADGRAM" {
                next_mode = ParseMode::TestBadGram;
                parse_lang.tps_start_note_count = parse_lang.tps_total_notes;
                parse_lang.tps_testing_bad_gram = true;
            } else if nea2 == "TESTGOODWORDS" {
                next_mode = ParseMode::TestGoodWords;
            } else if nea2 == "TESTBADWORDS" {
                next_mode = ParseMode::TestBadWords;
            } else {
                parse_state.add_note("Unknown keyword after NEA tag");
            }
        }
        if next_mode != ParseMode::Toplevel {
            parse_lang.tps_mode_until_brace = true;
            if let Some(nea3) = parse_state.get_next_token() {
                if nea3 != "{" {
                    parse_state.add_note("Expected open brace '{' but found something else");
                }
            } else {
                parse_state.add_note("Expected open brace '{' but found nothing");
            }
        }
        next_mode
    }

    /// The function parses the one file of language definition
    /// in text form and returns a vector of notes (mostly with problems)
    pub fn parse_dictionary_text(
        &mut self,
        spell_lang: &mut SpellLang,
        line_reader: &mut impl LineReader,
    ) {
        let file_ext_str = line_reader.get_extension();
        let file_ext: &str = &file_ext_str;
        let mut parse_mode: ParseMode = match file_ext {
            Self::EXT_AFF=>ParseMode::Toplevel,
            Self::EXT_DIC=>ParseMode::WordDic,
            Self::EXT_GOOD=>ParseMode::TestGoodWords,
            Self::EXT_WRONG=>ParseMode::TestBadWords,
            Self::EXT_NEADIC=>ParseMode::Toplevel,
            &_=>ParseMode::Toplevel,
        };
        self.store_noline_note(
            &spell_lang.slg_code,
            file_ext,
            &format!("Parsing: {}", line_reader.get_full_name()),
        );
        let mut line_no = 0;
        let mut note_count: u32 = 0;
        let bad_encoding: u32 = 0;
        let reporting_dupl = self.tps_warn.contains(Self::SHOW_DUPLICATES);
        let reporting_other = self.tps_warn.contains(Self::SHOW_DIC_OTHER);
        let orig_parse_mode = parse_mode; // for the whole file
        let mut finalized = false;
        loop {
            let parse_mode_before_line = parse_mode;
            line_no += 1;
            self.read_line_bytes(spell_lang, line_reader, line_no);
            let parsed_line = self.tps_parsed_line.clone();
            if self.tps_parse_status == ParseStatus::FileEnded {
                break;
            }
            if self.tps_parse_status == ParseStatus::EncodingErrorOrEmpty {
                continue;
            }
            // the file line is found to be non-empty
            let mut line_tokens = parsed_line.split_whitespace();
            let mut parse_state = LineParseState::new(line_no, &mut line_tokens);
            if parse_state.get_first_token() == "}" && self.tps_mode_until_brace {
                if parse_mode == ParseMode::TestBadGram {
                    if self.tps_start_note_count == self.tps_total_notes {
                        // we expect at least one note to be added while in the bad section
                        // but none has been added, so report an error
                        self.tps_test_bad_gram_passed = false;
                    }
                }
                parse_mode = ParseMode::Toplevel;
                self.tps_mode_until_brace = false;
                // todo check no more tokens
            } else if parse_mode == ParseMode::Toplevel || parse_mode == ParseMode::TestBadGram {
                if parse_state.get_first_token() == "SET" {
                    Self::parse_charset(spell_lang, &mut parse_state);
                }
                if parse_state.get_first_token() == "NEA" {
                    parse_mode = TextParser::parse_nea_token(self, &mut parse_state);
                } else {
                    Parser::parse_aff_line(spell_lang, &mut parse_state);
                }
            } else if orig_parse_mode == ParseMode::WordDic && spell_lang.slg_dic_count == 0 {
                // .dic file, 1st line
                Parser::parse_dictionary_count(spell_lang, &mut parse_state);
            } else if parse_mode == ParseMode::WordDic {
                Parser::parse_dic_line(
                    spell_lang,
                    &self.tps_parsed_line,
                    &mut parse_state,
                    reporting_dupl,
                    reporting_other,
                );
            } else if parse_mode == ParseMode::TestGoodWords {
                self.tps_test_good_words.push(parse_state.get_first_token().to_string());
            } else if parse_mode == ParseMode::TestBadWords {
                self.tps_test_bad_words.push(parse_state.get_first_token().to_string());
            }
            self.store_line_notes(
                &spell_lang.slg_code,
                file_ext,
                &mut parse_state,
                &parsed_line,
                &mut note_count,
            );
            if orig_parse_mode == ParseMode::Toplevel
                && parse_mode_before_line != parse_mode
                && !finalized
            {
                self.finalize_description_part(spell_lang, file_ext);
                finalized = true;
            }
            self.tps_total_notes += parse_state.get_note_length();
        }
        if orig_parse_mode == ParseMode::Toplevel && parse_mode == ParseMode::Toplevel && !finalized
        {
            self.finalize_description_part(spell_lang, file_ext);
        }
        self.store_summary_note(file_ext, &spell_lang.slg_code, bad_encoding, note_count);
    }
}
