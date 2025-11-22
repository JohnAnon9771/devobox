.PHONY: all build clean

all: build

build:
	@echo "🏗️  Construindo imagem Devobox (Arch)..."
	@podman build -t devobox-img -f Containerfile .

	@echo "🐘 Criando Postgres (Parado)..."
	@podman rm -f postgres 2>/dev/null || true
	@podman create --name postgres \
		-p 5432:5432 \
		-e POSTGRES_USER=dev -e POSTGRES_PASSWORD=devpass -e POSTGRES_DB=dev_default \
		docker.io/postgres:16

	@echo "⚡ Criando Redis (Parado)..."
	@podman rm -f redis 2>/dev/null || true
	@podman create --name redis \
		-p 6379:6379 \
		docker.io/redis:7-alpine

	@echo "📦 Criando Container Devobox (Parado)..."
	@podman rm -f devobox 2>/dev/null || true
	@podman create --name devobox \
		--network host \
		--userns=keep-id \
		--security-opt label=disable \
		-v "$(HOME)/code:/home/dev/code" \
		-v "devobox_mise:/home/dev/.local/share/mise" \
		-w /home/dev \
		-it devobox-img \
		bash

	@echo "✅ Build concluído! Tudo pronto."
