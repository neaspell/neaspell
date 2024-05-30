// The file processes files and character sets (encoding) and environment variables.

mod engine;
use engine::Lang;
use engine::ParseNote;
use engine::ParseState;
use engine::ParseStatus;
pub use engine::ModeFlag;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::BufWriter;
use std::io::{self, prelude::*, BufReader};
use std::path::{MAIN_SEPARATOR, MAIN_SEPARATOR_STR};
use std::str;

pub const PROGRAM_VERSION: &str = "0.1.4";

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
            if let Ok(line_utf8) = str::from_utf8(&bytes) {
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


#[derive(PartialEq,Clone, Copy)]
pub enum ParseMode {
    /// the line starts with a tag, as is in the .aff file
    Toplevel,
    /// the line contains word(s) of the .dic file or the initial count of the words
    WordDic,
    /// words passing the spelling rules
    PassTest,
    /// words failing the spelling rules
    FailTest,
}

// All the languages that are loaded
pub struct Speller {
    pub spl_check_level: u32,
    /// don't report problems with -l; for performance testing
    pub spl_skip_output: bool,
    pub spl_showing_details: bool,
    /// compatible processing, to have external test parity; there will be more spelling modes in the future
    pub spl_mode_flags: u32,
    /// search directories for the dictionaries
    pub spl_dic_paths: Vec<String>,
    /// if true, slash (/) or backslash (\) in file names must be
    /// according to the OS; by default false, both are interchangeable and are normalized
    pub spl_strict_slash: bool,
    /// search directories for the tests
    pub spl_test_paths: Vec<String>,
    pub spl_langs: Vec<Lang>,
    /// maximal number of notes
    pub spl_max_notes: u32,
    pub spl_warn: HashSet<&'static str>,
    pub spl_out_file_name: Option<String>,
    spl_out_writer: Option<BufWriter<File>>,
}

impl Speller {
    /// Option --warn value for duplicate words in dictionary
    pub const SHOW_DUPLICATES: &'static str = "dupl";
    /// Option --warn value for other dictionary problems
    pub const SHOW_DIC_OTHER: &'static str = "dic";

    // the file extensions
    const EXT_NEADIC: &'static str = "neadic";
    const EXT_AFF: &'static str = "aff";
    const EXT_DIC: &'static str = "dic";
    const EXT_GOOD: &'static str = "good";
    const EXT_WRONG: &'static str = "wrong";

    pub fn new() -> Speller {
        Speller {
            spl_check_level: 0,
            spl_skip_output: false,
            spl_showing_details: false,
            spl_mode_flags: 0,
            spl_dic_paths: vec![],
            spl_strict_slash: false,
            spl_test_paths: vec![],
            spl_langs: vec![],
            spl_max_notes: 10,
            spl_warn: HashSet::new(),
            spl_out_file_name: None,
            spl_out_writer: None,
        }
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

    pub fn open_out_file(&mut self) -> io::Result<()> {
        if let Some(out_name) = &self.spl_out_file_name {
            let file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(out_name.clone());
            self.spl_out_writer = Some(BufWriter::new(file?));
        }
        if self.spl_showing_details {
            self.write_output(&format!("Neaspell {}", PROGRAM_VERSION));
        }
        Ok(())
    }

    fn write_output(&mut self, s: &str) {
        if let Some(writer) = &mut self.spl_out_writer {
            let _ = writeln!(writer, "{s}");
        } else {
            println!("{s}");
        }
    }

    fn show_line_note(
        &mut self,
        file_code: &str,
        file_ext: &str,
        line_no: u32,
        line: &str,
        desc: &str,
    ) {
        if self.spl_showing_details {
            let out_text = if line_no != 0 {
                format!("{}.{}:{}: {}: {}", file_code, file_ext, line_no, desc, line)
            } else {
                format!("{}.{}: {}", file_code, file_ext, desc)
            };
            self.write_output(&out_text);
        }
    }

    fn show_noline_note(&mut self, file_code: &str, file_ext: &str, desc: &str) {
        self.show_line_note(file_code, file_ext, 0, "", desc);
    }

    fn show_parse_note(
        &mut self,
        file_code: &str,
        file_ext: &str,
        line: &str,
        parse_note: &ParseNote,
    ) {
        self.show_line_note(
            file_code,
            file_ext,
            parse_note.psn_line_no,
            line,
            parse_note.psn_desc,
        );
    }

    /// Reads bytes until the end of line (byte 0x0a, LF)
    /// and converts them to a string (if encoding is ok) and stores the line into "lang".
    fn read_line_bytes(
        lang: &mut Lang,
        reader: &mut BufReader<File>,
        line_no: u32,
    ) {
        let mut line_buf = Vec::<u8>::with_capacity(1024);
        let result = &reader.read_until(10, &mut line_buf);
        if let Ok(_) = result {
        } else {
            // io error, stop loop
            lang.lng_parse_status = ParseStatus::FileEnded;
            lang.lng_parsed_line = String::from("");
            return;
        }
        if line_buf.len() == 0 {
            // nothing more to read, not even end of line
            lang.lng_parse_status = ParseStatus::FileEnded;
            lang.lng_parsed_line = String::from("");
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
            if line_buf[ci] != 32 && line_buf[ci] != 9 { // space or tab characters
                is_non_empty = true;
                break; // don't treat '#' as comment if non-space is before it
            }
        }
        // bytes_to_string
        if let Ok(line_as_string) = Encoding::bytes_to_string(&line_buf, &lang.slg_set) {
            let mut line_as_string = line_as_string;
            if line_as_string.ends_with("\r\n") {
                line_as_string.pop();
                line_as_string.pop();
            } else if line_as_string.ends_with("\n") {
                line_as_string.pop();
            };
            lang.lng_parse_status = if is_non_empty {ParseStatus::LineReady} else {ParseStatus::EncodingErrorOrEmpty};
            lang.lng_parsed_line = line_as_string;
        } else {
            lang.lng_parse_status = ParseStatus::EncodingErrorOrEmpty;
            lang.lng_parsed_line = String::from("");
        }
    }

    fn show_file_summary(
        &mut self,
        extension: &str,
        lang_code: &str,
        bad_encoding: u32,
        note_count: u32,
    ) {
        if bad_encoding != 0 {
            self.show_noline_note(
                &lang_code,
                extension,
                &format!(
                    "Lines with bad character encoding: {}",
                    &bad_encoding.to_string()
                ),
            );
        }
        if note_count != 0 {
            self.show_noline_note(
                &lang_code,
                extension,
                &format!("Parse errors: {}", &note_count.to_string()),
            );
        }
    }

    pub fn finalize_description_part (&mut self, lang: &mut Lang) {
        let notes = &lang.finalize_parsing();
        if self.spl_showing_details {
            let mut note_count = 0;
            for note in notes {
                if note_count < self.spl_max_notes {
                    self.show_noline_note(&lang.slg_code, Self::EXT_AFF, &note);
                }
                note_count += 1;
            }
            if note_count != 0 {
                self.show_noline_note(
                    &lang.slg_code,
                    Self::EXT_AFF,
                    &format!("Total final notes: {}", &note_count.to_string()),
                );
            }
        }
    }

    fn parse_charset (lang: &mut Lang, parse_state: &mut ParseState) {
        // the SET tag
        if let Some(set_value) = parse_state.get_next_token() {
            let mut name_valid = false;
            for set_name in Encoding::CHAR_SET_NAME {
                if set_value == set_name {
                    name_valid = true;
                    lang.slg_set = set_value.to_string();
                    break;
                }
            }
            if !name_valid {
                parse_state.add_note ("SET element *limitation*: this encoding is not yet implemented");
            }
        } else {
            parse_state.add_note("No value for SET element");
        }
    }


    fn parse_nea_token(lang: &mut Lang, parse_state: &mut ParseState) -> ParseMode {
        // NEA DIC {
        // NEA TPASS {
        // NEA TFAIL {
        let mut next_mode = ParseMode::Toplevel;
        if let Some(nea2) = parse_state.get_next_token() {
            if nea2 == "DIC" {
                next_mode = ParseMode::WordDic;
            }
            else if nea2 == "TESTGOOD" {
                next_mode = ParseMode::PassTest;
            }
            else if nea2 == "TESTBAD" {
                next_mode = ParseMode::FailTest;
            }
            else {
                parse_state.add_note("Unknown keyword after NEA tag");
            }
        }
        if next_mode != ParseMode::Toplevel {
            lang.lng_mode_until_brace = true;
            if let Some(nea3) = parse_state.get_next_token() {
                if nea3 != "{" {
                    parse_state.add_note("Expected open brace '{' but found something else");
                }
            }
            else {
                parse_state.add_note("Expected open brace '{' but found nothing");
            }
        }
        next_mode
    }

    fn show_parse_notes(&mut self, file_code: &str, file_ext:&str, parse_state: &ParseState, line_as_string:&String, note_count: &mut u32) {
        for parse_note in parse_state.get_notes() {
            if *note_count < self.spl_max_notes {
                self.show_parse_note(
                    &file_code,
                    file_ext,
                    &line_as_string,
                    &parse_note,
                );
            } else if *note_count == self.spl_max_notes {
                self.show_noline_note(
                    &file_code,
                    file_ext,
                    "Next parse errors not shown",
                );
            }
            *note_count += 1;
        }

    }

    /// Opens one dictionary file, if it exists, and parses it. 
    /// Each file_ext has it's associated starting parse_mode.
    /// Returns true if file exists.
    pub fn read_dictionary_file(&mut self, lang: &mut Lang, base_file_name: &String, file_ext: &str, mut parse_mode:ParseMode) -> bool {
        let full_file_name = base_file_name.clone() + "." + &file_ext;
        let file_result = File::open(full_file_name.clone());
        let mut reader;
        if let Ok(file) = file_result {
            reader = BufReader::new(file);
        } else {
            return false;
        }
        self.show_noline_note(&lang.slg_code, file_ext, &format!("Parsing: {full_file_name}"));
        let mut line_no = 0;
        let mut note_count: u32 = 0;
        let bad_encoding: u32 = 0;
        let reporting_dupl = self.spl_warn.contains(Self::SHOW_DUPLICATES);
        let reporting_other = self.spl_warn.contains(Self::SHOW_DIC_OTHER);
        let orig_parse_mode = parse_mode; // for the whole file
        let mut finalized = false;
        loop {
            let parse_mode_before_line = parse_mode;
            line_no += 1;
            Self::read_line_bytes(lang, &mut reader, line_no);
            let parsed_line = lang.lng_parsed_line.clone();
            if lang.lng_parse_status == ParseStatus::FileEnded {
                break;
            }
            if lang.lng_parse_status == ParseStatus::EncodingErrorOrEmpty {
                continue;
            }
            // the file line is found to be non-empty
            let mut line_tokens = parsed_line.split_whitespace();
            let mut parse_state = ParseState::new (line_no, &mut line_tokens,);
            if parse_state.get_first_token() == "}" && lang.lng_mode_until_brace {
                parse_mode = ParseMode::Toplevel;
                lang.lng_mode_until_brace = false;
                // todo check no more tokens
            } else if parse_mode == ParseMode::Toplevel {
                if parse_state.get_first_token() == "SET" {
                    Self::parse_charset (lang, &mut parse_state);
                } if parse_state.get_first_token() == "NEA" {
                    parse_mode = Self::parse_nea_token (lang, &mut parse_state);
                } else {
                    lang.parse_aff_line(&mut parse_state);
                }
            } else if orig_parse_mode == ParseMode::WordDic && lang.slg_dic_count == 0 { // .dic file, 1st line
                lang.parse_dictionary_count (&mut parse_state);
            } else if parse_mode == ParseMode::WordDic {
                lang.parse_dic_line(&mut parse_state,reporting_dupl,reporting_other,);
            } else if parse_mode == ParseMode::PassTest {
                lang.lng_pass_expected.push (parse_state.get_first_token().to_string());
            } else if parse_mode == ParseMode::FailTest {
                lang.lng_fail_expected.push (parse_state.get_first_token().to_string());
            }
            self.show_parse_notes(&lang.slg_code, file_ext, &mut parse_state, &parsed_line, &mut note_count);
            if orig_parse_mode == ParseMode::Toplevel && parse_mode_before_line != parse_mode && !finalized{
                self.finalize_description_part (lang);
                finalized = true;
            }
        }
        self.show_file_summary(file_ext, &lang.slg_code, bad_encoding, note_count);
        if orig_parse_mode == ParseMode::Toplevel && parse_mode == ParseMode::Toplevel && !finalized{
            // finalizing
            self.finalize_description_part (lang);
        }
        true
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

    /// Reads the dictionary for the 'lang_code'. 'base_file_name' is nearly full file name, it's only missing file extension.
    /// ext_count:
    /// 2: aff, dic
    /// 4: aff, dic, good, wrong
    pub fn read_lang_single(&mut self, lang_code: &str, base_file_name: String, ext_count: u32) -> io::Result<()> {
        let mut lang = Lang::new(lang_code);
        lang.lng_mode_flags = self.spl_mode_flags;
        //let neadic_name = base_file_name.clone() + "." + Self::EXT_NEADIC;

        let mut neadic_present = false;
        let mut group_present = true; // two or four files: aff, dic; good, wrong
        let mut aff_present = false;
        let mut dic_present = false;
        let mut good_present = false;
        let mut wrong_present = false;
        if ext_count == 2 || ext_count == 4 {
            aff_present = self.read_dictionary_file (&mut lang, &base_file_name, Self::EXT_AFF, ParseMode::Toplevel);
            group_present = aff_present;
            if group_present {
                dic_present = self.read_dictionary_file (&mut lang, &base_file_name, Self::EXT_DIC, ParseMode::WordDic);
                group_present = dic_present;
            }
        }
        if ext_count == 4 && group_present {
            good_present = self.read_dictionary_file (&mut lang, &base_file_name, Self::EXT_GOOD, ParseMode::PassTest);
            group_present = good_present;
            if group_present {
                wrong_present = self.read_dictionary_file (&mut lang, &base_file_name, Self::EXT_WRONG, ParseMode::FailTest);
                //group_present = wrong_present;
            }
        }
        if aff_present {
            if !dic_present {
                self.write_output(&format!("Missing file: {base_file_name}.{}", Self::EXT_DIC));
            }
            if ext_count == 4 {
                if !good_present {
                    self.write_output(&format!("Missing file: {base_file_name}.{}", Self::EXT_GOOD));
                }
                if !wrong_present {
                    self.write_output(&format!("Missing file: {base_file_name}.{}", Self::EXT_WRONG));
                }
            }
        } else {
            // tryinf neadic
            neadic_present = self.read_dictionary_file (&mut lang, &base_file_name, Self::EXT_NEADIC, ParseMode::Toplevel);
        }
        if !aff_present && !neadic_present {
            self.write_output(&format!("Dictionary with base name not found: {base_file_name}"));
        }
        if self.spl_showing_details {
            self.show_noline_note(lang_code, Self::EXT_AFF, &lang.get_summary());
        }
        self.spl_langs.push(lang);
        Ok(())
    }

    /// Reads the dictionaries for the 'lang_code', e.g.
    /// "es*", "de_AT" or "*" or "de_med" or "../dict/de_CH".
    /// Slashes (/) or backslashes (\) are to be used depending on OS.
    /// todo if the aff file is missing (case: de_med), take the dictionary as extending the previous one
    pub fn read_lang_ext(&mut self, lang_code_ext: &str) {
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
            _ = self.read_lang_single(&lang_code, base_file_name, 2);
        }
    }

    /// Check several words or paragraph, not yet tokenized.
    /// The language (in the current code) is not yet known, several can be tried
    pub fn check_untokenized(&self, untokenized: &str) {
        for lang in &self.spl_langs {
            // todo let each tokenization take only one token, not all
            // then it'll be possible to try languages in sequence until one succeeds
            let checked_words = lang.check_untokenized(untokenized);
            // todo depending on spl_check_level, let the function return more info
            for (word, check_result) in &checked_words {
                if word.len() == 0 {
                    continue;
                }
                if !self.spl_skip_output {
                    if self.spl_check_level > 1 {
                        if *check_result {
                            println!("*");
                        } else {
                            println!("& {}", &word);
                        }
                    } else {
                        if *check_result {
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

    pub fn check_text_file(&self, text_name: &String) -> io::Result<()> {
        let file = File::open(text_name.clone())?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let untokenized = line?;
            self.check_untokenized(&untokenized);
        }
        //
        Ok(())
    }

    /// Runs a test case, either all words or a selection of words
    /// 'base_file_name' is nearly full file name, it's only missing file extension.
    /// 'test_case_name' is derived from 'base?file_name' and has no file separators.
    pub fn run_test_single(
        &mut self,
        base_file_name: String,
        test_case_name: &str,
        test_words: &Vec<&str>,
    ) -> io::Result<()> {
        let _ = self.read_lang_single("", base_file_name.clone(), 4);
        if self.spl_langs.len() == 0 {
            return Ok(());
        }
        if self.spl_langs.len() > 1 {
            self.write_output(&format!("Too many languages"));
            return Ok(());
        }
        let mut lang = self.spl_langs.pop().unwrap();
        for name_ix in 0..2 {
            // first try good words, then try bad words
            let expected_ok = name_ix == 0;
            let word_vec = if name_ix == 0 {&lang.lng_pass_expected} else {&lang.lng_fail_expected};
            let extension = if name_ix == 0 {"good"} else {"bad"};
            for word in word_vec {
                if word.len() == 0 {
                    continue;
                }
                if test_words.len() != 0 && !test_words.contains(&word.as_str()) {
                    continue;
                }
                let check_result = lang.check_token(&word);
                let test_passed = expected_ok == check_result;
                if test_passed {
                    lang.lng_passed_count+=1;
                } else {
                    lang.lng_failed_count+=1;
                }
                if self.spl_showing_details {
                    self.show_noline_note(
                        &test_case_name,
                        extension,
                        &format!("{}: {}", if test_passed { "PASS" } else { "FAIL" }, word,),
                    );
                } else if !test_passed {
                    self.write_output(word);
                }
            }
        }
        if self.spl_showing_details {
            if lang.lng_failed_count == 0 {
                self.write_output(&format!("ALL {} tests PASSED: {}",
                    lang.lng_passed_count, test_case_name));
            } else {
                self.write_output(&format!("{} tests PASSED, {} tests FAILED: {}", 
                    lang.lng_passed_count, lang.lng_failed_count, test_case_name));
            }
        }
        Ok(())
    }

    pub fn expand_dict_file_name(&mut self, dict_name_ext: &str) -> Vec<String>{
        if dict_name_ext.is_empty() {
            return vec![];
        }
        let ext_code_vec: Vec<String> = if dict_name_ext.contains(MAIN_SEPARATOR) {
            vec![String::from(dict_name_ext)] // a specific file is given
        } else {
            // search within configured directories
            if dict_name_ext.ends_with(Self::EXT_AFF) {
                let mut name_parts: Vec<&str> = dict_name_ext.split(".").collect();
                _ = name_parts.pop();
                let base_name = name_parts.join(".");
                Self::get_files_in_dirs_by_ext(&base_name, &self.spl_test_paths, Self::EXT_AFF)
            }
            else if dict_name_ext.ends_with(Self::EXT_NEADIC) {
                let mut name_parts: Vec<&str> = dict_name_ext.split(".").collect();
                _ = name_parts.pop();
                let base_name = name_parts.join(".");
                Self::get_files_in_dirs_by_ext(&base_name, &self.spl_test_paths, Self::EXT_NEADIC)
            }
            else {
                Self::get_files_in_dirs_by_ext(dict_name_ext, &self.spl_test_paths, Self::EXT_NEADIC)
            }
        };
        ext_code_vec
    }

    /// Reads the test files and executes the tests. The test names are the base file names.
    /// Format 1 (compatible): test case consists of 4 files: aff, dic, good, wrong.
    pub fn run_test_ext(&mut self, ext_code_vec: &Vec<String>, test_words: &Vec<&str>) {
        for ext_code in ext_code_vec {
            let (dir, name_after_delim) = ext_code.rsplit_once(MAIN_SEPARATOR).unwrap();
            let test_case_name = name_after_delim.split('.').next().unwrap(); // removed dot and the following characters, if any
            let base_file_name = format!("{}{}{}", dir, MAIN_SEPARATOR, test_case_name);
            _ = self.run_test_single(base_file_name, test_case_name, test_words);
        }
    }
}
