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

cd "$JAPL_HOME/src/japl" && cargo build --release 2>&1 | tail -1
cp "$JAPL_HOME/src/japl/target/release/japl" "$JAPL_HOME/bin/japl"
chmod +x "$JAPL_HOME/bin/japl"

echo "JAPL installed! Add to your shell config:"
echo "  export PATH=\"$JAPL_HOME/bin:\$PATH\""
echo ""
echo "Usage:"
echo "  japl build app.japl     # compile to .wasm"
echo "  japl run app.japl       # compile + run"
echo "  japl serve app.japl     # compile + serve HTTP"
echo "  japl check app.japl     # type check"
echo "  japl fmt app.japl       # format"
