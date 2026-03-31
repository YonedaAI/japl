#!/bin/bash
set -e
echo "Installing JAPL..."
command -v cargo >/dev/null 2>&1 || { echo "Rust required: https://rustup.rs"; exit 1; }
command -v wat2wasm >/dev/null 2>&1 || { echo "wat2wasm required: brew install wabt"; exit 1; }

JAPL_HOME="${JAPL_HOME:-$HOME/.japl}"
mkdir -p "$JAPL_HOME/bin"

if [ -d "$JAPL_HOME/src" ]; then
  cd "$JAPL_HOME/src" && git pull
else
  git clone https://github.com/YonedaAI/japl.git "$JAPL_HOME/src"
fi

cd "$JAPL_HOME/src/japl-compiler" && cargo build --release 2>&1 | tail -1
cd "$JAPL_HOME/src/japl-runtime" && cargo build --release 2>&1 | tail -1
cp "$JAPL_HOME/src/japl-compiler/target/release/japl-compiler" "$JAPL_HOME/bin/japl-compiler"
cp "$JAPL_HOME/src/japl-runtime/target/release/japl-runtime" "$JAPL_HOME/bin/japl-runtime"
cp "$JAPL_HOME/src/bin/japl" "$JAPL_HOME/bin/japl"
chmod +x "$JAPL_HOME/bin/japl-compiler" "$JAPL_HOME/bin/japl-runtime" "$JAPL_HOME/bin/japl"

echo "JAPL installed! export PATH=\"$JAPL_HOME/bin:\$PATH\""
