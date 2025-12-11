# 👨‍🍳 Devobox Cookbook

Receitas práticas e prontas para copiar e colar. Solucione cenários comuns rapidamente.

---

## Índice

- [Receitas Básicas](#receitas-básicas)
- [Receitas de Microsserviços](#receitas-de-microsserviços)
- [Receitas de Integração](#receitas-de-integração)
- [Receitas de Workflow](#receitas-de-workflow)
- [Receitas Avançadas](#receitas-avançadas)

---

## Receitas Básicas

### Receita 1: Rails + Postgres

**Cenário:** Aplicação Rails 7 com banco PostgreSQL

**Estrutura:**
```
~/code/meu-rails-app/
├── devobox.toml
├── Gemfile
└── app/
```

**Configuração (`devobox.toml`):**
```toml
[project]
name = "meu-rails-app"
env = ["RAILS_ENV=development"]
startup_command = "bin/rails server"

[services.pg]
type = "database"
image = "postgres:16"
ports = ["5432:5432"]
env = [
    "POSTGRES_PASSWORD=dev",
    "POSTGRES_DB=rails_app_dev"
]
healthcheck_command = "pg_isready -U postgres"
healthcheck_interval = "5s"
healthcheck_timeout = "3s"
healthcheck_retries = 10
```

**Comandos:**
```bash
# Entrar no devobox
devobox -d

# Instalar Ruby via mise
mise install ruby@3.2
mise use ruby@3.2

# Setup do projeto
bundle install
bin/rails db:create db:migrate

# Rodar servidor
bin/rails server

# Acessar: http://localhost:3000
```

---

### Receita 2: Node.js + MongoDB

**Cenário:** API Node.js com MongoDB

**Estrutura:**
```
~/code/node-api/
├── devobox.toml
├── package.json
└── src/
```

**Configuração (`devobox.toml`):**
```toml
[project]
name = "node-api"
env = [
    "NODE_ENV=development",
    "MONGODB_URI=mongodb://localhost:27017/myapp"
]
startup_command = "npm run dev"

[services.mongo]
type = "database"
image = "mongo:7"
ports = ["27017:27017"]
env = ["MONGO_INITDB_DATABASE=myapp"]
healthcheck_command = "mongosh --eval 'db.runCommand({ ping: 1 })'"
healthcheck_interval = "5s"
healthcheck_timeout = "3s"
healthcheck_retries = 10
```

**Comandos:**
```bash
devobox -d

# Instalar Node
mise install node@20
mise use node@20

# Instalar dependências
npm install

# Rodar dev server
npm run dev
```

**Conexão no código:**
```javascript
// src/db.js
const { MongoClient } = require('mongodb');

const client = new MongoClient(process.env.MONGODB_URI);

async function connect() {
  await client.connect();
  console.log('✓ Conectado ao MongoDB');
  return client.db();
}
```

---

### Receita 3: Python FastAPI + Redis

**Cenário:** API Python com Redis para cache

**Estrutura:**
```
~/code/fastapi-app/
├── devobox.toml
├── requirements.txt
└── app/
    └── main.py
```

**Configuração (`devobox.toml`):**
```toml
[project]
name = "fastapi-app"
env = ["REDIS_URL=redis://localhost:6379"]
startup_command = "uvicorn app.main:app --reload --host 0.0.0.0"

[services.redis]
type = "database"
image = "redis:7-alpine"
ports = ["6379:6379"]
healthcheck_command = "redis-cli ping"
healthcheck_interval = "3s"
healthcheck_timeout = "2s"
healthcheck_retries = 5
```

**Código (`app/main.py`):**
```python
from fastapi import FastAPI
import redis
import os

app = FastAPI()
cache = redis.from_url(os.getenv('REDIS_URL'))

@app.get("/")
async def root():
    visits = cache.incr('visits')
    return {"visits": visits}

@app.get("/health")
async def health():
    return {"status": "ok"}
```

**Comandos:**
```bash
devobox -d

# Instalar Python
mise install python@3.11
mise use python@3.11

# Criar venv e instalar deps
python -m venv venv
source venv/bin/activate
pip install -r requirements.txt

# Rodar servidor
uvicorn app.main:app --reload --host 0.0.0.0
```

---

## Receitas de Microsserviços

### Receita 4: Frontend (Vue) + Backend (Rails)

**Cenário:** Frontend Vue consome API Rails

**Estrutura:**
```
~/code/
├── frontend/
│   ├── devobox.toml
│   └── package.json
└── backend/
    ├── devobox.toml
    └── Gemfile
```

**Backend (`~/code/backend/devobox.toml`):**
```toml
[project]
name = "backend"
startup_command = "bin/rails server -p 3001 -b 0.0.0.0"

[services.backend-db]
type = "database"
image = "postgres:16"
ports = ["5433:5432"]
env = [
    "POSTGRES_PASSWORD=dev",
    "POSTGRES_DB=backend_dev"
]
healthcheck_command = "pg_isready -U postgres"
healthcheck_interval = "5s"
```

**Frontend (`~/code/frontend/devobox.toml`):**
```toml
[project]
name = "frontend"
env = ["VITE_API_URL=http://localhost:3001"]
startup_command = "npm run dev"

[dependencies]
include_projects = ["../backend"]

[services.frontend-cache]
image = "redis:7"
ports = ["6380:6379"]
```

**Uso:**
```bash
# Dentro do devobox
devobox project up frontend
```

**O que acontece:**
```
Zellij: devobox-frontend
┌─ Aba 1: frontend ─────────┐
│ $ npm run dev             │
│ ✓ http://localhost:5173   │
├─ Aba 2: backend ──────────┤
│ $ bin/rails server -p 3001│
│ ✓ http://localhost:3001   │
└───────────────────────────┘

Serviços rodando:
🗄️ backend-db (Postgres) :5433
🔴 frontend-cache (Redis) :6380
```

**Código frontend (`src/api.js`):**
```javascript
const API_URL = import.meta.env.VITE_API_URL;

export async function fetchUsers() {
  const response = await fetch(`${API_URL}/api/users`);
  return response.json();
}
```

---

### Receita 5: Arquitetura de 3 Camadas (Auth + API + Frontend)

**Estrutura:**
```
~/code/
├── frontend/
│   └── devobox.toml
├── api/
│   └── devobox.toml
└── auth-service/
    └── devobox.toml
```

**Auth Service (`~/code/auth-service/devobox.toml`):**
```toml
[project]
name = "auth-service"
startup_command = "node server.js"

[services.auth-db]
type = "database"
image = "postgres:16"
ports = ["5434:5432"]
env = ["POSTGRES_PASSWORD=dev", "POSTGRES_DB=auth"]
healthcheck_command = "pg_isready -U postgres"
```

**API (`~/code/api/devobox.toml`):**
```toml
[project]
name = "api"
env = ["AUTH_SERVICE_URL=http://localhost:3002"]
startup_command = "cargo run"

[dependencies]
include_projects = ["../auth-service"]

[services.api-db]
type = "database"
image = "postgres:16"
ports = ["5435:5432"]
env = ["POSTGRES_PASSWORD=dev", "POSTGRES_DB=api"]
healthcheck_command = "pg_isready -U postgres"
```

**Frontend (`~/code/frontend/devobox.toml`):**
```toml
[project]
name = "frontend"
env = [
    "VITE_API_URL=http://localhost:3003",
    "VITE_AUTH_URL=http://localhost:3002"
]
startup_command = "npm run dev"

[dependencies]
include_projects = ["../api", "../auth-service"]
```

**Ativação:**
```bash
devobox project up frontend
```

**Resultado:**
- 3 abas no Zellij (frontend, api, auth-service)
- 2 bancos Postgres (auth-db, api-db)
- Tudo orquestrado automaticamente

---

## Receitas de Integração

### Receita 6: Mailhog para Testes de Email

**Cenário:** Testar envio de emails sem servidor SMTP real

**Configuração (`devobox.toml`):**
```toml
[services.mailhog]
image = "mailhog/mailhog:latest"
ports = [
    "1025:1025",   # SMTP
    "8025:8025"    # Web UI
]
```

**Configuração da aplicação:**

**Rails (`config/environments/development.rb`):**
```ruby
config.action_mailer.delivery_method = :smtp
config.action_mailer.smtp_settings = {
  address: 'localhost',
  port: 1025
}
```

**Node.js (Nodemailer):**
```javascript
const nodemailer = require('nodemailer');

const transporter = nodemailer.createTransport({
  host: 'localhost',
  port: 1025,
  secure: false
});

await transporter.sendMail({
  from: 'app@example.com',
  to: 'user@example.com',
  subject: 'Teste',
  text: 'Email de teste via Mailhog'
});
```

**Uso:**
```bash
devobox service start mailhog

# Enviar email via app

# Acessar Web UI: http://localhost:8025
# ✓ Ver todos os emails capturados
```

---

### Receita 7: Múltiplos Bancos de Dados

**Cenário:** App que usa Postgres (dados) + Redis (cache) + MongoDB (logs)

**Configuração (`devobox.toml`):**
```toml
[services.pg]
type = "database"
image = "postgres:16"
ports = ["5432:5432"]
env = [
    "POSTGRES_PASSWORD=dev",
    "POSTGRES_DB=myapp"
]
healthcheck_command = "pg_isready -U postgres"
healthcheck_interval = "5s"

[services.redis]
type = "database"
image = "redis:7-alpine"
ports = ["6379:6379"]
healthcheck_command = "redis-cli ping"
healthcheck_interval = "3s"

[services.mongo]
type = "database"
image = "mongo:7"
ports = ["27017:27017"]
healthcheck_command = "mongosh --eval 'db.runCommand({ ping: 1 })'"
healthcheck_interval = "5s"
```

**Variáveis de ambiente:**
```toml
[project]
env = [
    "DATABASE_URL=postgresql://postgres:dev@localhost:5432/myapp",
    "REDIS_URL=redis://localhost:6379",
    "MONGODB_URI=mongodb://localhost:27017/logs"
]
```

**Comandos:**
```bash
# Iniciar todos os bancos
devobox db start

# Ver status
devobox db status

# Parar todos
devobox db stop
```

---

## Receitas de Workflow

### Receita 8: Workflow Git com SSH

**Configuração inicial:**
```bash
# No host (fora do container)
eval "$(ssh-agent -s)"
ssh-add ~/.ssh/id_ed25519

# Verificar
ssh-add -l
```

**Uso dentro do container:**
```bash
devobox

# Clonar repositório privado
git clone git@github.com:usuario/repo-privado.git

# Configurar git
git config --global user.name "Seu Nome"
git config --global user.email "seu@email.com"

# Workflow normal
git add .
git commit -m "feat: nova funcionalidade"
git push

# ✓ Usa sua chave SSH do host automaticamente
```

**Troubleshooting:**
```bash
# Se der erro de permissão
ssh-add -l  # Verifica se chave está carregada

# Se não aparecer
exit  # Sai do container
eval "$(ssh-agent -s)"
ssh-add ~/.ssh/id_ed25519
devobox  # Entra novamente
```

---

### Receita 9: Alternando entre Projetos

**Cenário:** Trabalhar em múltiplos projetos simultaneamente

**Setup:**
```
~/code/
├── projeto-a/devobox.toml
├── projeto-b/devobox.toml
└── projeto-c/devobox.toml
```

**Workflow:**
```bash
# Listar projetos
devobox project list

# Ativar projeto A
devobox project up projeto-a
npm run dev
# (servidor rodando...)

# Detach (mantém rodando)
Ctrl + o, d

# Ativar projeto B
devobox project up projeto-b
cargo run
# (servidor rodando...)

# Detach
Ctrl + o, d

# Ativar projeto C
devobox project up projeto-c
rails server

# Voltar ao projeto A
Ctrl + o, d
devobox project up projeto-a
# ✓ npm run dev ainda rodando!
```

**Gerenciamento:**
```bash
# Ver contexto atual
devobox project info

# Listar sessões Zellij
zellij list-sessions

# Anexar a sessão específica
zellij attach devobox-projeto-a
```

---

### Receita 10: Gerenciamento de Serviços em Background

**Cenário:** Controlar quando cada serviço sobe/desce

**Configuração (`devobox.toml`):**
```toml
[services.pg]
type = "database"
image = "postgres:16"
ports = ["5432:5432"]

[services.redis]
type = "database"
image = "redis:7"
ports = ["6379:6379"]

[services.mailhog]
type = "generic"
image = "mailhog/mailhog"
ports = ["1025:1025", "8025:8025"]

[services.minio]
type = "generic"
image = "minio/minio"
ports = ["9000:9000", "9001:9001"]
```

**Controle granular:**
```bash
# Apenas bancos pesados
devobox db start
# ✓ pg, redis

# Apenas serviços leves
devobox service start
# ✓ mailhog, minio

# Serviço específico
devobox db start pg
devobox service start mailhog

# Ver o que está rodando
devobox status

# Parar tudo
devobox down

# Modo econômico (auto-stop ao sair)
devobox -d --auto-stop
# ... trabalha ...
exit
# ✓ Todos os containers param
```

---

## Receitas Avançadas

### Receita 11: Imagem Base Customizada

**Cenário:** Precisa de pacotes adicionais (imagemagick, ffmpeg, etc.)

**Criar Containerfile customizado:**
```dockerfile
# ~/code/meu-projeto/Containerfile.custom
FROM devobox-img:latest

USER root

# Adicionar repositórios ou pacotes
RUN apt-get update && apt-get install -y \
    imagemagick \
    ffmpeg \
    libvips-dev \
    && rm -rf /var/lib/apt/lists/*

USER dev

# Ferramentas adicionais via mise (opcional)
RUN mise install golang@1.21
```

**Configuração (`devobox.toml`):**
```toml
[paths]
containerfile = "Containerfile.custom"

[build]
image_name = "devobox-img-custom"
```

**Build:**
```bash
# No host
devobox rebuild

# Agora seu container tem imagemagick e ffmpeg
devobox
which convert
# /usr/bin/convert
```

---

### Receita 12: Redis Compartilhado entre Projetos

**Cenário:** Evitar múltiplas instâncias de Redis

**Global (`~/.config/devobox/devobox.toml`):**
```toml
[services.shared-redis]
type = "database"
image = "redis:7-alpine"
ports = ["6379:6379"]
healthcheck_command = "redis-cli ping"
```

**Projeto A (`~/code/projeto-a/devobox.toml`):**
```toml
[project]
env = ["REDIS_URL=redis://localhost:6379/0"]  # DB 0
```

**Projeto B (`~/code/projeto-b/devobox.toml`):**
```toml
[project]
env = ["REDIS_URL=redis://localhost:6379/1"]  # DB 1
```

**Resultado:**
- 1 Redis rodando
- 2 projetos usam DBs separados (0 e 1)
- Economia de RAM

**Comandos:**
```bash
# Redis já sobe com devobox -d (global config)
devobox -d

# Verificar
redis-cli
> SELECT 0
> KEYS *  # Chaves do projeto A

> SELECT 1
> KEYS *  # Chaves do projeto B
```

---

### Receita 13: Integração CI/CD

**Cenário:** Usar Devobox em pipelines CI/CD

**GitHub Actions (`.github/workflows/test.yml`):**
```yaml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Install Podman
        run: |
          sudo apt-get update
          sudo apt-get install -y podman

      - name: Download Devobox
        run: |
          curl -L https://github.com/JohnAnon9771/devobox/releases/latest/download/devobox-linux-x86_64 \
            -o /usr/local/bin/devobox
          chmod +x /usr/local/bin/devobox

      - name: Initialize Devobox
        run: devobox init

      - name: Run Tests
        run: |
          devobox shell << 'EOF'
          mise install
          bundle install
          bin/rails test
          EOF
```

**GitLab CI (`.gitlab-ci.yml`):**
```yaml
stages:
  - test

test:
  image: ubuntu:22.04

  before_script:
    - apt-get update
    - apt-get install -y podman curl
    - curl -L https://github.com/JohnAnon9771/devobox/releases/latest/download/devobox-linux-x86_64 -o /usr/local/bin/devobox
    - chmod +x /usr/local/bin/devobox
    - devobox init

  script:
    - devobox shell -c "mise install && npm test"
```

---

## Dicas Gerais

### Debugging de Serviços

```bash
# Ver logs de um serviço
podman logs pg -f
podman logs backend-api --tail=50

# Entrar em um container de serviço
podman exec -it pg bash
# Agora você está dentro do Postgres container
```

### Limpeza Regular

```bash
# Semanal
devobox cleanup

# Mensal
devobox cleanup --images
devobox cleanup --volumes
```

### Performance

```bash
# Desabilitar CoW (Btrfs)
sudo chattr +C ~/.local/share/containers/storage

# Ver uso de recursos
podman stats
```

---

**Não encontrou sua receita?** Abra uma issue: https://github.com/JohnAnon9771/devobox/issues

**Quer contribuir com uma receita?** PRs são bem-vindos!
