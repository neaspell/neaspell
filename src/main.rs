// The file processes command-line options.
// It normalizes the slashes in file names.
// The option names and the variable names are defined here.

use std::env;
pub use neaspell::Speller;
pub use neaspell::ModeFlag;

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

pub struct CliSpeller {
    arg_tokens: ArgTokens,
    dict_codes: String, // comma-separated dictionary codes, possibly with asterisk wildcards, or
    // paths (with separators) to the dictionary files, without the file extension
    test_codes: String, // comma-separated test codes, possibly with asterisk wildcards
    test_words: String, // comma-separated test word, to filter-out the other words
    text_files: Vec<String>,
    options_finished: bool, // true after "--" argument
}

impl CliSpeller {
    pub fn new() -> CliSpeller {
        CliSpeller {
            arg_tokens: ArgTokens::new(),
            dict_codes: String::new(),
            test_codes: String::new(),
            test_words: String::new(),
            text_files: vec![],
            options_finished: false,
        }
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
    pub fn process_environment_variables(speller: &mut Speller) {
        // The first of the two variables is used if defined: NEA_DICPATH and DICPATH
        let _ = Speller::process_path_environment_variable(
            Self::NEA_DICPATH,
            &mut speller.spl_dic_paths,
        ) || Speller::process_path_environment_variable(
            Self::COMMON_DICPATH,
            &mut speller.spl_dic_paths,
        );
        let _ = Speller::process_path_environment_variable(
            Self::NEA_TESTPATH,
            &mut speller.spl_test_paths,
        );
    }

    pub fn do_all(&mut self, args: Vec<String>) {
        let mut speller = Speller::new();
        self.arg_tokens.set_arguments(args);
        Self::process_environment_variables(&mut speller);

        while let Some(arg) = self.arg_tokens.get_next_arg() {
            if arg == "--strict-slash" {
                speller.spl_skip_output = true;
            } else if self.options_finished || !arg.starts_with("-") {
                self.text_files.push(arg.clone());
            } else if arg == "-d" {
                // compatible: dictionary name
                if let Some(arg_value) = self.arg_tokens.get_arg_option() {
                    // language name "*" matches all aff files in any search directory
                    if self.dict_codes.len() != 0 {
                        self.dict_codes += ",";
                    }
                    self.dict_codes += &speller.normalize_path(&arg_value);
                }
            } else if arg == "--test" {
                if let Some(arg_value) = self.arg_tokens.get_arg_option() {
                    // test name "*" matches all aff files in any search directory
                    if self.test_codes.len() != 0 {
                        self.test_codes += ",";
                    }
                    self.test_codes += &speller.normalize_path(&arg_value);
                }
            } else if arg == "--test-word" {
                if let Some(arg_value) = self.arg_tokens.get_arg_option() {
                    // test name "*" matches all aff files in any search directory
                    if self.test_words.len() != 0 {
                        self.test_words += ",";
                    }
                    self.test_words += &arg_value;
                }
                //
            } else if arg == "--compat" {
                speller.spl_mode_flags |= ModeFlag::TestCompat as u32;
            } else if arg == "-D" {
                speller.spl_showing_details = true;
            } else if arg == "-q" {
                speller.spl_skip_output = true;
            } else if arg == "-l" {
                // compatible: list incorrect words
                speller.spl_check_level = 1;
            } else if arg == "-a" {
                // compatible: all output, report incorrect words with suggestions
                speller.spl_check_level = 2;
            } else if arg == "--out-file" {
                // output file instead of standard output
                if let Some(arg_value) = self.arg_tokens.get_arg_option() {
                    speller.spl_out_file_name = Some(arg_value);
                }
            } else if arg == "--max-notes" {
                // maximal number of notes per category
                if let Some(arg_value) = self.arg_tokens.get_arg_option() {
                    speller.spl_max_notes = arg_value.parse::<u32>().unwrap();
                }
            } else if arg == "--warn" {
                if let Some(arg_value) = self.arg_tokens.get_arg_option() {
                    for show_id in arg_value.split(',') {
                        if show_id == Speller::SHOW_DUPLICATES {
                            speller.spl_warn.insert(Speller::SHOW_DUPLICATES);
                        } else if show_id == Speller::SHOW_DIC_OTHER {
                            speller.spl_warn.insert(Speller::SHOW_DIC_OTHER);
                        } else {
                            println!("Unknown warning category: {arg_value}");
                        }
                    }
                }
            } else if arg == "--" {
                self.options_finished = true;
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
        if let Ok(_) = speller.open_out_file() {
            for dict_code_ext in self.dict_codes.split(",") {
                speller.read_lang_ext(dict_code_ext);
                if self.text_files.is_empty() {
                    // only parsing was interesting, now the language can be removed
                    let _lang = speller.spl_langs.pop();
                }
            }
            let test_words: Vec<&str> = if self.test_words.is_empty() {
                vec![]
            } else {
                self.test_words.split(",").collect()
            };
            for test_code_ext in self.test_codes.split(",") {
                let ext_code_vec = speller.expand_dict_file_name(test_code_ext);
                if ext_code_vec.is_empty() {
                    println!("Test {test_code_ext} was not found at directories listed by {}", Self::NEA_TESTPATH);
                }
                speller.run_test_ext(&ext_code_vec, &test_words);
            }
            for text_name in &self.text_files {
                let _ = speller.check_text_file(&text_name);
            }
        } else {
            println!("Could not start");
        }
    }
}

fn main() {
    let mut cli_speller = CliSpeller::new();
    cli_speller.do_all(env::args().collect());
}
/*
cd C:\0prog\spelling\neaspell
cd /mnt/c/0prog/spelling/neaspell/
cargo build --release
cargo run -- --quiet --test tests/affix1.neadic
cargo run -- -D -d *
(time target/release/neaspell -d ../dict/es_ES -l ../test/es-espanol.txt) > ../test/nea-es-espanol.txt 2>&1
valgrind --tool=callgrind target/release/neaspell -q -d ../dict/es_ES -l ../test/es-espanol.txt
callgrind_annotate --inclusive=yes callgrind.out.56199
*/
