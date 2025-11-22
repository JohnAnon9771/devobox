.SHELLFLAGS := -eu -o pipefail -c
SHELL := /bin/bash
DB_CONF ?= databases.conf

.PHONY: all build clean

all: build

build:
	@echo "🏗️  Construindo imagem Devobox (Arch)..."
	@podman build -t devobox-img -f Containerfile .

	@echo "🗄️  Lendo bancos de dados em $(DB_CONF)..."
	@if [ ! -f "$(DB_CONF)" ]; then \
		echo "❌ Arquivo $(DB_CONF) não encontrado"; \
		exit 1; \
	fi
	@while IFS='|' read -r name image ports env_vars; do \
		name="$$(echo $$name | xargs)"; \
		image="$$(echo $$image | xargs)"; \
		ports="$$(echo $$ports | xargs)"; \
		env_vars="$$(echo $$env_vars | xargs)"; \
		if [ -z "$$name" ] || [[ "$$name" == \#* ]]; then \
			continue; \
		fi; \
		echo "🧹 Removendo $$name (se existir)..."; \
		podman rm -f "$$name" 2>/dev/null || true; \
		create_args=""; \
		if [ -n "$$ports" ]; then \
			IFS=',' read -ra port_list <<< "$$ports"; \
			for port in "$${port_list[@]}"; do \
				port="$${port// /}"; \
				[ -n "$$port" ] && create_args="$$create_args -p $$port"; \
			done; \
		fi; \
		if [ -n "$$env_vars" ]; then \
			IFS=',' read -ra env_list <<< "$$env_vars"; \
			for env_var in "$${env_list[@]}"; do \
				env_var="$${env_var// /}"; \
				[ -n "$$env_var" ] && create_args="$$create_args -e $$env_var"; \
			done; \
		fi; \
		echo "📦 Criando $$name (Parado)..."; \
		podman create --name "$$name" $$create_args "$$image"; \
	done < "$(DB_CONF)"

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
