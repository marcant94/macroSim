set -e

# Versión que se muestra en la esquina inferior. En CI la inyecta el workflow
# con el tag del release; en local se usa la fecha, igual que el tag.
export MACROSIM_VERSION="${MACROSIM_VERSION:-$(date -u +%Y%m%d-%H%M)}"
# `option_env!` no invalida la caché de cargo: se fuerza recompilar el crate.
touch src/main.rs

cargo build --release
mkdir -p bin
rm -f bin/simulador_politica
mv -f target/release/simulador_politica bin/
./bin/simulador_politica
