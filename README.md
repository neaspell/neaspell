# About Neaspell

Neaspell is a work in progress. This is spelling-check library and CLI program written in memory-safe language Rust.

With the Libreoffice set of dictionaries, it can spell the Spanish texts.



# Tests
From the Rust web page
https://www.rust-lang.org/tools/install
install rustup.

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
