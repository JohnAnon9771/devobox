# 🧰 Devobox

> **Ambiente de desenvolvimento isolado e persistente usando Distrobox**

Container de desenvolvimento baseado em Arch Linux que isola completamente seu ambiente do sistema host, mantendo performance nativa.

💡 **Para quem**: Devs que querem ferramentas modernas sem "poluir" o sistema
⚡ **Benefício**: Host limpo + ambiente completo + performance nativa (sem VMs)

---

## 🚀 Quick Start

### Pré-requisitos

```bash
distrobox --version  # Verificar instalação
```

Não tem? Instale em: https://distrobox.it

### Instalação (4 passos)

```bash
# 1. Baixar configuração
mkdir -p ~/.config/distrobox
curl -fLo ~/.config/distrobox/distrobox.ini \
  https://raw.githubusercontent.com/JohnAnon9771/devobox/main/distrobox.ini

# 2. Crie pasta para home do container
mkdir -p ~/devobox

# 3. Criar container
distrobox assemble create --file ~/.config/distrobox/distrobox.ini

# 4. Criar atalho (opcional)
mkdir -p ~/.local/bin
cat > ~/.local/bin/devobox << 'EOF'
#!/usr/bin/env bash
distrobox enter -nw devobox
EOF
chmod +x ~/.local/bin/devobox
```

**Pronto!** Entre com `devobox` ou `distrobox enter -nw devobox`

---

## 📦 O que vem configurado

### Ferramentas

| 🛠️ Tool  | Descrição                                        |
| -------- | ------------------------------------------------ |
| Neovim   | Editor configurado                               |
| Zellij   | Terminal multiplexer                             |
| Lazygit  | Git TUI                                          |
| Mise     | Gerenciador de versões (Node, Ruby, Python, etc) |
| Starship | Prompt moderno                                   |
| Ripgrep  | Busca ultrarrápida                               |

### Volumes

- `~/code` → Seus projetos
- `~/.ssh` → Chaves SSH

### Isolamento

Home separado em `~/devobox/` no host. Histórico, cache e configs isolados do sistema.

---

## 💻 Uso

### Comandos básicos

```bash
# Entrar
devobox

# Sair
exit  # ou Ctrl+D

# Dentro do container:
sudo pacman -S pacote              # Instalar pacotes Arch
mise install node@20               # Instalar Node 20
mise use --global node@20          # Usar globalmente
zellij attach -c dev               # Sessão Zellij
```

---

## 🔗 Inicialização automática do Starship e Mise

O Devobox configura automaticamente o **Starship** (prompt) e o **Mise** (gerenciador de versões) na primeira vez que o container é criado. O arquivo `~/.profile` é baixado e carregado em todas as sessões.

Se precisar reconfigurar manualmente:

```bash
# Recriar profile
curl -fLo ~/.profile https://raw.githubusercontent.com/JohnAnon9771/devobox/main/config/profile
source ~/.profile
```

---

## ⚙️ Personalização

### Adicionar pacotes

Edite `distrobox.ini`:

```ini
additional_packages="neovim git docker postgresql seu-pacote"
```

### Montar mais volumes

```ini
volume=~/Documents:~/Documents
volume=~/Downloads:~/Downloads
```

### Adicionar hooks

```ini
init_hooks="curl -fLo ~/.local/bin/script https://example.com/script.sh"
```

### Aplicar mudanças

```bash
distrobox rm devobox
distrobox assemble create --file ~/.config/distrobox/distrobox.ini
```

---

## 🔧 Troubleshooting

<details>
<summary><strong>Aplicações UI não abrem ou esta com problemas de permissões</strong></summary>

```bash
sudo chown -R $USER:$USER ~/devobox
```

</details>

<details>
<summary><strong>Container não inicia</strong></summary>

```bash
distrobox list
podman ps -a
distrobox rm devobox
distrobox assemble create --file ~/.config/distrobox/distrobox.ini
```

</details>

<details>
<summary><strong>SSH não funciona</strong></summary>

```bash
ls -la ~/.ssh  # Verificar se montou
```

Se vazio, adicione `volume=~/.ssh:~/.ssh` no `distrobox.ini` e recrie.

</details>

<details>
<summary><strong>Neovim/Zellij não inicializam</strong></summary>

```bash
# Neovim
rm -rf ~/.config/nvim
git clone --depth 1 https://github.com/JohnAnon9771/my-nvim.git ~/.config/nvim

# Zellij
mkdir -p ~/.config/zellij
curl -fLo ~/.config/zellij/config.kdl \
  https://raw.githubusercontent.com/JohnAnon9771/devobox/main/config/zellij.kdl
```

</details>

---

## 📋 Comandos de Gerenciamento

```bash
distrobox list                     # Listar containers
distrobox stop devobox             # Parar
distrobox rm devobox               # Remover (⚠️ apaga ~/devobox/)
distrobox-export --bin /usr/bin/nvim --export-path ~/.local/bin  # Exportar para host
```

---

## 🙏 Agradecimentos

Obrigado ao **[Distrobox](https://distrobox.it)** 💜

O Distrobox é uma ferramenta **excepcional** que resolve um problema real de forma elegante: permite usar qualquer distribuição Linux dentro de containers, com integração perfeita ao sistema host, sem overhead de VMs e sem complexidade desnecessária.

---
