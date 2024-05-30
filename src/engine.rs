/// UFT-8 engine for spell checking.
use std::{collections::HashMap, str::SplitWhitespace};

type SpellHashMap<K, V> = HashMap<K, V>;
// todo implement another hash function, not requiring random (as in webassembly)
// keys are always Strings

pub enum ModeFlag {
    /// compatible processing, to have external test parity
    /// right now, 
    TestCompat = 1,
    //LowercasePreInternet = 1, or LowercaseInternet, www.england.uk, @unesco, perhaps with tokenizer, too
    // after other flags are defined, the option --compat will select TestCompat
    // that will include several flags

    // there will be more spelling modes in the future
    // parse programming identifiers: ParseHtml, parseHtml, parse_html
}

/// Parsed value of FLAG tag
#[derive(PartialEq)]
enum FlagFormat {
    SingleChar, // "UTF-8"
    DoubleChar, // "long"
    Numeric,    // "num"
}

#[derive(Clone)]
enum FlagType {
    FlagAffix,
    FlagAf,
    FlagCompRule,
    FlagCompound,
    FlagCompBegin,
    FlagCompLast,
    FlagCompMid,
    FlagCompEnd,
    FlagOnlyComp,
    FlagCompPermit,
    FlagCompForbid,
    FlagCompRoot,
    FlagNeedAffix,
    FlagCircumfix,
    FlagForbidden,
    FlagSubstandard,
    FlagNoSuggest,
    FlagKeepCase,
    FlagForceUcase,
    FlagWarn,
    FlagLemma,
}

/// pairs: tag name and associated flag type
type FlagNameAndType = (&'static str, FlagType);

/// Comment on a single line or a problem.
pub struct ParseNote {
    pub psn_line_no: u32, // 0 no data; when given > 0
    pub psn_desc: &'static str,
    pub psn_details: Option<String>, // displayed on a separate line, after description's line
}

#[derive(PartialEq,Clone,Copy)]
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
    pln_tokens: SplitWhitespace<'a>
}

impl<'a> ParsedLine<'a> {
    #[allow(dead_code)]
    pub fn new(pln_line:&'a str) -> ParsedLine<'a> {
        ParsedLine::<'a> {
            pln_line,
            pln_tokens: pln_line.split_whitespace()
        }
    }
}


/// While parsing a single dictionary line.
pub struct ParseState<'a> {
    /// line number in the file, starting with 1
    pst_line_no: u32,
    /// remaining tokens in the line
    pst_tokens: &'a mut SplitWhitespace<'a>,
    /// the first token in the line is often used as keyword
    pst_first_token: Option<&'a str>,
    /// warnings and explanations of error handling
    pst_notes: Vec<ParseNote>,
}

impl<'a> ParseState<'a> {
    pub fn new(pst_line_no: u32, pst_tokens: &'a mut SplitWhitespace<'a> ) -> ParseState<'a> {
        ParseState::<'a> {
            pst_line_no,
            pst_tokens,
            pst_first_token: None,
            pst_notes: vec![],
        }
    }

    pub fn add_note(&mut self, desc: &'static str) {
        self.pst_notes.push(ParseNote {
            psn_line_no: self.pst_line_no,
            psn_desc: desc,
            psn_details: None,
        })
    }

    pub fn add_note2(&mut self, desc: &'static str, detail: &String) {
        self.pst_notes.push(ParseNote {
            psn_line_no: self.pst_line_no,
            psn_desc: desc,
            psn_details: Some(detail.clone()),
        })
    }

    pub fn get_next_token(&mut self) -> Option<&str> {
        self.pst_tokens.next()
    }

    /// The function is expected to be called when the token is known to be present.
    /// It returns the token
    pub fn get_first_token(&mut self) -> &str {
        if let None = self.pst_first_token {
            self.pst_first_token = self.pst_tokens.next();
            if let None = self.pst_first_token {
                self.pst_first_token = Some("");
            }
        }
        if let Some(token) = self.pst_first_token {
            return token;
        }
        "" // not reachable, but compiler doesn't know it and needs something
    }

    pub fn get_notes(&self) -> &Vec<ParseNote> {
        &self.pst_notes
    }
}

/// Whan language script has lowercase and uppercase characters,
/// dictionary normalizes uppercase and initial-uppercase words to lowercase.
/// Here are all the casing possiblilites.
#[derive(PartialEq, Copy, Clone)]
pub enum CharCase {
    Lower,   // all characters are lowercase
    Initial, // the first character is uppercase, the remaining are lowercase
    Upper,   // all characters are uppercase
    Other,   // a mixture of uppercase and lowercase characters other than in CharCase::Initial
}

impl CharCase {
    /// Returns the word case and the string to use as dictionary key.
    /// With both tuple members, the original string can be restored.
    pub fn normalize_case(word: &str) -> (CharCase, String) {
        // web, Hague, UNICEF, 's-Gravenhage, 中国
        let mut first_lower_or_none = true;
        let mut next_lower_or_none = true; //
        let mut is_first = true;
        let mut any_lower = false;
        let mut any_upper = false;
        for ch in word.chars() {
            // ?todo minimize the unicode table to only the known characters,
            // all other can be cosidered caseless
            let is_lower = ch.is_lowercase();
            let is_upper = ch.is_uppercase();
            // at most one of is_lower and is_upper can be true
            if is_lower {
                // lowercase character with correponding uppercase character
                any_lower = true;
            } else if is_upper {
                // uppercase character with correponding lowercase character
                any_upper = true;
                if is_first {
                    first_lower_or_none = false;
                } else {
                    next_lower_or_none = false;
                }
            } else {
                // nothing to do, caseless character
            }
            is_first = false;
        }
        if next_lower_or_none {
            // all lowercase except possibly the first
            if first_lower_or_none {
                return (CharCase::Lower, String::from(word));
            } else {
                // first was upper
                return (CharCase::Initial, String::from(word.to_lowercase()));
            }
        } else {
            if any_lower && any_upper {
                return (CharCase::Other, String::from(word));
            }
            return (CharCase::Upper, String::from(word.to_lowercase()));
        }
    }

    #[allow(dead_code)]
    fn restore_case(char_case: CharCase, word: &str) -> String {
        // web, Hague, UNICEF, 's-Gravenhage, 中国
        if char_case == CharCase::Lower {
            return word.to_string();
        } else if char_case == CharCase::Initial {
            let mut c = word.chars();
            let result = match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            };
            return result;
        } else if char_case == CharCase::Upper {
            return word.to_uppercase();
        } else {
            return word.to_string();
        }
    }
}

const CLEAN_REGEX_PAIRS: [(&'static str, &'static str); 3] = [
    // workarounds until better implemented
    ("(^", ")"), // uk_UA.aff:1503: SFX R есь сього (^весь)
    //("(", ")"), // uk_UA.aff:1503: SFX R есь сього (^весь)
    (".+", ""), // af_ZA
    ("^", ""),  // af_ZA
]; // to remove from

/// Simple regular expression, with the brackets "[]"
/// used for defining character sets and the caron "^"
/// after the opening bracket complementing the set.
/// The dot "." means any character.
/// The other regex punctuation {}*+?() is not allowed.
pub struct Regex {
    rgx_def: String,                        // definition string
    rgx_vec: Vec<(String, bool)>, // vector of included (.1=true) or excluded (.1=false) characters
    rgx_error: Option<(&'static str, u32)>, // description and column number (starting with 1)
}

/*
pub struct WordFlag {
    wdf_name: String, // one-character, two-character, or numeric
    wdf_id: u16, // calculated value of flag, 0 or more, sequential values
    wdf_valid: bool, // true if PFX or SFX tag was seen
}
*/

/// Parsed PFX or SFX non-first line.
pub struct AffixEntry {
    // flag stripping affix/flags [condition [morphological_fields...]]"
    afe_sub: String, // text to be subtracted from the word form before applying affix
    afe_add: String, // text added after subtracting from word form
    afe_next_flags: Vec<String>, // this affix can be combined with the next affixes, listed by names
    afe_cond: Regex,             // condition to use the affix
    #[allow(dead_code)]
    afe_morph: Vec<String>, // additional morphological fields
    #[allow(dead_code)]
    afe_ix: u32,
}

/// Parsed from the initial line of affix group with data from the next corresponding lines
/// It corresponds to a flag that is given to a word entry.
pub struct AffixGroup {
    // flag cross_product number
    afg_name: String, // the name of a group, corresponding to den_flags
    afg_ix: u32,      // zero or more, index in slg_aff_groups
    afg_is_pre: bool, // true for prefix group
    #[allow(dead_code)]
    afg_circum: bool, // true if can be part of circumflex,
    afg_size: u32,    // member count as given in the aff file
    afg_affixes: Vec<AffixEntry>,
    afg_prev_flags: Vec<u32>, // the reverse of afe_next_flags
}

/// One word with flags from a dic file
pub struct FlaggedWord {
    #[allow(dead_code)]
    flw_char_case: CharCase,
    flw_word: String,       // word without the flags
    flw_flags: Vec<String>, // flags (if present) or empty string
}

/// One line from a dic file
pub struct DicEntry {
    /// Line number in the dictionary file
    den_line_no: u32,
    /// The line in the dictionary file defining the entry
    den_source: String,
    den_words: Vec<FlaggedWord>,
}

impl Regex {
    pub fn new(rgx_def: String) -> Regex {
        // rgx_vec[i].1 is true if the characters
        // in rgx_vec[i].0 are accepted (included)
        let mut rgx_vec: Vec<(String, bool)> = vec![];
        let mut in_brackets = false;
        let mut is_included = true; // the
        let mut rgx_error: Option<(&'static str, u32)> = None;
        let mut bracket_chars = "".to_string();
        let mut pos: u32 = 0;
        let mut rgx_clean: &str = &rgx_def;
        for (clean_pre, clean_post) in CLEAN_REGEX_PAIRS {
            if rgx_def.starts_with(clean_pre) && rgx_def.ends_with(clean_post) {
                rgx_clean = &rgx_def[clean_pre.len()..rgx_def.len() - clean_post.len()];
                // todo Warning ("A compatible regex prefix was removed");
                break;
            }
        }
        for c in rgx_clean.chars() {
            pos += 1;
            if c == '[' {
                if in_brackets {
                    rgx_error = Some(("Open brackets ([) inside brackets in regex", pos));
                }
                in_brackets = true;
                is_included = true;
            } else if c == '.' {
                if !in_brackets {
                    rgx_vec.push((String::from(""), false));
                } else {
                    rgx_error = Some(("Dot (.) inside brackets in regex", pos));
                }
            } else if c == ']' {
                if !in_brackets {
                    rgx_error = Some(("Close brackets (]) not within brackets in regex", pos));
                }
                rgx_vec.push((bracket_chars, is_included));
                in_brackets = false;
                bracket_chars = "".to_string();
            } else if c == '^' {
                if in_brackets && is_included {
                    is_included = false;
                } else {
                    rgx_error = Some(("Unexpected caron (^) in regex", pos));
                }
            } else {
                if in_brackets {
                    bracket_chars.push(c);
                } else {
                    if "{}*+?()".contains(c) {
                        rgx_error = Some(("Unexpected character in regex", pos));
                    } else {
                        rgx_vec.push((String::from(c), true));
                    }
                }
            }
        }
        Regex {
            rgx_def,
            rgx_vec,
            rgx_error,
        }
    }

    /// The function returns true if the regular expression matches String s
    /// at the edge, either from the start (is_prefix==true)
    /// or from the end (is_prefix==false).
    pub fn match_edge(&self, s: &str, is_prefix: bool) -> bool {
        if let Some(_) = self.rgx_error {
            return false;
        }
        if self.rgx_vec.len() > s.len() {
            return false;
        }
        if is_prefix {
            let r = &self.rgx_vec;
            for it in r.iter().zip(s.chars()) {
                let (ri, si) = it;
                if ri.0.contains(si) != ri.1 {
                    return false;
                }
            }
        } else {
            let r = &self.rgx_vec;
            for it in r.iter().rev().zip(s.chars().rev()) {
                let (ri, si) = it;
                if ri.0.contains(si) != ri.1 {
                    return false;
                }
            }
        }
        true
    }
}

impl std::fmt::Display for Regex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.rgx_def)
    }
}

impl AffixEntry {
    pub fn new(
        afe_sub: String,
        afe_add: String,
        afe_next_flags: Vec<String>,
        afe_cond: String,
    ) -> AffixEntry {
        AffixEntry {
            afe_sub,
            afe_add,
            afe_next_flags,
            afe_cond: Regex::new(afe_cond),
            afe_morph: vec![],
            afe_ix: 0,
        }
    }

    /*
    pub fn to_string(&self) -> String {
        format!("{}{}{}", self.afe_sub, self.afe_add, self.afe_cond)
    }
    */
}

impl AffixGroup {
    pub fn build_affix_group(
        afg_name: String,
        afg_is_pre: bool,
        afg_circum: bool,
        afg_size: u32,
    ) -> AffixGroup {
        AffixGroup {
            afg_name,
            afg_ix: 0,
            afg_is_pre,
            afg_circum,
            afg_size,
            afg_affixes: Vec::with_capacity(afg_size as usize),
            afg_prev_flags: vec![],
        }
    }

    pub fn add_entry(&mut self, affix_entry: AffixEntry) {
        self.afg_affixes.push(affix_entry);
    }

    pub fn is_complete(&self) -> bool {
        self.afg_affixes.len() >= self.afg_size as usize
    }
}

impl FlaggedWord {
    pub fn new(word: &str, flw_flags: Vec<String>) -> FlaggedWord {
        let (flw_char_case, flw_word) = CharCase::normalize_case(word);
        FlaggedWord {
            flw_char_case,
            flw_word,
            flw_flags,
        }
    }
}

impl DicEntry {
    pub fn new(den_line_no: u32, den_source: String) -> DicEntry {
        DicEntry {
            den_line_no,
            den_source,
            den_words: vec![],
        }
    }

    /// The key for the HashMap.
    /// If there are multiple words, they're using separator for joining them.
    pub fn hash_key(&self) -> String {
        let mut key = String::from("");
        for flagged_word in &self.den_words {
            if key.len() == 0 {
                key += &flagged_word.flw_word;
            } else {
                key += " ";
                key += &flagged_word.flw_word;
            }
        }
        key
    }
}

// A spelling dictionary for a single language
pub struct Lang {
    /// Language code and possibly state, e.g. "de" or "es_ES" or "sr-Latn"
    /// Parsed from file name, can be updated with LANG element.
    pub slg_code: String,
    // combined ModeFlag values
    pub lng_mode_flags: u32,

    pub lng_parse_status: ParseStatus,
    pub lng_parsed_line: String,
    /// flag: closing brace "}" will revert the ParseMode to Toplevel
    pub lng_mode_until_brace: bool,
    pub lng_passed_count: u32,
    pub lng_failed_count: u32,
    /// test items, each one or more words, expected to pass
    pub lng_pass_expected: Vec<String>,
    /// test items, each one or more words, expected to fail
    pub lng_fail_expected: Vec<String>,

    pub slg_set: String,  // SET element: character set, thus far only "UTF-8"
    slg_flag: FlagFormat, // FLAG element: format of affix flags
    slg_try: String,
    slg_key: String,
    /// The characters from tag_wordchars can be initial or final characters in words or not.
    /// Typically, these are dot (.), hyphen (-), apostrophe (-), digits (0-9) and similar
    /// that are valid in the middle of words. Also used for adding few letters to the end of numbers.
    tag_wordchars: String,
    /// True when the tag WORDCHARS include all the ascii digits 0..=9.
    slg_wordchar_digits: bool,
    /// characters from tag_wordchars, except ascii digits if all ascii digits were present
    slg_wordchars: Vec<char>,
    slg_ignore: String,
    slg_name: String,
    slg_home: String,
    slg_version: String,
    slg_cplx_pref: bool, // COMPLEXPREFIXES
    slg_prefix_max: u8, // maximal number of prefixes that can be removed, see COMPLEXPREFIXES
    slg_suffix_max: u8, // maximal number of suffixes that can be removed, see COMPLEXPREFIXES
    slg_sug_split: bool, // NOSPLITSUGS sets it to false
    slg_sug_dots: bool,  // SUGSWITHDOTS sets it to true
    slg_rep: Vec<(String, String)>,
    slg_phone: Vec<(String, String)>,
    slg_iconv: Vec<(String, String)>,
    slg_oconv: Vec<(String, String)>,
    slg_map: (Vec<String>, bool),   // (array_itself, parsed)
    slg_break: (Vec<String>, bool), // (array_itself, parsed)
    slg_af_parsed: bool,
    slg_af: Vec<String>,
    slg_compoundrule_parsed: bool,
    slg_compoundrule: Vec<String>,
    slg_comp_check_dup: bool,
    slg_comp_check_rep: bool,
    slg_comp_check_case: bool,
    slg_check_sharp_s: bool,
    slg_check_comp_triple: bool,
    slg_simplified_triple: bool,
    slg_only_max_diff: bool,
    slg_full_string: bool,
    slg_comp_more_suffixes: bool,
    slg_comp_min: u32,
    slg_comp_word_max: u32,
    slg_max_cpd_sugs: u32,
    slg_max_ngram_sugs: u32,
    slg_max_diff: u32,
    slg_aff_groups: Vec<AffixGroup>, // storage for affixes
    slg_pfxes: Vec<u32>,             // indexes of prefixes in slg_aff_groups
    slg_sfxes: Vec<u32>,             // indexes of suffixes in slg_aff_groups
    slg_flag_hash: SpellHashMap<String, (FlagType, u32)>, // (afg_name, type, afg_ix)
    slg_affix_ct: u32,
    pub slg_dic_count: u32,
    slg_dic_hash: SpellHashMap<String, DicEntry>,
    slg_dic_duplicated: u32, // number of duplicated entries
    slg_noparse_tags: SpellHashMap<String, u32>, // tags not set parsed
    slg_noparse_flags: SpellHashMap<String, u32>, // flags in dictionary not known
}

impl Lang {
    pub fn new(slg_code: &str) -> Lang {
        Lang {
            slg_code: String::from(slg_code),
            lng_mode_flags: 0,
            lng_parse_status: ParseStatus::FileEnded,
            lng_parsed_line: String::from(""),
            lng_mode_until_brace: false,
            lng_passed_count: 0,
            lng_failed_count: 0,
            lng_pass_expected: vec![],
            lng_fail_expected: vec![],        

            slg_set: String::from("UTF-8"),
            slg_flag: FlagFormat::SingleChar,
            slg_try: String::from(""),
            slg_key: String::from(""),
            tag_wordchars: String::from(""),
            slg_wordchar_digits: false,
            slg_wordchars: vec![],
            slg_ignore: String::from(""),
            slg_name: String::from(""),
            slg_home: String::from(""),
            slg_version: String::from(""),
            slg_cplx_pref: false,
            slg_prefix_max: 1,
            slg_suffix_max: 2,
            slg_sug_split: true,
            slg_sug_dots: false,
            slg_rep: vec![],
            slg_phone: vec![],
            slg_map: (vec![], false),
            slg_break: (vec![], false),
            slg_iconv: vec![],
            slg_oconv: vec![],
            slg_af_parsed: false,
            slg_af: vec![],
            slg_compoundrule_parsed: false,
            slg_compoundrule: vec![],
            slg_comp_check_dup: false,
            slg_comp_check_rep: false,
            slg_comp_check_case: false,
            slg_check_sharp_s: false,
            slg_check_comp_triple: false,
            slg_simplified_triple: false,
            slg_only_max_diff: false,
            slg_full_string: false,
            slg_comp_more_suffixes: false,
            slg_comp_min: 0,
            slg_comp_word_max: 0,
            slg_max_cpd_sugs: 0,
            slg_max_ngram_sugs: 0,
            slg_max_diff: 5,
            slg_pfxes: vec![],
            slg_sfxes: vec![],
            slg_aff_groups: vec![],
            slg_flag_hash: SpellHashMap::new(),
            slg_affix_ct: 0,
            slg_dic_count: 0,
            slg_dic_hash: SpellHashMap::new(),
            slg_dic_duplicated: 0,
            slg_noparse_tags: SpellHashMap::new(),
            // temporarily tracking the tags that are not yet implemented
            // also can be used for ordering between tags
            slg_noparse_flags: SpellHashMap::new(),
        }
    }

    /// Parses string with multiple flags.
    /// With FLAG UTF-8, each flag is one character, multiple flags are not separated.
    /// With FLAG long, each flag is two characters, multiple flags are not separated
    /// With FLAG num, each flag is an unsigned number, multiple flags are separated by commas
    fn parse_flags(&self, flags: &str) -> Vec<String> {
        if flags.len() == 0 {
            return vec![];
        }
        if self.slg_flag == FlagFormat::SingleChar {
            // one-character flags
            return flags.chars().map(|c| c.to_string()).collect();
        }
        if self.slg_flag == FlagFormat::DoubleChar {
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
        if self.slg_flag == FlagFormat::Numeric {
            return flags.split(",").map(|s| s.to_string()).collect();
        }
        vec![]
    }

    /// Parses COMPOUNDRULE string with multiple flags.
    /// Asterisk, question mark and parenthesis are regex characters.
    /// SingleChar flags are all the remaining characters: mn*t,
    /// DoubleChar and Numeric flags are enclosed in parentheses.
    /// Returns the vector of flags.
    fn parse_compoundrule_flags(&self, flags: &str) -> Vec<String> {
        if self.slg_flag == FlagFormat::SingleChar {
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
    fn parse_simple(
        tag: &str,
        variab: &mut bool,
        value: bool,
        parse_state: &mut ParseState,
    ) -> bool {
        // SUGSWITHDOTS
        if parse_state.get_first_token() == tag {
            let tokens: Vec<&str> = parse_state.pst_tokens.collect();
            if tokens.len() > 0 && !tokens[0].starts_with("#") {
                parse_state.add_note("Unexpected argument");
            }
            *variab = value;
            return true;
        }
        false
    }

    /// Parses the tag and its string value.
    /// If no errors, it updates "variab".
    /// The "parse_state.note" is set to Some if a message is to be issued.
    /// It returns true if the tag was procesed.
    fn parse_string(tag: &str, variab: &mut String, parse_state: &mut ParseState) -> bool {
        if parse_state.get_first_token() == tag {
            if let Some(try_value) = parse_state.pst_tokens.next() {
                *variab = try_value.to_string();
            } else {
                parse_state.add_note("Missing value");
            }
            return true;
        }
        false
    }

    /// Parses the tag and its unsigned numeric value.
    /// If no errors, it updates "variab".
    /// The "parse_state.note" is set to Some if a message is to be issued.
    /// It returns true if the tag was procesed.
    fn parse_number(tag: &str, variab: &mut u32, parse_state: &mut ParseState) -> bool {
        if parse_state.get_first_token() == tag {
            if let Some(number_value) = parse_state.pst_tokens.next() {
                let number_value = number_value.parse::<u32>();
                if let Ok(number_value) = number_value {
                    *variab = number_value;
                } else {
                    parse_state.add_note("Expected number");
                }
            } else {
                parse_state.add_note("Missing value");
            }
            return true;
        }
        false
    }

    /// Parses the tag with an array of String values.
    /// If no errors, it updates "select_value".
    /// The "note" is set to Some if a message is to be issued.
    /// It returns true if the tag was procesed.
    fn parse_string_table(
        tag: &str,
        variab: &mut (Vec<String>, bool), // .0 table itself, .1 flag: has been parsed
        parse_state: &mut ParseState,
    ) -> bool {
        if parse_state.get_first_token() == tag {
            // MAP 5 ## a simple table
            // MAP aáAÁ
            let tokens: Vec<&str> = parse_state.pst_tokens.collect();
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
            return true;
        }
        false
    }

    /// Parses the tag with an array of (String,String) values.
    /// If no errors, it updates "select_value".
    /// The "note" is set to Some if a message is to be issued.
    /// It returns true if the tag was procesed.
    fn parse_pair_table(
        tag: &str,
        variab: &mut Vec<(String, String)>,
        parse_state: &mut ParseState,
    ) -> bool {
        if parse_state.get_first_token() == tag {
            // REP 20 # replacement table
            // REP ke que
            let tokens: Vec<&str> = parse_state.pst_tokens.collect();
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
            return true;
        }
        false
    }

    fn parse_simple_flag(
        &mut self,
        simple_flag_table: &[FlagNameAndType],
        parse_state: &mut ParseState,
    ) -> bool {
        let simple_ix = simple_flag_table
            .iter()
            .position(|sf| parse_state.get_first_token() == sf.0);
        if let Some(simple_ix) = simple_ix {
            // a name of simple COMPOUND* (COMPOUND_FLAG etc) and similar tag
            let flag_type = &simple_flag_table[simple_ix].1;
            if let Some(comp_flag) = parse_state.pst_tokens.next() {
                self.slg_flag_hash
                    .insert(String::from(comp_flag), (flag_type.clone(), 0));
            } else {
                parse_state.add_note("No flag value for element");
            }
            return true;
        }
        false
    }

    fn parse_affix(&mut self, parse_state: &mut ParseState, is_prefix: bool) {
        let tokens: Vec<&str> = parse_state.pst_tokens.collect();
        if tokens.len() < 3 {
            // any PFX or SFX element, initial or not, should have at least three
            // arguments after the tag name
            parse_state.add_note("Less than 3 tokens for PFX or SFX");
            return;
        }
        let mut is_first =
            self.slg_aff_groups.len() == 0 || self.slg_aff_groups.last().unwrap().is_complete();
        if !is_first
            && self.slg_aff_groups.len() != 0
            && self.slg_aff_groups.last().unwrap().afg_name != tokens[0]
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
                    AffixGroup::build_affix_group(group_name, is_prefix, can_circum, group_size);
                affix_group.afg_ix = self.slg_aff_groups.len() as u32;
                if is_prefix {
                    self.slg_pfxes.push(affix_group.afg_ix);
                } else {
                    self.slg_sfxes.push(affix_group.afg_ix);
                }
                self.slg_aff_groups.push(affix_group);
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
                self.parse_flags(&next),
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
            let aff_groups: &mut Vec<AffixGroup> = &mut self.slg_aff_groups;
            let last_aff_group: &mut AffixGroup = aff_groups.last_mut().unwrap();
            affix_entry.afe_ix = last_aff_group.afg_affixes.len() as u32;
            if last_aff_group.afg_affixes.len() < last_aff_group.afg_size as usize {
                last_aff_group.add_entry(affix_entry);
            } else {
                parse_state.add_note("too many affix entries");
            }
        }
    }

    /// The second step in parsing WORDCHARS tag. Set slg_wordchar_digits.
    fn parse_wordchars(&mut self) {
        self.slg_wordchar_digits = true;
        for c in '0'..='9' {
            if !self.tag_wordchars.contains(c) {
                self.slg_wordchar_digits = false;
                break;
            }
        }
        if self.slg_wordchar_digits {
            self.slg_wordchars = self
                .tag_wordchars
                .chars()
                .filter(|c| !c.is_ascii_digit())
                .collect();
        } else {
            self.slg_wordchars = self.tag_wordchars.chars().collect();
        }
    }

    /// Parses most of the tags, except SET that is handled by the caller.
    /// Todo implement the remaining tags.
    /// The line parts are without the initial comment and eol.
    /// Comments after the tag, at the end of line, are still present.
    ///
    /// In the code, notes can still be returned. This is to be changed to modifying parse_state.note
    pub fn parse_aff_line(&mut self, mut parse_state: &mut ParseState) {
        if parse_state.get_first_token() == "FLAG" {
            if let Some(flag_value) = parse_state.pst_tokens.next() {
                if flag_value == "UTF-8" {
                    self.slg_flag = FlagFormat::SingleChar;
                } else if flag_value == "long" {
                    self.slg_flag = FlagFormat::DoubleChar;
                } else if flag_value == "num" {
                    self.slg_flag = FlagFormat::Numeric;
                } else {
                    parse_state
                        .add_note("Unknown FLAG value, allowed are 'UTF-8', 'long', and 'num'");
                }
            } else {
                parse_state.add_note("No value for FLAG element");
            }
        } else if Self::parse_string("TRY", &mut self.slg_try, &mut parse_state) {
            // parsed, nothing more to do
        } else if Self::parse_string("LANG", &mut self.slg_code, &mut parse_state) {
            // parsed, nothing more to do
        } else if Self::parse_string("KEY", &mut self.slg_key, &mut parse_state) {
            // parsed, nothing more to do
        } else if Self::parse_string("WORDCHARS", &mut self.tag_wordchars, &mut parse_state) {
            self.parse_wordchars();
        } else if Self::parse_string("IGNORE", &mut self.slg_ignore, &mut parse_state) {
            // parsed, nothing more to do
        } else if Self::parse_string("NAME", &mut self.slg_name, &mut parse_state) {
            // parsed, nothing more to do; not documented; Dizionario italiano
        } else if Self::parse_string("HOME", &mut self.slg_home, &mut parse_state) {
            // parsed, nothing more to do; not documented; https://libreitalia.org
        } else if Self::parse_string("VERSION", &mut self.slg_version, &mut parse_state) {
            // parsed, nothing more to do; not documented; 5.1.0 (13/10/2020)
        } else if Self::parse_simple(
            "COMPLEXPREFIXES",
            &mut self.slg_cplx_pref,
            false,
            &mut parse_state,
        ) {
            self.slg_prefix_max = 2;
            self.slg_suffix_max = 1;
        } else if Self::parse_simple(
            "NOSPLITSUGS",
            &mut self.slg_sug_split,
            false,
            &mut parse_state,
        ) {
            // parsed, nothing more to do
        } else if Self::parse_simple(
            "SUGSWITHDOTS",
            &mut self.slg_sug_dots,
            true,
            &mut parse_state,
        ) {
            // parsed, nothing more to do
        } else if Self::parse_simple(
            "CHECKCOMPOUNDDUP",
            &mut self.slg_comp_check_dup,
            true,
            &mut parse_state,
        ) {
            // parsed, nothing more to do
        } else if Self::parse_simple(
            "CHECKCOMPOUNDREP",
            &mut self.slg_comp_check_rep,
            true,
            &mut parse_state,
        ) {
            // parsed, nothing more to do
        } else if Self::parse_simple(
            "CHECKCOMPOUNDCASE",
            &mut self.slg_comp_check_case,
            true,
            &mut parse_state,
        ) {
            // parsed, nothing more to do
        } else if Self::parse_simple(
            "CHECKSHARPS",
            &mut self.slg_check_sharp_s,
            true,
            &mut parse_state,
        ) {
            // parsed, nothing more to do
        } else if Self::parse_simple(
            "CHECKCOMPOUNDTRIPLE",
            &mut self.slg_check_comp_triple,
            true,
            &mut parse_state,
        ) {
            // parsed, nothing more to do
        } else if Self::parse_simple(
            "SIMPLIFIEDTRIPLE",
            &mut self.slg_simplified_triple,
            true,
            &mut parse_state,
        ) {
            // parsed, nothing more to do
        } else if Self::parse_simple(
            "ONLYMAXDIFF",
            &mut self.slg_only_max_diff,
            true,
            &mut parse_state,
        ) {
            // parsed, nothing more to do
        } else if Self::parse_simple(
            "FULLSTRIP",
            &mut self.slg_full_string,
            true,
            &mut parse_state,
        ) {
            // parsed, nothing more to do
        } else if Self::parse_simple(
            "COMPOUNDMORESUFFIXES",
            &mut self.slg_comp_more_suffixes,
            true,
            &mut parse_state,
        ) {
            // parsed, nothing more to do
        } else if Self::parse_number("COMPOUNDMIN", &mut self.slg_comp_min, &mut parse_state) {
            // parsed, nothing more to do
        } else if Self::parse_number(
            "COMPOUNDWORDMAX",
            &mut self.slg_comp_word_max,
            &mut parse_state,
        ) {
            // parsed, nothing more to do
        } else if Self::parse_number("MAXCPDSUGS", &mut self.slg_max_cpd_sugs, &mut parse_state) {
            // parsed, nothing more to do
        } else if Self::parse_number(
            "MAXNGRAMSUGS",
            &mut self.slg_max_ngram_sugs,
            &mut parse_state,
        ) {
            // parsed, nothing more to do
        } else if Self::parse_number("MAXDIFF", &mut self.slg_max_diff, &mut parse_state) {
            // parsed, nothing more to do
        } else if Self::parse_pair_table("REP", &mut self.slg_rep, &mut parse_state) {
            // parsed, nothing more to do
        } else if Self::parse_pair_table("PHONE", &mut self.slg_phone, &mut parse_state) {
            // parsed, nothing more to do
        } else if Self::parse_pair_table("ICONV", &mut self.slg_iconv, &mut parse_state) {
            // parsed, nothing more to do
        } else if Self::parse_pair_table("OCONV", &mut self.slg_oconv, &mut parse_state) {
            // parsed, nothing more to do
        } else if Self::parse_string_table("MAP", &mut self.slg_map, &mut parse_state) {
            // parsed, nothing more to do
        } else if Self::parse_string_table("BREAK", &mut self.slg_break, &mut parse_state) {
            // parsed, nothing more to do
        } else if parse_state.get_first_token() == "COMPOUNDRULE" {
            // COMPOUNDRULE 4
            // COMPOUNDRULE 1np
            // COMPOUNDRULE mn*t
            let tokens: Vec<&str> = parse_state.pst_tokens.collect();
            if !self.slg_compoundrule_parsed {
                let group_size = tokens[0].parse::<u32>();
                if let Ok(group_size) = group_size {
                    _ = self.slg_compoundrule.try_reserve(group_size as usize);
                }
                self.slg_compoundrule_parsed = true;
            } else {
                if tokens.len() != 1 {
                    parse_state.add_note("Expected one argument for COMPOUNDRULE");
                }
                let comp_rule_value: &str = tokens[0];
                for comp_rule_flag in &self.parse_compoundrule_flags(comp_rule_value) {
                    self.slg_flag_hash.insert(
                        comp_rule_flag.clone(),
                        (FlagType::FlagCompRule, self.slg_compoundrule.len() as u32),
                    );
                }
                self.slg_compoundrule.push(comp_rule_value.to_string());
            }
        } else if self.parse_simple_flag(
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
            self.parse_affix(parse_state, is_prefix);
        } else if parse_state.get_first_token() == "AF" {
            // AF 333
            // AF TbTc # 1
            // AF TbTcff # 2
            // ...
            let tokens: Vec<&str> = parse_state.pst_tokens.collect();
            if !self.slg_af_parsed {
                let group_size = tokens[0].parse::<u32>();
                if let Ok(group_size) = group_size {
                    _ = self.slg_af.try_reserve(group_size as usize);
                }
                self.slg_af_parsed = true;
            } else {
                if tokens.len() >= 1 {
                    let af_index = self.slg_af.len();
                    self.slg_af.push(tokens[0].to_string());
                    let af_number_str = self.slg_af.len().to_string(); // == af_index plus 1
                    self.slg_flag_hash
                        .insert(af_number_str, (FlagType::FlagAf, af_index as u32));
                    if tokens.len() >= 2 && !tokens[1].starts_with("#") {
                        parse_state.add_note("Superfluous arguments after AF element");
                    }
                } else {
                    parse_state.add_note("Expected one argument for AF");
                }
            }
        } else {
            self.slg_noparse_tags
                .entry(parse_state.get_first_token().to_string())
                .or_insert(0);
            *self.slg_noparse_tags.get_mut(parse_state.get_first_token()).unwrap() += 1;
        }
    }

    pub fn parse_dic_entry(
        &mut self,
        dic_entry: &mut DicEntry,
        parse_state: &mut ParseState,
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
                        dic_entry
                            .den_words
                            .push(FlaggedWord::new(before_slash, self.parse_flags(&fwd_flags)));
                    }
                } else {
                    parse_state.add_note ("Incorrect slash at the start of word");
                }
            } else {
                dic_entry
                    .den_words
                    .push(FlaggedWord::new(flagged_word_str, vec![]));
            }
        }
        for flagged_word in &dic_entry.den_words {
            for flag in &flagged_word.flw_flags {
                let present = self.slg_flag_hash.contains_key(flag);
                if !present {
                    if reporting_other {
                        parse_state.add_note2 ("Unknown flag",  flag);
                    }
                    self.slg_noparse_flags.entry(flag.to_string()).or_insert(0);
                    *self.slg_noparse_flags.get_mut(&flag.to_string()).unwrap() += 1;
                }
            }
        }
    }

    pub fn parse_dictionary_count (&mut self, parse_state: &mut ParseState) {
        // 57157
        let group_size = parse_state.get_first_token().parse::<u32>();
        if let Ok(group_size) = group_size {
            let result = self.slg_dic_hash.try_reserve(group_size as usize);
            if let Err(_result) = result {
                parse_state.add_note("Not enough memory for dictionary");
                // todo also prevent processing of the next lines
            }
            self.slg_dic_count = group_size;
        } else {
            parse_state.add_note("Entry count not recognized as number");
        }
        if let Some(_) = parse_state.pst_tokens.next() {
            parse_state.add_note("Unexpected argument after entry count");
        }
    }

    /// The function returns up to 2 notes
    /// .0 is generic error description
    /// .1 is detail, e.g. older definition being duplicated
    /// The line is without the initial comment and eol.
    /// Comments after the words, at the end of line, are still present.
    pub fn parse_dic_line(
        &mut self,
        parse_state: &mut ParseState,
        reporting_dupl: bool,
        reporting_other: bool,
    ) {
        let mut dic_entry = DicEntry::new(parse_state.pst_line_no, self.lng_parsed_line.clone());
        self.parse_dic_entry(&mut dic_entry, parse_state, reporting_other);
        if dic_entry.den_words.len() == 0 {
            // empty or comment line
            return;
        }
        let key = dic_entry.hash_key();
        let existing_entry = self.slg_dic_hash.get_key_value(&key);
        let mut description: Option<String> = None;
        let mut inserting_ok = true;
        if let Some(existing_entry) = existing_entry {
            self.slg_dic_duplicated += 1;
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
            self.slg_dic_hash.insert(key, dic_entry);
        }
        if let Some(note) = description {
            if reporting_dupl {
                parse_state.add_note2("Duplicate entry", &note);
            }
        }
    }

    pub fn finalize_parsing(&mut self) -> Vec<String> {
        let mut notes: Vec<String> = vec![];
        // set up slg_flag_hash, map from affix group names (flags) to their indexes
        // also count the affixes
        self.slg_affix_ct = 0;
        for affix_group in &self.slg_aff_groups {
            self.slg_flag_hash.insert(
                affix_group.afg_name.clone(),
                (FlagType::FlagAffix, affix_group.afg_ix),
            );
            self.slg_affix_ct += affix_group.afg_affixes.len() as u32;
        }
        // set up prev_hash in order to initialize afg_prev_flags, calculated from afe_next_flags
        let mut prev_hash: HashMap<u32, Vec<u32>> = HashMap::new(); // (key=next_ix, value=Vec<prev_ix>)
        for affix_group in self.slg_aff_groups.iter_mut() {
            let mut flags_defined = false;
            let mut flags_uniform = true; // true when all afg_affixes members have the same afe_next_flags
            let mut next_flags = &vec![];
            for affix_entry in affix_group.afg_affixes.iter_mut() {
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
            let prev_ix = affix_group.afg_ix;
            for next_flag in next_flags {
                if let Some((_flag_type, next_ix)) = self.slg_flag_hash.get(next_flag) {
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
                    "Unknown next flags in group {}: {next_flag}",
                    affix_group.afg_name
                ));
            }
        }
        for (next_ix, prev_vec) in prev_hash {
            let affix_group = &mut self.slg_aff_groups[next_ix as usize];
            affix_group.afg_prev_flags = prev_vec;
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

    pub fn get_summary(&self) -> String {
        let mut noparse_tags = String::from("");
        let mut first_tag = true;
        for (key, value) in &self.slg_noparse_tags {
            noparse_tags += if first_tag { ", other tags " } else { "," };
            noparse_tags += key;
            noparse_tags.push('*');
            noparse_tags += &value.to_string();
            first_tag = false;
        }
        let mut noparse_flags = String::from("");
        let mut first_flag = true;
        for (key, value) in &self.slg_noparse_flags {
            noparse_flags += if first_flag { ", other flags " } else { "," };
            noparse_flags += key;
            noparse_flags.push('*');
            noparse_flags += &value.to_string();
            first_flag = false;
        }
        let duplicated = if self.slg_dic_duplicated != 0 {
            format!(", {} duplicated entries", self.slg_dic_duplicated)
        } else {
            String::from("")
        };
        let summary = format!(
            "encoding {}, affixes {}/{}, word entries {}{duplicated}{noparse_tags}{noparse_flags}.",
            self.slg_set,
            self.slg_flag_hash.len(),
            self.slg_affix_ct,
            self.slg_dic_hash.len(),
        );
        summary
    }

    /// The function returns true is the word is present in the dictionary
    /// and (optionally) if it has the required flag.
    /// todo: process multi-word entries
    fn word_present(&self, char_case: CharCase, word: &str, flag: Option<&String>) -> bool {
        let dict_entry = self.slg_dic_hash.get(word);
        if let Some(dict_entry) = dict_entry {
            let dict_case = dict_entry.den_words[0].flw_char_case;
            if dict_case == CharCase::Upper {
                if char_case == CharCase::Initial {
                    // the uppercase abbreviations (in dictionary) are not allowed with initial case (in text)
                    // todo define Modeflag value to allow in identifiers in programming languages like ParseHtml
                    return false;
                }
            }
            if dict_case == CharCase::Upper || dict_case == CharCase::Initial {
                if (self.lng_mode_flags as u32 & ModeFlag::TestCompat as u32) != 0  && char_case == CharCase::Lower {
                    //mail addresses and other internet identificators are lowercase
                    // such lowercase is not allowed in ModeFlag::TestCompat
                    return false;
                }
            }
            if let Some(flag) = flag {
                return dict_entry.den_words[0].flw_flags.contains(&flag);
            }
            return true; // no flags to check
        }
        false // word not in dictionary
    }

    /// Returns true if 'substring' is at the start or at the end of 'word',
    /// depending on 'is_prefix'.
    fn is_substring_at_edge(word: &str, substring: &str, is_prefix: bool) -> bool {
        if is_prefix {
            word.starts_with(substring)
        } else {
            word.ends_with(substring)
        }
    }

    /// The function returns true if the word is correctly spelled in Lang "self"
    /// and (for languages with uppercase and lowercase letters)
    /// has the character case as in the dictionary.
    /// Thus far, some amount of prefixes (prefix_ct) or suffixes 8suffix_ct) has already been removed from the original word.
    /// For the second affix of the same place, only affix groups in ix_subset are allowed.
    pub fn check_decased_word(
        &self,
        mut char_case: CharCase,
        word: &str,
        ix_subset: Option<&Vec<u32>>,
        prefix_ct:u8, // so many prefixes has been processed
        suffix_ct: u8, // so many prefixes has been processed
    ) -> bool {
        if self.word_present(char_case, word, None) {
            return true;
        }
        let mut base_word = String::with_capacity(128); // not to allocate it often, it's defined here
        // after removing affix from a word with other casing, the casing of the new word can be different
        let originally_other_case = char_case == CharCase::Other;
        for affix_group in &self.slg_aff_groups {
            let new_prefix_ct = if affix_group.afg_is_pre {prefix_ct+1} else {prefix_ct};
            let new_suffix_ct = if affix_group.afg_is_pre {suffix_ct} else {suffix_ct+1};
            // new_prefix_ct and new_suffix_ct are the counts after applying any affix_entry from affix_group
            if new_prefix_ct > self.slg_prefix_max || new_suffix_ct > self.slg_suffix_max {
                continue; // this would be too many levels for prefixes or suffixes
            }
            if new_prefix_ct == 2 || new_suffix_ct == 2 {
                // when applying the second affix of the same place, only some affixes are allowed
                if let Some(subset) = ix_subset {
                    if !subset.contains(&affix_group.afg_ix) {
                        continue; // skip such affix group, not in a vector of required indexes
                    }
                }
            }
            for affix_entry in &affix_group.afg_affixes {
                if !Lang::is_substring_at_edge(word, &affix_entry.afe_add, affix_group.afg_is_pre) {
                    continue;
                }
                // from word to base_word: -add, +sub
                base_word.clear();
                if affix_group.afg_is_pre {
                    base_word += &affix_entry.afe_sub;
                    base_word += &word[affix_entry.afe_add.len()..];
                } else {
                    base_word += &word[..word.len() - affix_entry.afe_add.len()];
                    base_word += &affix_entry.afe_sub;
                }
                if originally_other_case {
                    (char_case, base_word) = CharCase::normalize_case(&base_word);
                }
                // now check the base_word
                if !affix_entry
                    .afe_cond
                    .match_edge(&base_word, affix_group.afg_is_pre)
                {
                    continue;
                }
                if self.word_present(char_case, &base_word, Some(&affix_group.afg_name)) {
                    return true;
                }
                if self.check_decased_word(
                    char_case,
                    &base_word,
                    Some(&affix_group.afg_prev_flags),
                    new_prefix_ct, new_suffix_ct,
                ) {
                    return true;
                }
            }
        }
        // lng_mode_flags
        false
    }

    pub fn check_token(&self, word: &str) -> bool {
        if word.len() == 0 {
            return true;
        }
        /*
        - Dictionary forms of the words can be uppercased in general text:
        test, Test TEST
        London london LONDON
        HTML
        - Letters within the word can be uppercased:
        's-Gravenhage 'S-GRAVENHAGE
        TikTok TIKTOK
        - Dictionary forms of the words can be lowercased in internet addresses:
        UNICEF (@unicef), unicef.org
        - Full sentences with mixed character case
        I visited the UNICEF web page unicef.org.
        The official name of the Hague is 's-Gravenhage.
        TikTok is well known.

        */
        let (char_case, normalized_word) = CharCase::normalize_case(word);
        let mut result = self.check_decased_word(char_case, &normalized_word, None, 0, 0, );
        if !result {
            // let's trim the characters that are optionally in the word
            let trimmed_word = &normalized_word.trim_matches(|c| self.is_non_alphabetic_in_word(c));
            result = self.check_decased_word(char_case, trimmed_word, None, 0, 0, );
        }
        //     fn is_non_alphabetic_in_word(&self, c:char) -> bool {

        result
    }

    /// Returns true if the (non-alphabetic) character can be either in a word or not.
    /// There are two spaces in example 'It's five o'clock.' so three token are produced.
    /// In the first token ('It's), the first apostrophe is not part of word,
    /// the second one is part of word.
    fn is_non_alphabetic_in_word(&self, c: char) -> bool {
        self.slg_wordchar_digits && c.is_ascii_digit() || self.slg_wordchars.contains(&c)
    }

    // Returns true if the character can be in a word.
    fn in_word_or_optional(&self, c: char) -> bool {
        c.is_alphabetic() || self.is_non_alphabetic_in_word(c)
    }

    pub fn tokenize(&self, line: &str) -> Vec<String> {
        let parts = line.split(|c: char| !self.in_word_or_optional(c));
        parts.map(|s| s.to_string()).collect()
    }

    /// Check several words or paragraph, not yet tokenized.
    pub fn check_untokenized<'a>(&self, untokenized: &'a str) -> Vec<(String, bool)> {
        let words: Vec<String> = self.tokenize(&untokenized);
        let mut checked_words: Vec<(String, bool)> = vec![];
        for word in words {
            if word.len() == 0 {
                continue;
            }
            let check_result = self.check_token(&word);
            // todo depending on spl_check_level, let the function return more info
            checked_words.push((word, check_result))
            //println!("Word {}: {}", String::from(result_s), word);
        }
        checked_words
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::Regex;

    #[test]
    fn regex_test() {
        let regex1 = Regex::new(String::from("[ai]to"));
        let regex2 = Regex::new(String::from("ato"));
        assert_eq!(regex1.match_edge("regato", false), true);
        assert_eq!(regex1.match_edge("regoto", false), false);
        assert_eq!(regex1.match_edge("regar", false), false);
        assert_eq!(regex1.match_edge("to", false), false);
        assert_eq!(regex2.match_edge("regato", false), true);
        assert_eq!(regex2.match_edge("regat", false), false);
        assert_eq!(regex2.match_edge("regito", false), false);
    }
}
