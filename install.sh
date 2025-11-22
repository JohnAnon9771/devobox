#!/usr/bin/env bash
set -e

APP_NAME="devobox"
TARGET_CONFIG_DIR="$HOME/.config/$APP_NAME"
TARGET_BIN_DIR="$HOME/.local/bin"

echo "🚀 Instalando $APP_NAME..."

if ! command -v podman &> /dev/null; then
    echo "❌ Erro: Podman não está instalado."
    exit 1
fi

echo "📂 Configurando diretório em $TARGET_CONFIG_DIR..."
mkdir -p "$TARGET_CONFIG_DIR"
cp -r config/* "$TARGET_CONFIG_DIR/"

echo "🔗 Criando link simbólico em $TARGET_BIN_DIR..."
mkdir -p "$TARGET_BIN_DIR"
cp bin/devobox "$TARGET_CONFIG_DIR/devobox"
chmod +x "$TARGET_CONFIG_DIR/devobox"
ln -sf "$TARGET_CONFIG_DIR/devobox" "$TARGET_BIN_DIR/$APP_NAME"

echo "🏗️  Executando build inicial dos containers..."
cd "$TARGET_CONFIG_DIR"
make build

echo ""
echo "✅ Instalação concluída com sucesso!"
echo "Certifique-se que $TARGET_BIN_DIR está no seu PATH."
echo "👉 Digite '$APP_NAME shell' para começar."
