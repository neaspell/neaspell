/// UTF-8 engine for spell checking.
//use std::collections::HashMap;
pub use hashbrown::{HashMap,HashSet};

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

/// Parsed value of FLAG tag, and the default value when no FLAG.
#[derive(PartialEq)]
pub enum FlagFormat {
    /// default, one character 
    SingleChar,
    /// "FLAG UTF-8", one unicode character
    SingleUni,
    /// "FLAG long", two characters
    /// compat: charcode up to including 255, but no more.
    /// The two flags are combined into a number, the first character represents
    /// the upper 8 bits. The resulting number must be <= 65509.
    DoubleChar,
    /// "FLAG num", an integer
    /// compat: 1 to 65509.
    Numeric,
}

/// Each word in the dictionary can have one or more flags.
/// Flags can be defined with many elements.
#[derive(Clone)]
pub enum FlagType {
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
pub type FlagNameAndType = (&'static str, FlagType);

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
    pub rgx_def: String,                        // definition string
    pub rgx_vec: Vec<(String, bool)>, // vector of included (.1=true) or excluded (.1=false) characters
    pub rgx_error: Option<(&'static str, u32)>, // description and column number (starting with 1)
}

/*
pub struct WordFlag {
    wdf_name: String, // one-character, two-character, or numeric
    wdf_id: u16, // calculated value of flag, 0 or more, sequential values
    wdf_valid: bool, // true if PFX or SFX tag was seen
}
*/

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

/// Parsed PFX or SFX non-first line.
pub struct AffixEntry {
    // flag stripping affix/flags [condition [morphological_fields...]]"
    pub afe_sub: String, // text to be subtracted from the word form before applying affix
    pub afe_add: String, // text added after subtracting from word form
    pub afe_next_flags: Vec<String>, // this affix can be combined with the next affixes, listed by names
    pub afe_cond: Regex,             // condition to use the affix
    #[allow(dead_code)]
    pub afe_morph: Vec<String>, // additional morphological fields
    #[allow(dead_code)]
    pub afe_ix: u32,
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
}

/// Parsed from the initial line of affix group with data from the next corresponding lines
pub struct AffixClass {
    // flag cross_product number
    pub afc_name: String, // the name of a group, corresponding to den_flags
    pub afc_ix: u32,      // zero or more, index in slg_aff_groups
    pub afc_is_pre: bool, // true for prefix group
    #[allow(dead_code)]
    pub afc_circum: bool, // true if can be part of circumflex,
    pub afc_size: u32,    // member count as given in the aff file
    pub afc_affixes: Vec<AffixEntry>,
    pub afc_prev_flags: Vec<u32>, // the reverse of afe_next_flags
}

impl AffixClass {
    pub fn build_affix_group(
        afg_name: String,
        afg_is_pre: bool,
        afg_circum: bool,
        afg_size: u32,
    ) -> AffixClass {
        AffixClass {
            afc_name: afg_name,
            afc_ix: 0,
            afc_is_pre: afg_is_pre,
            afc_circum: afg_circum,
            afc_size: afg_size,
            afc_affixes: Vec::with_capacity(afg_size as usize),
            afc_prev_flags: vec![],
        }
    }

    pub fn add_entry(&mut self, affix_entry: AffixEntry) {
        self.afc_affixes.push(affix_entry);
    }

    pub fn is_complete(&self) -> bool {
        self.afc_affixes.len() >= self.afc_size as usize
    }
}

/// One word with flags from a dic file
pub struct FlaggedWord {
    #[allow(dead_code)]
    pub flw_char_case: CharCase,
    pub flw_word: String,       // word without the flags
    pub flw_flags: Vec<String>, // flags (if present) or empty string
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

/// One line from a dic file
pub struct DicEntry {
    /// Line number in the dictionary file
    pub den_line_no: u32,
    /// The line in the dictionary file defining the entry
    pub den_source: String,
    pub den_words: Vec<FlaggedWord>,
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

/// A spelling dictionary for a single language. Knows how to spell and how to suggest correct word.
pub struct SpellLang {
    /// Language or test code and possibly state, e.g. "de" or "es_ES" or "sr-Latn" or "affix1"
    /// Parsed from file name, can be updated with LANG element.
    pub slg_code: String,
    // combined ModeFlag values
    pub slg_mode_flags: u32,

    pub slg_set: String,      // SET element: character set for aff and dic files
    pub slg_flag: FlagFormat, // FLAG element: format of affix flags
    pub slg_try: String,
    pub slg_key: String,
    /// The characters from tag_wordchars can be initial or final characters in words or not.
    /// Typically, these are dot (.), hyphen (-), apostrophe (-), digits (0-9) and similar
    /// that are valid in the middle of words. Also used for adding few letters to the end of numbers.
    pub tag_wordchars: String,
    /// True when the tag WORDCHARS include all the ascii digits 0..=9.
    pub slg_wordchar_digits: bool,
    /// characters from tag_wordchars, except ascii digits if all ascii digits were present
    pub slg_wordchars: Vec<char>,
    pub slg_ignore: String,
    pub slg_name: String,
    pub slg_home: String,
    pub slg_version: String,
    pub slg_cplx_pref: bool, // COMPLEXPREFIXES
    pub slg_prefix_max: u8,  // maximal number of prefixes that can be removed, see COMPLEXPREFIXES
    pub slg_suffix_max: u8,  // maximal number of suffixes that can be removed, see COMPLEXPREFIXES
    pub slg_sug_split: bool, // NOSPLITSUGS sets it to false
    pub slg_sug_dots: bool,  // SUGSWITHDOTS sets it to true
    pub slg_rep: Vec<(String, String)>,
    pub slg_phone: Vec<(String, String)>,
    pub slg_iconv: Vec<(String, String)>,
    pub slg_oconv: Vec<(String, String)>,
    pub slg_map: (Vec<String>, bool),   // (array_itself, parsed)
    pub slg_break: (Vec<String>, bool), // (array_itself, parsed)
    pub slg_af_parsed: bool,
    pub slg_af: Vec<String>,
    pub slg_compoundrule_parsed: bool,
    pub slg_compoundrule: Vec<String>,
    pub slg_comp_check_dup: bool,
    pub slg_comp_check_rep: bool,
    pub slg_comp_check_case: bool,
    pub slg_check_sharp_s: bool,
    pub slg_check_comp_triple: bool,
    pub slg_simplified_triple: bool,
    pub slg_only_max_diff: bool,
    pub slg_full_string: bool,
    pub slg_comp_more_suffixes: bool,
    pub slg_comp_min: u32,
    pub slg_comp_word_max: u32,
    pub slg_max_cpd_sugs: u32,
    pub slg_max_ngram_sugs: u32,
    pub slg_max_diff: u32,
    pub slg_aff_groups: Vec<AffixClass>, // storage for affixes
    pub slg_pfxes: Vec<u32>,             // indexes of prefixes in slg_aff_groups
    pub slg_sfxes: Vec<u32>,             // indexes of suffixes in slg_aff_groups
    pub slg_flag_hash: HashMap<String, (FlagType, u32)>, // (afg_name, type, afg_ix)
    pub slg_affix_ct: u32,
    pub slg_dic_count: u32,
    pub slg_dic_hash: HashMap<String, DicEntry>,
    pub slg_dic_duplicated: u32, // number of duplicated entries
    pub slg_noparse_tags: HashMap<String, u32>, // tags not set parsed
    pub slg_noparse_flags: HashMap<String, u32>, // flags in dictionary not known
}

impl SpellLang {
    pub fn new(slg_code: &str) -> SpellLang {
        SpellLang {
            slg_code: String::from(slg_code),
            slg_mode_flags: 0,
            slg_set: String::from("UTF-8"),
            slg_flag: FlagFormat::SingleUni,
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
            slg_flag_hash: HashMap::new(),
            slg_affix_ct: 0,
            slg_dic_count: 0,
            slg_dic_hash: HashMap::new(),
            slg_dic_duplicated: 0,
            slg_noparse_tags: HashMap::new(),
            // temporarily tracking the tags that are not yet implemented
            // also can be used for ordering between tags
            slg_noparse_flags: HashMap::new(),
        }
    }
}

#[derive(PartialEq, Copy, Clone)]
pub enum TokenType {
    NotWord,
    IsWord,
    IsGoodWord, // spelling-check passed
    IsBadWord, // spelling-check failed
}

/// Functions for spelling words and suggesting corrections.
pub struct Spell {}

impl Spell {
    /// The function returns true if the word is present in the dictionary
    /// and (optionally) if it has the required flag.
    /// todo: process multi-word entries
    fn word_present(
        spell_lang: &SpellLang,
        char_case: CharCase,
        word: &str,
        flag: Option<&String>,
    ) -> bool {
        let dict_entry = spell_lang.slg_dic_hash.get(word);
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
                if (spell_lang.slg_mode_flags as u32 & ModeFlag::TestCompat as u32) != 0
                    && char_case == CharCase::Lower
                {
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

    /// The function returns true if the word is correctly spelled in spell_lang
    /// and (for languages with uppercase and lowercase letters)
    /// has the character case as in the dictionary.
    /// Thus far, some amount of prefixes (prefix_ct) or suffixes 8suffix_ct) has already been removed from the original word.
    /// For the second affix of the same place, only affix groups in ix_subset are allowed.
    fn check_decased_word(
        spell_lang: &SpellLang,
        mut char_case: CharCase,
        word: &str,
        ix_subset: Option<&Vec<u32>>,
        prefix_ct: u8, // so many prefixes has been processed
        suffix_ct: u8, // so many prefixes has been processed
    ) -> bool {
        if Spell::word_present(spell_lang, char_case, word, None) && ix_subset == None {
            return true;
        }
        let mut base_word = String::with_capacity(128); // not to allocate it often, it's defined here
                                                        // after removing affix from a word with other casing, the casing of the new word can be different
        let originally_other_case = char_case == CharCase::Other;
        for affix_group in &spell_lang.slg_aff_groups {
            let new_prefix_ct = if affix_group.afc_is_pre {
                prefix_ct + 1
            } else {
                prefix_ct
            };
            let new_suffix_ct = if affix_group.afc_is_pre {
                suffix_ct
            } else {
                suffix_ct + 1
            };
            // new_prefix_ct and new_suffix_ct are the counts after applying any affix_entry from affix_group
            if new_prefix_ct > spell_lang.slg_prefix_max
                || new_suffix_ct > spell_lang.slg_suffix_max
            {
                continue; // this would be too many levels for prefixes or suffixes
            }
            if new_prefix_ct == 2 || new_suffix_ct == 2 {
                // when applying the second affix of the same place, only some affixes are allowed
                if let Some(subset) = ix_subset {
                    if !subset.contains(&affix_group.afc_ix) {
                        continue; // skip such affix group, not in a vector of required indexes
                    }
                }
            }
            for affix_entry in &affix_group.afc_affixes {
                if !Spell::is_substring_at_edge(word, &affix_entry.afe_add, affix_group.afc_is_pre)
                {
                    continue;
                }
                // from word to base_word: -add, +sub
                base_word.clear();
                if affix_group.afc_is_pre {
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
                    .match_edge(&base_word, affix_group.afc_is_pre)
                {
                    continue;
                }
                if Spell::word_present(
                    spell_lang,
                    char_case,
                    &base_word,
                    Some(&affix_group.afc_name),
                ) {
                    return true;
                }
                if Spell::check_decased_word(
                    spell_lang,
                    char_case,
                    &base_word,
                    Some(&affix_group.afc_prev_flags),
                    new_prefix_ct,
                    new_suffix_ct,
                ) {
                    return true;
                }
            }
        }
        // lng_mode_flags
        false
    }

    /// Returns true if the (non-alphabetic) character can be either in a word or not.
    /// There are two spaces in example 'It's five o'clock.' so three token are produced.
    /// In the first token ('It's), the first apostrophe is not part of word,
    /// the second one is part of word.
    fn is_non_alphabetic_in_word(spell_lang: &SpellLang, c: char) -> bool {
        spell_lang.slg_wordchar_digits && c.is_ascii_digit()
            || spell_lang.slg_wordchars.contains(&c)
    }

    // Returns true if the character can be in a word.
    fn in_word_or_optional(spell_lang: &SpellLang, c: char) -> bool {
        c.is_alphabetic() || Spell::is_non_alphabetic_in_word(spell_lang, c)
    }

    pub fn check_token(spell_lang: &SpellLang, word: &str) -> bool {
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
        let mut result =
            Spell::check_decased_word(&spell_lang, char_case, &normalized_word, None, 0, 0);
        if !result {
            // let's trim the characters that are optionally in the word
            let trimmed_word =
                &normalized_word.trim_matches(|c| Spell::is_non_alphabetic_in_word(spell_lang, c));
            result = Spell::check_decased_word(&spell_lang, char_case, trimmed_word, None, 0, 0);
        }
        //     fn is_non_alphabetic_in_word(&self, c:char) -> bool {

        result
    }

    /// Changes `untokenized_text` into a vector of tuples
    /// Vec<(a_string_of_charactes: String, token_type: TokenType)>
    fn tokenize(spell_lang: &SpellLang, untokenized_text: &str) -> Vec<(String, TokenType)> {
        let parts = 
            untokenized_text.match_indices(|c: char| !Spell::in_word_or_optional(spell_lang, c));
        let mut token_vec = Vec::<(String, TokenType)>::new();
        let mut last_ix: usize = 0; // end of last pushed non-word
        for part in parts {
            let (start_ix, word) = part;
            if last_ix < start_ix {
                token_vec.push ((untokenized_text[last_ix..start_ix].to_string(), TokenType::IsWord));
            }
            token_vec.push ((word.to_string(), TokenType::NotWord));
            last_ix = start_ix + word.len();
        }
        if last_ix < untokenized_text.len() {
            token_vec.push ((untokenized_text[last_ix..].to_string(), TokenType::IsWord));
        }
        token_vec
    }

    /// Check several words or paragraph, not yet tokenized.
    pub fn check_text<'a>(
        spell_lang: &SpellLang,
        untokenized_text: &'a str,
    ) -> Vec<(String, TokenType)> {
        let mut tokens: Vec<(String, TokenType)> = Spell::tokenize(spell_lang, &untokenized_text);
        for token in &mut tokens {
            let (word, token_type) = token;
            if word.len() == 0 || *token_type != TokenType::IsWord {
                continue;
            }
            let check_result = Spell::check_token(&spell_lang, &word);
            // todo depending on spl_check_level, let the function return more info
            *token_type = if check_result {TokenType::IsGoodWord} else {TokenType::IsBadWord};
        }
        tokens
    }
}

#[cfg(test)]
mod tests {
    use crate::core_speller::Regex;

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
