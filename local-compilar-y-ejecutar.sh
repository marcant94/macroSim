set -e
cargo build --release
mkdir -p bin
rm -f bin/simulador_politica
mv -f target/release/simulador_politica bin/
./bin/simulador_politica
