# About Neaspell

Neaspell is a spelling-check library (work-in-progress) and CLI program written in memory-safe language Rust. Neaspell can be used in the internet when compiled into wasm. Development and testing are done on Windows and Linux.

With the Libreoffice set of dictionaries, it can spell the Spanish or English texts.

# Building
From the Rust web page
```
https://www.rust-lang.org/tools/install
```
install rustup. From the rust workspace directory (the one with README.md file), compile with
```
cargo build
```
# Running from Windows or Linux console
The following example assumes the dictionary files ../aff-dic/es_ES.aff and ../aff-dic/es_ES.dic. The text to be spell-checked is in the ../tests/es1.txt file. Then run
```
cargo --quiet run -- -d ../aff-dic/es_ES ../tests/es1.txt -l
```
The command outputs the words with spelling errors. In Powershell, the backslashes can be used, too.

# Running from browser
Install wasm-pack from
```
https://rustwasm.github.io/wasm-pack/installer/
```
and a local http server with
```
cargo install miniserve
```
From the neaspell_wasm directory, compile the example and then run locally
```
wasm-pack build --target web
miniserve . --index "src\index.html" -p 8080
```
Then you can see the example web page at
```
http://127.0.0.1:8080/
```

# Testing
The tests are run on Linux and Windows in bash shell.
In Windows, git bash is used for testing.
In Windows, Powershell can also be used.

## In bash with script, optionally including hunspell
Here is an example in bash. From the neaspell directory, run 
```
export NEA_TESTPATH="tests"
export INT_TEST_CMD="cargo --quiet run -- --compat --test"
tests/neatest.sh affix1 charcase1
```
The script neatest.sh outputs the names of commands that are run.
If the tests pass, nothing else is written to the output.

If the hunspell is checked out next to the neaspell, the
tests can be run in both programs.
Make sure the hunspell command is also available. Then execute in the console
```
export NEA_TESTPATH="tests"
export EXT_TEST_DIR=../hunspell/tests
export INT_TEST_CMD="cargo --quiet run -- --compat --test"
export EXT_TEST_CMD="../hunspell/tests/test.sh"
tests/neatest.sh --int affix1 charcase1
```

## Without the script
In Powershell, run
```
$env:nea_testpath="tests"
cargo --quiet run -- --compat --test affix1,charcase1
```
If the test passes, nothing is written to the output.
To see details, add option -D before --compat.
In Linux, instead of the first line, set the variable with 
```
export NEA_TESTPATH="tests"
```
