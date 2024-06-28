// The main function that calls the library to do all.
use neaspell_std::CliSpeller;
use std::env;

fn main() {
    let mut cli_speller = CliSpeller::new();
    cli_speller.do_all(env::args().collect());
}
/*
cd C:\0prog\spelling\neaspell
cd /mnt/c/0prog/spelling/neaspell/
cargo build --release
cargo --quiet run -- --test tests/affix1.neadic
cargo run -- -D -d *
(time target/release/neaspell -d ../dict/es_ES -l ../test/es-espanol.txt) > ../test/nea-es-espanol.txt 2>&1
valgrind --tool=callgrind target/release/neaspell -q -d ../dict/es_ES -l ../test/es-espanol.txt
callgrind_annotate --inclusive=yes callgrind.out.56199
*/
