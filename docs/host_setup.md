# 🐧 Configuração do Host (Linux)

Este guia cobre como preparar sua máquina Linux para extrair o máximo de performance e conveniência do `devobox`.

## 1. Configurando o SSH Agent (Crítico para Git)

O `devobox` usa uma técnica chamada **SSH Agent Forwarding**. Isso permite que você clone repositórios privados dentro do container usando as chaves SSH que já estão na sua máquina, sem precisar copiá-las.

### Passo A: Verificar se você já tem chaves
Rode no seu terminal:
```bash
ls -al ~/.ssh
```
Se você ver arquivos como `id_ed25519` ou `id_rsa`, pule para o **Passo C**.

### Passo B: Gerar uma nova chave SSH
Se não tiver chaves, gere uma nova (recomendamos Ed25519):
```bash
ssh-keygen -t ed25519 -C "seu-email@exemplo.com"
```
Dê Enter para aceitar o local padrão. Você pode definir uma senha (passphrase) para segurança extra.

**Importante:** Lembre-se de adicionar a chave pública (`~/.ssh/id_ed25519.pub`) nas configurações do seu GitHub/GitLab.

### Passo C: Adicionar a chave ao Agente
Para que o `devobox` veja sua chave, ela precisa estar carregada no agente SSH do seu sistema.

1. **Inicie o agente** (se não estiver rodando):
   ```bash
   eval "$(ssh-agent -s)"
   ```

2. **Adicione sua chave**:
   ```bash
   ssh-add ~/.ssh/id_ed25519
   # Ou, se for chave antiga RSA:
   # ssh-add ~/.ssh/id_rsa
   ```

3. **Verifique**:
   ```bash
   ssh-add -l
   ```
   Se aparecer uma hash longa, **está funcionando!** O `devobox` detectará isso automaticamente.

---

## 2. Configurando o Podman (Linux Nativo)

Como você está no Linux, o Podman roda nativamente sem máquinas virtuais, garantindo a performance "Zero Overhead".

### Instalação
```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y podman

# Fedora
sudo dnf install -y podman

# Arch Linux
sudo pacman -S podman
```

### Ajuste de Permissões (Rootless)
Para rodar containers sem `sudo` (recomendado), verifique se os namespaces de usuário estão ativos:

```bash
# Deve retornar "user.max_user_namespaces = [número > 0]"
sysctl kernel.unprivileged_userns_clone
```

Se tiver problemas de permissão (UID mapping), rode:
```bash
# Adiciona faixas de sub-UIDs e sub-GIDs para seu usuário
sudo usermod --add-subuids 100000-165535 --add-subgids 100000-165535 $USER
podman system migrate
```

---

## 3. Variáveis de Ambiente Opcionais

Você pode adicionar ao seu `.bashrc` ou `.zshrc`:

```bash
# Define onde seus projetos ficam (Padrão: ~/code)
export DEVOBOX_CODE_DIR="$HOME/meus-projetos"
```
