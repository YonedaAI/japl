#!/bin/bash
set -e

echo "Installing JAPL..."

# Detect OS and arch
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "$ARCH" in
  x86_64) ARCH="amd64" ;;
  aarch64|arm64) ARCH="arm64" ;;
esac

JAPL_HOME="${JAPL_HOME:-$HOME/.japl}"
BIN_DIR="$JAPL_HOME/bin"

mkdir -p "$BIN_DIR"

# For now: clone the repo and build from source
# (Pre-built binaries would be better but require CI)
echo "Building from source..."

# Check dependencies
command -v node >/dev/null 2>&1 || { echo "Error: Node.js required. Install from https://nodejs.org"; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo "Error: Rust required. Install from https://rustup.rs"; exit 1; }
command -v wat2wasm >/dev/null 2>&1 || { echo "Error: wat2wasm required. Install with: brew install wabt"; exit 1; }

# Clone or update
if [ -d "$JAPL_HOME/src" ]; then
  echo "Updating JAPL..."
  cd "$JAPL_HOME/src" && git pull
else
  echo "Downloading JAPL..."
  git clone https://github.com/YonedaAI/japl.git "$JAPL_HOME/src"
fi

# Build compiler
echo "Building compiler..."
cd "$JAPL_HOME/src/compiler/ts"
npm ci --silent
npx tsc

# Build runtime
echo "Building runtime..."
cd "$JAPL_HOME/src/japl-runtime"
cargo build --release 2>&1 | tail -1

# Create japl symlink
cp "$JAPL_HOME/src/bin/japl" "$BIN_DIR/japl"
chmod +x "$BIN_DIR/japl"

# Link runtime
ln -sf "$JAPL_HOME/src/japl-runtime/target/release/japl-runtime" "$BIN_DIR/japl-runtime"

echo ""
echo "JAPL installed to $JAPL_HOME"
echo ""
echo "Add to your PATH:"
echo "  export PATH=\"$BIN_DIR:\$PATH\""
echo ""
echo "Then run:"
echo "  japl version"
echo "  japl new myapp"
echo "  cd myapp && japl run src/main.japl"
