#!/usr/bin/env bash
# Baquara RS - Script de compilação e instalação local
set -euo pipefail

SCRIPT_DIR="$(dirname "$(readlink -f "$0")")"
cd "$SCRIPT_DIR"

echo "==> [Baquara RS] Compilando binário otimizado em release..."
cargo build --release

echo "==> [Baquara RS] Instalando binário em ~/.local/bin..."
mkdir -p ~/.local/bin
install -m 755 target/release/baquara ~/.local/bin/baquara

# Criar wrapper para modo MCP se chamado como baquara-mcp
ln -sf ~/.local/bin/baquara ~/.local/bin/baquara-mcp

echo "==> [Baquara RS] Instalação concluída com sucesso!"
~/.local/bin/baquara --version
