# 🚀 Guia de Início Rápido

**De zero a produtivo em 15 minutos.**

Este guia leva você desde a instalação até criar seu primeiro projeto com banco de dados, tudo funcionando perfeitamente.

---

## ✅ Pré-requisitos

Antes de começar, verifique que você tem:

### 1. Podman Instalado

```bash
podman --version
# Deve mostrar: podman version 4.0.0 ou superior
```

**Não tem Podman?** Instale agora:

```bash
# Ubuntu/Debian
sudo apt-get update && sudo apt-get install -y podman

# Fedora
sudo dnf install -y podman

# Arch Linux
sudo pacman -S podman
```

### 2. PATH Configurado

```bash
echo $PATH | grep -q "$HOME/.local/bin" && echo "✅ PATH ok" || echo "❌ Adicione ao PATH"
```

**Se mostrar "❌"**, adicione ao seu `~/.bashrc` ou `~/.zshrc`:

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### 3. SSH Agent (Para Git)

```bash
ssh-add -l
# Se mostrar uma chave, está OK!
```

**Se der erro**, configure o SSH agent:

```bash
# 1. Inicie o agent
eval "$(ssh-agent -s)"

# 2. Adicione sua chave
ssh-add ~/.ssh/id_ed25519  # ou id_rsa

# 3. Verifique
ssh-add -l
```

**Não tem chaves SSH?** Gere uma:

```bash
ssh-keygen -t ed25519 -C "seu-email@exemplo.com"
# Pressione Enter para aceitar defaults
# Depois adicione a chave pública ao GitHub/GitLab
```

---

## 📦 Instalação

### Passo 1: Baixar o Devobox

```bash
curl -L https://github.com/JohnAnon9771/devobox/releases/latest/download/x86_64-unknown-linux-gnu.tar.gz -o devobox.tar.gz
tar -xzf devobox.tar.gz
chmod +x devobox
mv devobox ~/.local/bin/devobox
rm devobox.tar.gz
```

### Passo 2: Verificar Instalação

```bash
devobox --version
# Deve mostrar a versão instalada
```

### Passo 3: Setup Inicial

```bash
devobox init
```

**O que está acontecendo?**

Durante os próximos 3-5 minutos, o Devobox está:
1. ✓ Criando diretório de config em `~/.config/devobox`
2. ✓ Baixando imagem base Debian Bookworm
3. ✓ Instalando ferramentas (Neovim, Zellij, Mise, Git)
4. ✓ Construindo seu container principal
5. ✓ Preparando containers de serviço

**Enquanto espera:** Tome um café! ☕

### Passo 4: Entrar no Ambiente

```bash
devobox
```

**Pronto!** Você está dentro do seu ambiente Devobox.

**O que mudou?**
- ✓ Seu prompt é diferente (Starship)
- ✓ Você está em um Zellij session
- ✓ Seu diretório `~/code` está montado e acessível

---

## 🎯 Seu Primeiro Projeto

Vamos criar um projeto Node.js simples para você ver a mágica acontecer.

### Passo 1: Criar Diretório

```bash
# Você está dentro do devobox agora
cd ~/code
mkdir hello-devobox
cd hello-devobox
```

### Passo 2: Instalar Node.js

```bash
# Use o Mise para instalar Node 20
mise install node@20
mise use node@20

# Verifique
node --version
# Deve mostrar: v20.x.x
```

**O que aconteceu?**
- O Node foi instalado **apenas dentro do container**
- Seu sistema host não foi tocado
- A instalação é super rápida (Mise usa cache)

### Passo 3: Criar uma Aplicação Simples

```bash
# Criar um package.json
npm init -y

# Criar um arquivo de app
cat > app.js << 'EOF'
const http = require('http');

const server = http.createServer((req, res) => {
  res.writeHead(200, { 'Content-Type': 'text/plain' });
  res.end('Hello from Devobox! 🚀\n');
});

server.listen(3000, () => {
  console.log('Server running at http://localhost:3000/');
});
EOF

# Rodar a aplicação
node app.js
```

Abra seu navegador (no seu host!) e acesse: **http://localhost:3000**

**Viu?** O container usa `--network host`, então `localhost:3000` funciona direto!

**Para parar:** `Ctrl + C`

---

## 🗄️ Adicionando um Banco de Dados

Agora vamos adicionar um Postgres ao seu projeto.

### Passo 1: Criar Arquivo de Configuração

Crie um arquivo `devobox.toml` no seu projeto:

```bash
cat > devobox.toml << 'EOF'
[project]
name = "hello-devobox"
env = ["NODE_ENV=development"]

[services.pg]
type = "database"
image = "postgres:16"
ports = ["5432:5432"]
env = [
    "POSTGRES_PASSWORD=dev",
    "POSTGRES_DB=hello"
]
healthcheck_command = "pg_isready -U postgres"
healthcheck_interval = "5s"
healthcheck_timeout = "3s"
healthcheck_retries = 5
EOF
```

**O que esse arquivo faz?**
- Define um serviço chamado `pg` (Postgres)
- Marca como `type: database` (controlado via `devobox db`)
- Configura healthcheck (Devobox espera até estar pronto)
- Define porta 5432 e variáveis de ambiente

### Passo 2: Iniciar o Banco

```bash
# Sair do shell atual (mas vai continuar dentro do container)
exit

# Entrar novamente, mas agora iniciando serviços
cd ~/code/hello-devobox
devobox -d
```

**O que está acontecendo?**

```
🚀 Iniciando todos os serviços...
  🔌 Iniciando pg...
💖 Verificando healthchecks...
  🩺 Aguardando pg ficar saudável... ✅ Saudável!
🚀 Entrando no devobox...
```

O Devobox:
1. ✓ Detectou o `devobox.toml`
2. ✓ Criou o container do Postgres
3. ✓ Esperou até ele estar realmente pronto (healthcheck)
4. ✓ Abriu o shell

**Nada de "Connection Refused"!**

### Passo 3: Testar a Conexão

```bash
# Instalar o cliente postgres
sudo apt-get update && sudo apt-get install -y postgresql-client

# Conectar no banco
psql -h localhost -U postgres -d hello
# Senha: dev

# Testar
postgres=# SELECT version();
postgres=# \q
```

**Funcionou!** Seu banco está rodando em um container separado, mas acessível via localhost.

---

## 🎹 Entendendo o Zellij

Você está dentro do **Zellij**, um multiplexador de terminal. Pense nele como "abas para o terminal".

### Por que Zellij?

1. **Persistência:** Se você fechar o terminal do host, seus processos continuam rodando
2. **Organização:** Múltiplas abas/painéis sem abrir várias janelas
3. **Detach:** Saia sem matar processos, volte depois exatamente onde parou

### Atalhos Essenciais

O Zellij usa `Alt` como tecla modificadora:

| Ação | Atalho |
|------|--------|
| **Nova aba** | `Alt + t` |
| **Próxima aba** | `Alt + n` |
| **Aba anterior** | `Alt + p` |
| **Novo painel** (split) | `Alt + n` |
| **Navegar painéis** | `Alt + Setas` |
| **Scroll mode** | `Ctrl + s` (setas para scroll, `Esc` para sair) |
| **Detach** (sair mantendo rodando) | `Ctrl + o`, depois `d` |

**Dica:** Olhe a barra verde no rodapé — mostra os atalhos disponíveis!

### Exercício Prático

1. **Crie uma nova aba:** `Alt + t`
2. **Rode um servidor:** `node app.js`
3. **Volte para a aba anterior:** `Alt + p`
4. **Seu servidor continua rodando!**
5. **Detach:** `Ctrl + o`, depois `d`

Você voltou ao host. Mas o servidor continua rodando!

Para voltar:

```bash
devobox
```

**Magia!** Você reconectou à mesma sessão. Tudo como você deixou.

---

## 📝 Cheatsheet de Comandos

### Entrar/Sair

```bash
devobox              # Entrar no ambiente
devobox -d           # Entrar COM todos serviços iniciados
Ctrl + o, depois d   # Detach (sair mantendo tudo rodando)
exit                 # Sair e parar shell (serviços continuam)
```

### Gerenciar Serviços

```bash
# Bancos de dados
devobox db start     # Iniciar todos bancos
devobox db start pg  # Iniciar apenas Postgres
devobox db stop      # Parar bancos

# Serviços genéricos
devobox service start
devobox service stop

# Ver status
devobox status
```

### Projetos

```bash
devobox project list       # Listar projetos em ~/code
devobox project up myapp   # Ativar workspace do projeto
devobox project info       # Ver contexto atual
```

### Manutenção

```bash
devobox rebuild      # Reconstruir ambiente
devobox cleanup      # Limpar recursos não usados
```

---

## 🐛 Troubleshooting Rápido

### Problema: Container não inicia

```bash
# Ver logs
podman logs devobox

# Reconstruir
devobox rebuild
```

### Problema: Performance lenta (Btrfs/ZFS)

```bash
# Desabilitar Copy-on-Write
sudo chattr +C ~/.local/share/containers/storage
```

### Problema: "Permission denied" em arquivos

O Devobox usa `--userns=keep-id` para mapear seu UID. Se tiver problemas:

```bash
# Configurar sub-UIDs e sub-GIDs
sudo usermod --add-subuids 100000-165535 --add-subgids 100000-165535 $USER
podman system migrate
```

### Problema: SSH não funciona dentro do container

```bash
# No host, verifique se o agent está rodando
ssh-add -l

# Se não mostrar chaves
eval "$(ssh-agent -s)"
ssh-add ~/.ssh/id_ed25519
```

### Problema: Porta já em uso

Se você já tem Postgres rodando no host:

```toml
# Mude a porta no devobox.toml
[services.pg]
ports = ["5433:5432"]  # Host:Container
```

Agora conecte via `psql -h localhost -p 5433`

---

## 🎓 Próximos Passos

**Parabéns!** Você tem um ambiente Devobox funcionando com:
- ✅ Container isolado
- ✅ Node.js instalado
- ✅ Postgres rodando
- ✅ Zellij configurado

### Agora você pode:

**Aprender conceitos mais profundos:**
➡️ [Guia Completo](docs/GUIDE.md) - Entenda Hub & Spoke, workflows e comparações

**Ver exemplos práticos:**
➡️ [Cookbook](docs/COOKBOOK.md) - Receitas para Rails, Django, microserviços, etc.

**Entender a arquitetura:**
➡️ [Documentação de Arquitetura](docs/ARCHITECTURE.md) - Para contribuidores

---

## 💡 Dicas Finais

### Use `devobox project up` para Projetos Reais

Quando você tem um projeto com múltiplos serviços:

```bash
cd ~/code/meu-projeto
devobox project up meu-projeto
```

Isso:
- Inicia todos serviços do projeto
- Cria uma sessão Zellij dedicada
- Roda o `startup_command` automaticamente
- Isola completamente de outros projetos

### Sempre Use Detach

**Não faça:** Fechar o terminal e matar processos

**Faça:** `Ctrl + o`, depois `d` para detach

Isso mantém:
- Servidores rodando
- Builds em andamento
- Sessões preservadas

### Aprenda Zellij

Invista 10 minutos aprendendo Zellij. Vai multiplicar sua produtividade.

Principais atalhos:
- `Alt + t` - Nova aba
- `Alt + n` - Split horizontal
- `Ctrl + o, d` - Detach

---

**Agora você está pronto para desenvolver sem medo!** 🚀

Tem dúvidas? Abra uma issue: https://github.com/JohnAnon9771/devobox/issues
