// The lib module processes dictionary files, character sets (encodings),
// environment variables, and command-line options.
// It normalizes the slashes in file names.
// The option names and the variable names are defined here.

use neaspell_core::core_speller;
use neaspell_core::core_speller::SpellLang;
use neaspell_core::core_speller::TokenType;
use neaspell_core::text_parser;
use neaspell_core::text_parser::LineReader;
use core_speller::ModeFlag;
use core_speller::Spell;
use neaspell_core::text_parser::Parser;
use std::env;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::stdout;
use std::io::BufWriter;
use std::io::{self, prelude::*, BufReader};
use std::path::{MAIN_SEPARATOR, MAIN_SEPARATOR_STR};
use std::str;
use text_parser::TextParser;

pub const PROGRAM_VERSION: &str = "0.1.5";

pub struct ArgTokens {
    pub args: Vec<String>,           // command-line arguments
    pub agt_current_ix: usize,       // index of the next argument to take
    pub agt_last_ix: usize,          // index of the last non-value argument
    pub agt_option_processed: usize, // positive when a value for an option was taken
}

/// CLI arguments to the process.
impl ArgTokens {
    pub fn new() -> ArgTokens {
        ArgTokens {
            args: vec![],
            agt_current_ix: 0,
            agt_last_ix: 0,
            agt_option_processed: 0,
        }
    }

    /// Initialize with arguments, [0] is process name,[1] is the first actual argument, etc
    pub fn set_arguments(&mut self, args: Vec<String>) {
        self.args = args;
        self.agt_current_ix = 1; // the process name will be skipped
    }

    pub fn get_next_arg(&mut self) -> Option<String> {
        self.agt_current_ix += self.agt_option_processed;
        self.agt_option_processed = 0;
        if self.agt_current_ix < self.args.len() {
            let result = self.args[self.agt_current_ix].clone();
            self.agt_last_ix = self.agt_current_ix;
            self.agt_current_ix += 1;
            return Some(result);
        }
        None
    }

    fn get_arg_option(&mut self) -> Option<String> {
        let next_ix = self.agt_current_ix + self.agt_option_processed;
        if next_ix < self.args.len() {
            self.agt_option_processed += 1;
            return Some(self.args[next_ix].clone());
        }
        println!(
            "Missing value for argument: {}",
            self.args[self.agt_current_ix]
        );
        None
    }
}


struct StdLineReader {
    pub slr_base_name: String,
    pub slr_extension: String,
    slr_reader: Option<BufReader<File>>
}

impl StdLineReader {
    pub fn new(slr_base_name: &str, slr_extension:&str) -> StdLineReader {
        let full_file_name = slr_base_name.to_string() + "." + &slr_extension;
        let file_result = File::open(full_file_name.clone());
        if let Ok(file) = file_result {
            return StdLineReader {
                slr_base_name: slr_base_name.to_string(),
                slr_extension: slr_extension.to_string(),
                slr_reader: Some(BufReader::new(file))};
        }
        return StdLineReader {
            slr_base_name: slr_base_name.to_string(),
            slr_extension: slr_extension.to_string(),
            slr_reader:None
        }
    }
}

impl LineReader for StdLineReader {
    fn get_base_name(&self) -> String {
        self.slr_base_name.clone()
    }
    fn get_extension(&self) -> String {
        self.slr_extension.clone()
    }
    fn read_line(&mut self) -> Option<Vec::<u8>> {
        let mut line_buf: Vec::<u8> = vec![];
        if let Some(buf_reader) = &mut self.slr_reader {
            let result = buf_reader.read_until(10, &mut line_buf);
            if let Ok(_) = result {
                return Some(line_buf);
            }    
        };
        None
    }
}

pub struct CliSpeller {
    csr_arg_tokens: ArgTokens,
    csr_dict_codes: String, // comma-separated dictionary codes, possibly with asterisk wildcards, or
    // paths (with separators) to the dictionary files, without the file extension
    csr_test_codes: Vec<String>, // names or test files, possibly with asterisk wildcards
    csr_test_words: String, // comma-separated test word, to filter-out the other words
    csr_text_files: Vec<String>,
    csr_options_finished: bool, // true after "--" argument

    // the second group of variables fields imply usage of files and environment variables
    /// search directories for the dictionaries
    pub spl_dic_paths: Vec<String>,
    /// if true, slash (/) or backslash (\) in file names must be
    /// according to the OS; by default false, both are interchangeable and are normalized
    pub spl_strict_slash: bool,
    /// search directories for the tests
    pub spl_test_paths: Vec<String>,
    pub spl_out_file_name: Option<String>,
    pub spl_out_writer: Option<Box<dyn Write>>,
}

impl CliSpeller {
    // the file extensions

    pub fn new() -> CliSpeller {
        CliSpeller {
            csr_arg_tokens: ArgTokens::new(),
            csr_dict_codes: String::new(),
            csr_test_codes: vec![],
            csr_test_words: String::new(),
            csr_text_files: vec![],
            csr_options_finished: false,

            spl_dic_paths: vec![],
            spl_strict_slash: false,
            spl_test_paths: vec![],
            spl_out_file_name: None,
            spl_out_writer: None,
        }
    }

    /// Returns true if the environment variable exists.
    pub fn process_path_environment_variable(var_name: &str, var_vec: &mut Vec<String>) -> bool {
        if let Some(paths) = env::var_os(var_name) {
            for dic_path in env::split_paths(&paths) {
                let path_wildcarded = dic_path.into_os_string().into_string().unwrap();
                let entry_vec = Self::list_wildcarded(&path_wildcarded);
                for entry in entry_vec {
                    var_vec.push(entry);
                }
            }
            return true;
        }
        false
    }

    const NEA_DICPATH: &'static str = "NEA_DICPATH";
    const COMMON_DICPATH: &'static str = "DICPATH";
    const NEA_TESTPATH: &'static str = "NEA_TESTPATH";
    /// Process environment variable, e.g.
    /// ```
    /// $Env:NEA_DICPATH=".;C:\0prog\spelling\dictionaries\*"
    /// export NEA_DICPATH='.:/mnt/c/0prog/spelling/dictionaries/*'
    /// $Env:NEA_TESTPATH=".;C:\0prog\spelling\tests"
    /// ```
    /// The separators are OS specific(Linux: slash and colon; Windows: backslash and semicolon).
    /// There can be one asterisk (after the last path separator only) in path_wildcarded and it means "any".
    pub fn process_environment_variables(&mut self) {
        // The first of the two variables is used if defined: NEA_DICPATH and DICPATH
        let _ = CliSpeller::process_path_environment_variable(
            Self::NEA_DICPATH,
            &mut self.spl_dic_paths,
        ) || CliSpeller::process_path_environment_variable(
            Self::COMMON_DICPATH,
            &mut self.spl_dic_paths,
        );
        let _ = CliSpeller::process_path_environment_variable(
            Self::NEA_TESTPATH,
            &mut self.spl_test_paths,
        );
    }

    pub fn normalize_path(&self, path: &String) -> String {
        if self.spl_strict_slash {
            return path.clone();
        }
        if MAIN_SEPARATOR == '\\' {
            return path.replace("/", MAIN_SEPARATOR_STR); // windows
        } else {
            return path.replace("\\", MAIN_SEPARATOR_STR); // Linux
        }
    }

    pub fn parse_cli_options(&mut self, text_parser: &mut TextParser) {
        while let Some(arg) = self.csr_arg_tokens.get_next_arg() {
            if arg == "--strict-slash" {
                text_parser.tps_skip_output = true;
            } else if self.csr_options_finished || !arg.starts_with("-") {
                self.csr_text_files.push(arg.clone());
            } else if arg == "-d" {
                // compatible: dictionary name
                if let Some(arg_value) = self.csr_arg_tokens.get_arg_option() {
                    // language name "*" matches all aff files in any search directory
                    if self.csr_dict_codes.len() != 0 {
                        self.csr_dict_codes += ",";
                    }
                    self.csr_dict_codes += &self.normalize_path(&arg_value);
                }
            } else if arg == "--test" {
                if let Some(arg_value) = self.csr_arg_tokens.get_arg_option() {
                    // test name "*" matches all aff files in any search directory
                    for test_code in self.normalize_path(&arg_value).split(",") {
                        self.csr_test_codes.push (test_code.to_string());
                    }
                }
            } else if arg == "--test-word" {
                if let Some(arg_value) = self.csr_arg_tokens.get_arg_option() {
                    // test name "*" matches all aff files in any search directory
                    if self.csr_test_words.len() != 0 {
                        self.csr_test_words += ",";
                    }
                    self.csr_test_words += &arg_value;
                }
                //
            } else if arg == "--compat" {
                text_parser.tps_mode_flags |= ModeFlag::TestCompat as u32;
            } else if arg == "-D" {
                text_parser.tps_showing_details = true;
            } else if arg == "-q" {
                text_parser.tps_skip_output = true;
            } else if arg == "-l" {
                // compatible: list incorrect words
                text_parser.tps_check_level = 1;
            } else if arg == "-a" {
                // compatible: all output, report incorrect words with suggestions
                text_parser.tps_check_level = 2;
            } else if arg == "--out-file" {
                // output file instead of standard output
                if let Some(arg_value) = self.csr_arg_tokens.get_arg_option() {
                    self.spl_out_file_name = Some(arg_value);
                }
            } else if arg == "--max-notes" {
                // maximal number of notes per category
                if let Some(arg_value) = self.csr_arg_tokens.get_arg_option() {
                    text_parser.tps_max_notes = arg_value.parse::<u32>().unwrap();
                }
            } else if arg == "--warn" {
                if let Some(arg_value) = self.csr_arg_tokens.get_arg_option() {
                    for show_id in arg_value.split(',') {
                        if show_id == TextParser::SHOW_DUPLICATES {
                            text_parser.tps_warn.insert(TextParser::SHOW_DUPLICATES);
                        } else if show_id == TextParser::SHOW_DIC_OTHER {
                            text_parser.tps_warn.insert(TextParser::SHOW_DIC_OTHER);
                        } else {
                            println!("Unknown warning category: {arg_value}");
                        }
                    }
                }
            } else if arg == "--" {
                self.csr_options_finished = true;
            } else if arg == "-m" { // compatible: morphological description
                 /*
                    todo
                    necesita  st:necesitar fl:E
                    desambiguación  st:desambiguar fl:A
                    desambiguaciones  st:desambiguar fl:A fl:S
                 */
            } else {
                println!("Unknown option: {arg}");
            }
        }
    }

    pub fn open_out_file(&mut self, text_parser: &mut TextParser) -> io::Result<()> {
        if let Some(out_name) = &self.spl_out_file_name {
            let file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(out_name.clone());
            self.spl_out_writer = Some(Box::new(BufWriter::new(file?)));
        } else {
            self.spl_out_writer = Some(Box::new(stdout()));
        }
        if text_parser.tps_showing_details {
            text_parser.store_note(&format!("Neaspell {}", PROGRAM_VERSION));
        }
        Ok(())
    }

    fn matches_wildcarded(name: &str, pre_wild: &str, post_wild: &str) -> bool {
        name.starts_with(pre_wild) && name.ends_with(post_wild)
    }

    /// Returns the list of directory entries matching path_wildcarded.
    /// There can be one asterisk (only after the separator) and it means "any".
    /// Note: use OS specific directory separator, slash or backslash.
    ///
    /// A directory can be given
    /// A Directory with wildcard specification can be given
    pub fn list_wildcarded(path_wildcarded: &str) -> Vec<String> {
        let mut entry_vec: Vec<String> = vec![];
        if !path_wildcarded.contains("*") {
            entry_vec.push(String::from(path_wildcarded));
            return entry_vec;
        }
        let rsplit_separ = path_wildcarded.rsplit_once(MAIN_SEPARATOR);
        let mut path = path_wildcarded;
        let mut last_wildcarded = "";
        if let Some(pair) = rsplit_separ {
            (path, last_wildcarded) = pair;
        }
        let wildcarded_vec: Vec<&str> = last_wildcarded.split('*').collect(); // split at the wildcard
        let pre_wild = wildcarded_vec[0];
        let post_wild = if path_wildcarded.contains("*") {
            if wildcarded_vec.len() == 2 {
                wildcarded_vec[1]
            } else {
                ""
            }
            // todo warn if wildcarded_vec.len() > 2; not implemented
        } else {
            ""
        };
        let entries_opt = fs::read_dir(path);
        if let Ok(entries) = entries_opt {
            for entry_result in entries {
                let entry = entry_result.unwrap();
                let entry_all = format!("{}", entry.path().display());
                if let Ok(entry_last) = entry.file_name().into_string() {
                    // entry_last is the last part of file name after the last separator
                    if Self::matches_wildcarded(&entry_last, pre_wild, post_wild) {
                        entry_vec.push(entry_all);
                    }
                }
            }
        }
        entry_vec
    }

    const WILDCARD_STR: &'static str = "*"; // the only wildcard character defined

    /// Finds the base file names (without extension) given
    /// base file bame (aftewr the last separator, before extension).
    /// This can be language code ("es", "de_AT" or "*" or "de_med") or test code or something else.
    pub fn get_files_in_dirs_by_ext(
        base_file_name: &str,
        directories: &Vec<String>,
        file_ext: &str,
    ) -> Vec<String> {
        let mut dict_vec = vec![];
        for search_dir in directories {
            let having_wildcard = base_file_name.contains(Self::WILDCARD_STR);
            // the following is disabled
            //let name_wildcard = if having_wildcard {""} else {Self::WILDCARD_STR}; // if no wildcards, add one before extension
            let name_wildcard = "";
            let ext_wildcard = if having_wildcard {
                ""
            } else {
                Self::WILDCARD_STR
            };
            let path_wildcarded: String = format!(
                "{}{}{}{}.{}{}",
                search_dir, MAIN_SEPARATOR, base_file_name, name_wildcard, file_ext, ext_wildcard,
            );
            let mut dir_result = Self::list_wildcarded(&path_wildcarded);
            dict_vec.append(&mut dir_result);
            if dict_vec.len() != 0 && !having_wildcard {
                return dict_vec;
            }
        }
        dict_vec
    }

    pub fn expand_dict_file_name(&mut self, dict_name_ext: &str) -> Vec<String> {
        if dict_name_ext.is_empty() {
            return vec![];
        }
        let ext_code_vec: Vec<String> = if dict_name_ext.contains(MAIN_SEPARATOR) {
            vec![String::from(dict_name_ext)] // a specific file is given
        } else {
            // search within configured directories
            if dict_name_ext.ends_with(TextParser::EXT_AFF) {
                let mut name_parts: Vec<&str> = dict_name_ext.split(".").collect();
                _ = name_parts.pop();
                let base_name = name_parts.join(".");
                Self::get_files_in_dirs_by_ext(&base_name, &self.spl_test_paths, TextParser::EXT_AFF)
            } else if dict_name_ext.ends_with(TextParser::EXT_NEADIC) {
                let mut name_parts: Vec<&str> = dict_name_ext.split(".").collect();
                _ = name_parts.pop();
                let base_name = name_parts.join(".");
                Self::get_files_in_dirs_by_ext(&base_name, &self.spl_test_paths, TextParser::EXT_NEADIC)
            } else {
                Self::get_files_in_dirs_by_ext(
                    dict_name_ext,
                    &self.spl_test_paths,
                    TextParser::EXT_NEADIC,
                )
            }
        };
        ext_code_vec
    }

    /// Reads the dictionary for the 'lang_code'. 'base_file_name' is nearly full file name, it's only missing file extension.
    pub fn read_lang_single(
        &mut self,
        text_parser: &mut TextParser,
        lang_code: &str,
        base_file_name: String,
        including_tests: bool, 
    ) {
        let mut spell_lang = SpellLang::new(lang_code);
        spell_lang.slg_mode_flags = text_parser.tps_mode_flags;
        let ext_count: u32 = if including_tests {4} else {2}; // after so many loaded files, loading can stop
        let ext_vec = [TextParser::EXT_AFF, TextParser::EXT_DIC, TextParser::EXT_GOOD, TextParser::EXT_WRONG, TextParser::EXT_NEADIC];

        let mut load_count: u32 = 0;
        let mut missing_ext: Vec<String> = Vec::new();
        for file_ext in ext_vec {
            if file_ext == TextParser::EXT_NEADIC {
                if missing_ext.len() as u32 == ext_count {
                    // all the previous file extensions are missing, load from Self::EXT_NEADIC
                    missing_ext.clear();
                } else {
                    // some of the file extensions were present, but not all, don't load
                    break;
                }
            }
            let present = {
                let mut std_line_reader= StdLineReader::new (&base_file_name, file_ext);
                if std_line_reader.slr_reader.is_some() {
                    text_parser.parse_dictionary_text(&mut spell_lang, &mut std_line_reader);
                    if let Some(writer) = &mut self.spl_out_writer {
                        for line_note in &text_parser.tps_line_notes {
                            let _ = writeln!(writer, "{line_note}");
                        }
                    }
                    text_parser.tps_line_notes.clear();
                    true
                } else {
                    false
                }
            };
            if present {
                load_count+=1;
            } else {
                missing_ext.push(file_ext.to_string())
            }
            if load_count == ext_count {
                break;
            }
        }
        if load_count == 0 {
            text_parser.store_note(&format!(
                "Dictionary with base name not found: {base_file_name}"
            ));
        } else {
            for ext_str in missing_ext {
                text_parser.store_note(&format!(
                    "Missing file: {base_file_name}.{ext_str}",
                ));

            }
        }
        if text_parser.tps_showing_details {
            text_parser.store_noline_note(
                lang_code,
                TextParser::EXT_AFF,
                &Parser::get_summary(&spell_lang),
            );
        }
        text_parser.tps_langs.push(spell_lang);
    }

    /// Reads the dictionaries for the 'lang_code', e.g.
    /// "es*", "de_AT" or "*" or "de_med" or "../dict/de_CH".
    /// Slashes (/) or backslashes (\) are to be used depending on OS.
    /// todo if the aff file is missing (case: de_med), take the dictionary as extending the previous one
    pub fn read_lang_ext(&mut self, text_parser: &mut TextParser, lang_code_ext: &str) {
        let ext_code_vec: Vec<String> = self.expand_dict_file_name(lang_code_ext);
        for ext_code in ext_code_vec {
            let (dir, name_after_delim) = ext_code.rsplit_once(MAIN_SEPARATOR).unwrap();
            let plain_file_name = name_after_delim.split('.').next().unwrap(); // removed dot and the following characters, if any
            let base_file_name = format!("{}{}{}", dir, MAIN_SEPARATOR, plain_file_name);
            let lang_parts: Vec<&str> = plain_file_name.split('_').collect();
            let lang_code = if lang_parts.len() >= 2 {
                format!("{}_{}", lang_parts[0], lang_parts[1]) // skipping what is afterwards
            } else {
                format!("{}", lang_parts[0])
            };
            _ = self.read_lang_single(text_parser, &lang_code, base_file_name, false);
        }
    }

    /// Check several words or paragraph, not yet tokenized.
    /// The language (in the current code) is not yet known, several can be tried
    pub fn check_text(&self, text_parser: &mut TextParser, untokenized: &str) {
        for lang in &text_parser.tps_langs {
            // todo let each tokenization take only one token, not all
            // then it'll be possible to try languages in sequence until one succeeds
            let checked_tokens = Spell::check_text(&lang, untokenized);
            // todo depending on spl_check_level, let the function return more info
            for (word, token_type) in &checked_tokens {
                if word.len() == 0 {
                    continue;
                }
                if *token_type != TokenType::IsGoodWord && *token_type != TokenType::IsBadWord {
                    continue;
                }
                if !text_parser.tps_skip_output {
                    if text_parser.tps_check_level > 1 {
                        if *token_type == TokenType::IsGoodWord {
                            println!("*");
                        } else {
                            println!("& {}", &word);
                        }
                    } else {
                        if *token_type == TokenType::IsGoodWord {
                            // nothing to do
                        } else {
                            println!("{}", &word);
                        }
                    };
                }
                //println!("Word {}: {}", String::from(result_s), word);
            }
        }
    }

    pub fn check_text_file(&self, text_parser: &mut TextParser, text_name: &String) -> io::Result<()> {
        let file = File::open(text_name.clone())?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let untokenized = line?;
            self.check_text(text_parser, &untokenized);
        }
        //
        Ok(())
    }

    /// Runs a test case, either all words or a selection of words
    /// 'base_file_name' is nearly full file name, it's only missing file extension.
    /// 'test_case_name' is derived from 'base_file_name' and has no file separators.
    pub fn run_test_single(
        &mut self,
        text_parser: &mut TextParser,
        base_file_name: String,
        test_case_name: &str,
        test_words: &Vec<&str>,
    ) -> io::Result<()> {
        let _ = self.read_lang_single(text_parser, "", base_file_name.clone(), true);
        if text_parser.tps_langs.len() == 0 {
            return Ok(());
        }
        if text_parser.tps_langs.len() > 1 {
            text_parser.store_note(&format!("Too many languages"));
            return Ok(());
        }
        let lang = text_parser.tps_langs.pop().unwrap();
        for sec_ix in 0..3 {
            // three test sections: 0 bad grammar, 1 good words, 2 bad words
            if sec_ix == 0 && !text_parser.tps_testing_bad_gram {
                continue; // no such test
            }
            let expected_ok = sec_ix == 0 || sec_ix == 1;
            let sec_bad_gram = vec!["BAD-GRAM".to_string()];
            let word_vec = if sec_ix == 0 {
                &sec_bad_gram
            } else if sec_ix == 1 {
                &text_parser.tps_test_good_words
            } else {
                &text_parser.tps_test_bad_words
            };
            let word_vec = word_vec.to_owned();
            let extension = if sec_ix == 0 {
                "BAD-GRAM"
            } else if sec_ix == 1 {
                "GOOD-WORDS"
            } else {
                "BAD-WORDS"
            };
            for word in word_vec {
                if word.len() == 0 {
                    continue;
                }
                if test_words.len() != 0 && !test_words.contains(&word.as_str()) {
                    continue;
                }
                let test_passed = if sec_ix == 0 {
                    text_parser.tps_test_bad_gram_passed
                } else {
                    let check_result = Spell::check_token(&lang, &word);
                    expected_ok == check_result
                };
                if test_passed {
                    text_parser.tps_passed_count += 1;
                } else {
                    text_parser.tps_failed_count += 1;
                }
                if text_parser.tps_showing_details {
                    text_parser.store_noline_note(
                        &test_case_name,
                        extension,
                        &format!("{}: {}", if test_passed { "PASS" } else { "FAIL" }, word,),
                    );
                } else if !test_passed {
                    text_parser.store_note(&word);
                }
            }
        }
        if text_parser.tps_showing_details {
            if text_parser.tps_failed_count == 0 {
                text_parser.store_note(&format!(
                    "ALL {} tests PASSED: {}",
                    text_parser.tps_passed_count, test_case_name
                ));
            } else {
                text_parser.store_note(&format!(
                    "{} tests PASSED, {} tests FAILED: {}",
                    text_parser.tps_passed_count, text_parser.tps_failed_count, test_case_name
                ));
            }
        }
        Ok(())
    }

    /// Reads the test files and executes the tests. The test names are the base file names.
    /// Format 1 (compatible): test case consists of 4 files: aff, dic, good, wrong.
    pub fn run_test_ext(
        &mut self,
        text_parser: &mut TextParser,
        ext_code_vec: &Vec<String>,
        test_words: &Vec<&str>,
    ) {
        for ext_code in ext_code_vec {
            let (dir, name_after_delim) = ext_code.rsplit_once(MAIN_SEPARATOR).unwrap();
            let test_case_name = name_after_delim.split('.').next().unwrap(); // removed dot and the following characters, if any
            let base_file_name = format!("{}{}{}", dir, MAIN_SEPARATOR, test_case_name);
            _ = self.run_test_single(text_parser, base_file_name, test_case_name, test_words);
        }
    }

    pub fn execute_task(&mut self, text_parser: &mut TextParser) {
        if let Ok(_) = self.open_out_file(text_parser) {
            let dict_code_string = self.csr_dict_codes.clone();
            for dict_code_ext in dict_code_string.split(",") {
                self.read_lang_ext(text_parser, dict_code_ext);
                if self.csr_text_files.is_empty() {
                    // only parsing was interesting, now the language can be removed
                    let _lang = text_parser.tps_langs.pop();
                }
            }
            let test_word_string = self.csr_test_words.clone();
            let test_words: Vec<&str> = if self.csr_test_words.is_empty() {
                vec![]
            } else {
                test_word_string.split(",").collect()
            };
            for test_code_ext in self.csr_test_codes.to_owned() {
                let ext_code_vec = self.expand_dict_file_name(&test_code_ext);
                if ext_code_vec.is_empty() {
                    println!(
                        "Test {test_code_ext} was not found at directories listed by {}",
                        Self::NEA_TESTPATH
                    );
                }
                self.run_test_ext(text_parser, &ext_code_vec, &test_words);
            }
            for text_name in &self.csr_text_files {
                let _ = self.check_text_file(text_parser, &text_name);
            }
        } else {
            println!("Could not start");
        }
    }

    pub fn do_all(&mut self, args: Vec<String>) {
        let mut text_parser = TextParser::new();
        self.csr_arg_tokens.set_arguments(args);
        self.process_environment_variables();
        self.parse_cli_options(&mut text_parser);
        self.execute_task(&mut text_parser);
    }
}
