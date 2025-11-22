#!/usr/bin/env bash
set -e

APP_NAME="devobox"
TARGET_CONFIG_DIR="$HOME/.config/$APP_NAME"
TARGET_BIN_DIR="$HOME/.local/bin"
BACKUP_DIR="${TARGET_CONFIG_DIR}.bak"

FORCE=false

for arg in "$@"; do
    case "$arg" in
        --force)
            FORCE=true
            ;;
        -h|--help)
            echo "Uso: ./install.sh [--force]"
            echo "  --force  Sobrescreve configuração existente sem perguntar"
            exit 0
            ;;
        *)
            echo "⚠️  Argumento desconhecido: $arg"
            echo "Uso: ./install.sh [--force]"
            exit 1
            ;;
    esac
done

echo "🚀 Instalando $APP_NAME..."

if ! command -v podman &> /dev/null; then
    echo "❌ Erro: Podman não está instalado."
    exit 1
fi

if [ -d "$TARGET_CONFIG_DIR" ]; then
    if [ "$FORCE" = false ]; then
        echo "⚠️  Uma configuração existente foi encontrada em $TARGET_CONFIG_DIR."
        read -rp "Deseja sobrescrever? (y/N) " CONFIRM
        if [[ ! "$CONFIRM" =~ ^[Yy]$ ]]; then
            echo "ℹ️ Instalação abortada. Reexecute com --force para sobrescrever ou remova $TARGET_CONFIG_DIR."
            exit 1
        fi
    fi

    echo "🗂️  Criando backup em $BACKUP_DIR..."
    rm -rf "$BACKUP_DIR"
    cp -a "$TARGET_CONFIG_DIR" "$BACKUP_DIR"
fi

echo "📂 Configurando diretório em $TARGET_CONFIG_DIR..."
rm -rf "$TARGET_CONFIG_DIR"
mkdir -p "$TARGET_CONFIG_DIR"
cp -a config/. "$TARGET_CONFIG_DIR/"

echo "🔗 Criando link simbólico em $TARGET_BIN_DIR..."
mkdir -p "$TARGET_BIN_DIR"
cp bin/devobox "$TARGET_CONFIG_DIR/devobox"
chmod +x "$TARGET_CONFIG_DIR/devobox"
ln -sf "$TARGET_CONFIG_DIR/devobox" "$TARGET_BIN_DIR/$APP_NAME"

echo "🏗️  Executando build inicial dos containers..."
cd "$TARGET_CONFIG_DIR"
make build

if [[ ":$PATH:" != *":$TARGET_BIN_DIR:"* ]]; then
    echo "⚠️  $TARGET_BIN_DIR não está no seu PATH. Adicione com:"
    echo "    export PATH=\"$TARGET_BIN_DIR:$PATH\""
fi

echo ""
echo "✅ Instalação concluída com sucesso!"
echo "Certifique-se que $TARGET_BIN_DIR está no seu PATH."
echo "👉 Digite '$APP_NAME shell' para começar."
