# 📖 Guia Completo do Devobox

Compreensão profunda de conceitos, workflows e melhores práticas.

---

## Índice

1. [Conceitos Fundamentais](#parte-1-conceitos-fundamentais)
2. [Sistema de Configuração](#parte-2-sistema-de-configuração)
3. [Workflows](#parte-3-workflows)
4. [Comparações Detalhadas](#parte-4-comparações-detalhadas)
5. [Tópicos Avançados](#parte-5-tópicos-avançados)

---

## Parte 1: Conceitos Fundamentais

### Os 4 Pilares em Profundidade

#### Pilar 1: Higiene Absoluta do Host

**O problema raiz:**

Em distribuições rolling release (Arch, Manjaro) ou com atualizações frequentes (Ubuntu com PPAs), as bibliotecas do sistema evoluem constantemente:

- OpenSSL atualiza de 1.1 para 3.0
- glibc atualiza quebrando ABIs
- libicu muda versões

Se você instala runtimes de linguagem (Ruby, Node, Python) **direto no host**, elas compilam contra essas bibliotecas. Quando as bibliotecas mudam, as runtimes quebram.

**Cenário real:**

```bash
# Segunda-feira
$ ruby -v
ruby 3.1.0

# Atualização do sistema
$ sudo pacman -Syu
# openssl: 1.1.1 → 3.0.0

# Tenta usar o Ruby
$ bundle install
# ERRO: OpenSSL não encontrado
# Gems nativas não compilam
```

**A solução Devobox:**

Isolar as runtimes dentro de um container baseado em **Debian Stable**:

- Debian Bookworm congela versões de bibliotecas por ~2 anos
- Seu host pode atualizar livremente
- Container permanece estável

**Benefícios:**

- Zero conflitos entre projetos
- Atualizações do sistema não quebram desenvolvimento
- Rollback simples: `devobox rebuild`

---

#### Pilar 2: Performance Nativa

**O problema de virtualização:**

Docker Desktop (Mac/Windows) e muitas VMs usam camadas de virtualização que degradam I/O:

- Volumes compartilhados passam por drivers de rede virtual
- `npm install` pode levar 5x mais tempo
- Filesystem watches (Webpack, Vite) ficam lentos

**A solução Devobox:**

Aproveitar a arquitetura nativa do Linux:

**Bind Mounts:**

```bash
# Container vê o mesmo inode do arquivo no host
# Leitura/escrita acontece direto no kernel
# Zero overhead
```

**Host Networking:**

```bash
# Container não tem IP próprio
# Usa a mesma pilha de rede do host
# localhost:3000 é literalmente localhost:3000
# Sem NAT, sem bridge, sem overhead
```

**Benchmark real:**
| Operação | Docker Desktop (Mac) | Devobox (Linux) |
|----------|---------------------|----------------|
| `npm install` (50 deps) | 45s | 8s |
| `cargo build` | 2m30s | 45s |
| File watch latency | ~500ms | ~10ms |

---

#### Pilar 3: Filosofia "Pet" vs "Cattle"

**Cattle (gado) — Containers tradicionais:**

```bash
docker-compose up
# Trabalha...
docker-compose down
# TUDO PERDIDO:
# - Histórico bash (Ctrl+R)
# - Ferramentas instaladas (pry, debugger)
# - Estado do terminal
```

A cada `up`, você começa do zero. Containers são descartáveis.

**Pet (animal de estimação) — Devobox:**

```bash
devobox
# Trabalha...
# Detach: Ctrl+o, d
# TUDO PRESERVADO via Zellij:
# - Histórico shell
# - Processos rodando
# - Sessões abertas
```

O container é **persistente**. Você o trata como um segundo computador.

**Como funciona:**

1. **Imutabilidade declarativa:** Ferramentas definidas em `mise.toml`
2. **Persistência via Zellij:** Terminal multiplexor mantém sessões
3. **Singleton pattern:** Um container Hub reutilizado

**Quando usar cada abordagem:**

| Cenário               | Abordagem            |
| --------------------- | -------------------- |
| CI/CD, Produção       | Cattle (descartável) |
| Desenvolvimento local | Pet (persistente)    |

---

#### Pilar 4: Orquestração Inteligente

**Problemas comuns de orquestração:**

1. **Race conditions:** App sobe antes do banco estar pronto

   ```bash
   rails server
   # ERRO: Connection refused (PostgreSQL ainda iniciando)
   ```

2. **Desperdício de recursos:** 3 projetos, 3 instâncias do Postgres

3. **Healthchecks passivos:** Docker Compose marca como "healthy" se o processo existe, não se está respondendo

**A solução Devobox:**

**Healthchecks ativos:**

```toml
[services.pg]
healthcheck_command = "pg_isready -U postgres"
healthcheck_interval = "5s"
healthcheck_retries = 10
```

Devobox:

1. Inicia o container
2. Executa `pg_isready` a cada 5s
3. Só libera seu shell quando retornar sucesso
4. Se falhar 10 vezes, reporta erro

**Separação de concerns:**

- **Database** (`type: database`): Infraestrutura pesada (Postgres, MySQL)
- **Generic**: Serviços auxiliares (Redis, Mailhog)

Controle granular:

```bash
devobox db start        # Apenas bancos
devobox service start   # Apenas auxiliares
```

**Dependências entre projetos:**

```toml
[dependencies]
include_projects = ["../backend-api", "../auth-service"]
```

Devobox resolve recursivamente e sobe tudo na ordem certa.

---

### Hub & Spoke: Arquitetura Detalhada

O Devobox usa um padrão inspirado em redes: **Hub & Spoke** (cubo e raios).

#### Hub (Container Central)

**Características:**

- Nome: `devobox` (singleton)
- Network: `--network host`
- Persistente (sobrevive a reinícios)
- User namespace: `--userns=keep-id`

**Por que host network?**

```
Tradicional (Bridge):
App → Bridge → NAT → Host → Internet
      (overhead)

Devobox (Host):
App → Host → Internet
     (zero overhead)
```

Benefícios:

- Localhost funciona naturalmente
- Sem mapeamento de portas
- Performance de rede nativa

**Trade-off:**

- Menos isolamento de rede
- Porta em uso no host = porta em uso no container

**Decisão:** Para desenvolvimento local, performance > isolamento.

#### Spokes (Containers Satélites)

**Características:**

- Network: `bridge` (padrão Podman)
- Isolados do Hub
- Porta-mapped explicitamente

**Por que bridge?**

```
┌─────────────┐
│     Hub     │  ← Host network (0.0.0.0)
└──────┬──────┘
       │
   ┌───┴────┬────────┐
   │        │        │
 ┌─▼─┐   ┌─▼─┐   ┌─▼─┐
 │ PG │   │ R │   │MH │  ← Bridge network (isolados)
 └───┘   └───┘   └───┘
   ↑       ↑       ↑
 :5432   :6379   :8025  (port mapping)
```

Benefícios:

- Bancos isolados do código (segurança)
- Port mapping explícito (evita conflitos)
- Múltiplas instâncias possíveis (frontend-db, backend-db)

---

### Pet vs Cattle: Quando Usar Cada Um

#### Cattle (Efêmero)

**Use quando:**

- CI/CD pipelines
- Ambientes de teste automatizado
- Deploy de produção
- Não precisa de estado

**Exemplo:**

```bash
# GitHub Actions
docker run --rm myapp npm test
# Container morre após teste
```

#### Pet (Persistente)

**Use quando:**

- Desenvolvimento local
- Debugging interativo
- Precisa de histórico/estado
- Ferramentas instaladas manualmente

**Exemplo:**

```bash
# Devobox
devobox
gem install pry  # Instala debugger
# Amanhã: pry ainda está lá
```

**Hybrid approach (Devobox):**

- Configuração declarativa (cattle-like) via `mise.toml`
- Persistência de sessão (pet-like) via Zellij
- Melhor dos dois mundos

---

## Parte 2: Sistema de Configuração

### Cascata: Global → Local → Projeto

Devobox resolve configuração em 3 níveis:

```
1. Defaults (hardcoded)
   ↓
2. Global (~/.config/devobox/devobox.toml)
   ↓
3. Local (./devobox.toml)
   ↓
Final Config
```

**Exemplo:**

```toml
# Global: ~/.config/devobox/devobox.toml
[services.pg]
type = "database"
image = "postgres:16"
ports = ["5432:5432"]
```

```toml
# Local: ~/code/meu-app/devobox.toml
[services.pg]
env = ["POSTGRES_DB=myapp"]  # Override/adiciona
```

**Resultado:**

```toml
# Merged
[services.pg]
type = "database"
image = "postgres:16"
ports = ["5432:5432"]
env = ["POSTGRES_DB=myapp"]
```

---

### Formato TOML: Services

#### Estrutura Básica

```toml
[services.NOME]
type = "database" | "generic"  # Opcional (default: generic)
image = "docker.io/postgres:16"
ports = ["HOST:CONTAINER"]
env = ["KEY=VALUE"]
volumes = ["HOST:CONTAINER"]
healthcheck_command = "comando"
healthcheck_interval = "5s"
healthcheck_timeout = "3s"
healthcheck_retries = 5
```

#### Exemplo Completo

```toml
[services.pg]
type = "database"
image = "postgres:16"
ports = ["5432:5432"]
env = [
    "POSTGRES_PASSWORD=dev",
    "POSTGRES_DB=myapp",
    "POSTGRES_USER=dev"
]
volumes = ["/data/pg:/var/lib/postgresql/data"]
healthcheck_command = "pg_isready -U dev"
healthcheck_interval = "5s"
healthcheck_timeout = "3s"
healthcheck_retries = 10
```

---

### Database vs Generic: Por que a Distinção?

#### Database Services

**Características:**

- Infraestrutura pesada
- Dados persistentes críticos
- Inicialização mais lenta
- Uso de memória significativo

**Exemplos:** Postgres, MySQL, MongoDB

**Controle:**

```bash
devobox db start
devobox db stop
devobox db restart pg
```

#### Generic Services

**Características:**

- Serviços auxiliares
- Geralmente leves
- Podem ser efêmeros
- Menos críticos

**Exemplos:** Redis (cache), Mailhog, Mocks

**Controle:**

```bash
devobox service start
devobox service stop
devobox service restart mailhog
```

**Benefício:**

Você pode fazer:

```bash
devobox up --dbs-only
# Sobe apenas Postgres, MySQL
# Economiza RAM não subindo Redis, Mailhog
```

---

### Healthchecks: Como Funcionam

**Fluxo de inicialização:**

```
1. Container criado
   ↓
2. Container iniciado
   ↓
3. Processo principal rodando
   ↓
4. ⏱️ HEALTHCHECK (aqui está a diferença)
   ↓
5. Shell liberado
```

**Configuração:**

```toml
healthcheck_command = "pg_isready -U postgres"
healthcheck_interval = "5s"    # Espera entre tentativas
healthcheck_timeout = "3s"     # Timeout por tentativa
healthcheck_retries = 10       # Máximo de tentativas
```

**Algoritmo:**

```rust
for attempt in 1..=10 {
    result = exec("pg_isready -U postgres")
    if result.success() {
        return Healthy
    }
    sleep(5s)
}
return Unhealthy
```

**Por que isso importa:**

```bash
# SEM healthcheck (Docker Compose tradicional)
docker-compose up
rails server
# ERRO: Connection refused

# COM healthcheck (Devobox)
devobox up
# 🩺 Aguardando pg... ✅ Saudável!
rails server
# SUCCESS: Conectado ao banco
```

---

### Dependências entre Projetos

Projetos podem importar serviços de outros projetos:

```toml
# ~/code/frontend/devobox.toml
[dependencies]
include_projects = [
    "../backend-api",
    "../auth-service"
]
```

**Resolução:**

```
1. Carrega ~/code/frontend/devobox.toml
2. Para cada include_project:
   a. Carrega ../backend-api/devobox.toml
   b. Extrai [services.*]
   c. Carrega ../auth-service/devobox.toml
   d. Extrai [services.*]
3. Merge todos os serviços
4. Valida duplicatas
5. Inicia em ordem topológica
```

**Prevenção de ciclos:**

```
frontend → backend → auth → backend
                     ❌ ERRO: Ciclo detectado!
```

---

## Parte 3: Workflows

### Workflow Diário

#### Manhã: Conectar

```bash
# No host
devobox
```

**O que acontece:**

1. Verifica se container `devobox` existe
   - Se não: cria e inicia
   - Se parado: inicia
   - Se rodando: apenas conecta
2. Verifica se sessão Zellij `devobox` existe
   - Se não: cria nova
   - Se existe: anexa
3. Abre shell

**Resultado:** Você está exatamente onde parou ontem.

---

#### Durante o Dia: Multitarefa

Use Zellij para organizar:

```
Aba 1: Editor         Alt + t → Nova aba
┌────────────────┐    ┌────────────────┐
│ nvim .         │    │                │
│                │    │                │
│                │    │                │
│                │    │                │
└────────────────┘    └────────────────┘

Aba 2: Servidor       Alt + t → Nova aba
┌────────────────┐    ┌────────────────┐
│ npm run dev    │    │ Alt + n → Split│
│ (rodando...)   │    │ ┌─────┬───────┐│
│                │    │ │ git │ logs  ││
│                │    │ └─────┴───────┘│
└────────────────┘    └────────────────┘
```

**Comandos:**

- `Alt + t` — Nova aba
- `Alt + n` — Split painel
- `Alt + setas` — Navegar
- `Ctrl + s` — Scroll mode

---

#### Noite: Detach

**NÃO faça:**

```bash
exit  # Mata a sessão
```

**FAÇA:**

```bash
Ctrl + o, d  # Detach
```

**Diferença:**

| Ação       | Resultado                     |
| ---------- | ----------------------------- |
| `exit`     | Fecha Zellij, mata processos  |
| `Ctrl+o,d` | Desanexa, processos continuam |

**Amanhã:**

```bash
devobox
# ✅ Servidor ainda rodando
# ✅ Builds em progresso
# ✅ Histórico intacto
```

---

### Workflow Multi-Projeto

#### Estrutura

```
~/code/
├── frontend/
│   └── devobox.toml
├── backend/
│   └── devobox.toml
└── auth-service/
    └── devobox.toml
```

#### Descoberta

```bash
devobox project list
```

**Output:**

```
📁 Projetos disponíveis em ~/code:
  - frontend
  - backend
  - auth-service
```

#### Ativação

```bash
devobox project up frontend
```

**O que acontece:**

1. Lê `~/code/frontend/devobox.toml`
2. Resolve `include_projects` (se houver)
3. Inicia todos os serviços necessários
4. Cria sessão Zellij `devobox-frontend`
5. Carrega env vars do projeto
6. Muda para `~/code/frontend`
7. Executa `startup_command` (se definido)

#### Alternância

```bash
# Trabalhando no frontend
devobox project up frontend
npm run dev

# Detach
Ctrl + o, d

# Alternar para backend
devobox project up backend
rails server

# Detach
Ctrl + o, d

# Voltar ao frontend
devobox project up frontend
# ✅ npm run dev ainda rodando!
```

**Benefício:** Sessões paralelas. Servidores não param.

---

### Workflow de Microsserviços

#### Padrão "App as a Service"

Em vez de rodar manualmente cada serviço, declare dependências:

```toml
# ~/code/frontend/devobox.toml
[project]
startup_command = "npm run dev"

[dependencies]
include_projects = ["../backend-api", "../auth-service"]
```

```toml
# ~/code/backend-api/devobox.toml
[project]
startup_command = "rails server -p 3001"

[services.api-db]
type = "database"
image = "postgres:16"
ports = ["5433:5432"]
env = ["POSTGRES_PASSWORD=dev"]
```

#### Execução

```bash
devobox project up frontend
```

**Resultado:**

```
Zellij: devobox-frontend
┌─────────────────────────────────┐
│ Aba 1: frontend                 │
│ $ npm run dev                   │
│ > vite                          │
│ ✓ http://localhost:5173         │
├─────────────────────────────────┤
│ Aba 2: backend-api              │
│ $ rails server -p 3001          │
│ => Rails 7.0 app                │
│ ✓ http://localhost:3001         │
├─────────────────────────────────┤
│ Aba 3: auth-service             │
│ $ node server.js                │
│ ✓ http://localhost:3002         │
└─────────────────────────────────┘

Serviços em background:
🗄️ api-db (Postgres) — :5433
```

**Tudo com um comando!**

---

#### Dicas para Microsserviços

**1. Use portas alternativas**

```toml
# Projeto principal: porta padrão
startup_command = "npm run dev"  # → :5173

# Dependências: portas alternativas
startup_command = "rails server -p 3001"  # → :3001
```

**2. Bind em 0.0.0.0, não 127.0.0.1**

```bash
# ❌ Errado (não funciona com port mapping)
rails server -b 127.0.0.1

# ✅ Correto
rails server -b 0.0.0.0
```

**3. Verifique logs se falhar**

```bash
podman logs backend-api -f
```

---

## Parte 4: Comparações Detalhadas

### Round 1: Devobox vs Desenvolvimento Local

#### O Caos das Atualizações

**Cenário:**
Você desenvolve no Arch Linux (ou Ubuntu com PPAs).

**Dia 1:**

```bash
$ ruby -v
ruby 3.1.0

$ bundle install
✓ Gems instaladas
```

**Dia 2 (após update):**

```bash
$ sudo pacman -Syu
# openssl: 1.1.1 → 3.0.0
# glibc: 2.35 → 2.36

$ bundle install
❌ ERRO: OpenSSL não encontrado
❌ ERRO: gem nokogiri não compila
```

**Solução improvisada:**

- Downgrade de bibliotecas (quebra outros apps)
- Compilar openssl antigo manualmente
- Usar Docker (perde performance)
- Reinstalar Ruby via rbenv/asdf

**Tempo perdido:** 2-4 horas

#### Como Devobox Resolve

**Isolamento:**

```
┌────────────────────────┐
│ Host (Arch Linux)      │
│ - Kernel: 6.x          │
│ - OpenSSL: 3.0         │ ← Atualiza livremente
│ - glibc: 2.36          │
└───────────┬────────────┘
            │
┌───────────▼────────────┐
│ Container (Debian 12)  │
│ - OpenSSL: 1.1.1       │ ← Congelado
│ - glibc: 2.35          │
│ - Ruby compila contra  │
│   estas versões        │
└────────────────────────┘
```

**Resultado:**

```bash
# No host
$ sudo pacman -Syu
# (atualiza tudo)

# No container
$ devobox
$ bundle install
✓ Funciona perfeitamente
```

---

### Round 2: Devobox vs Docker Compose

#### Problema 1: Arquivos do Root

**Docker Compose tradicional:**

```bash
$ docker-compose run web rails g migration AddUser
# Cria arquivo de migração

$ ls -la db/migrate
-rw-r--r-- 1 root root 245 ... 20230515_add_user.rb
              ^^^^

$ nvim db/migrate/20230515_add_user.rb
❌ Permission denied

$ sudo chown -R $USER:$USER db/migrate
# Tem que fazer isso TODA VEZ
```

**Por que acontece:**

Container roda como root (UID 0). Arquivos criados pertencem a root no host.

**Como Devobox resolve:**

```bash
--userns=keep-id

# Mapeia matematicamente:
Host UID 1000 → Container UID 1000
```

```bash
$ devobox
$ rails g migration AddUser

$ ls -la db/migrate
-rw-r--r-- 1 joao joao 245 ... 20230515_add_user.rb
              ^^^^ ^^^^

$ nvim db/migrate/20230515_add_user.rb
✓ Funciona!
```

---

#### Problema 2: N Dockerfiles

**Docker Compose:**

Você tem 5 projetos. Cada um precisa de:

```dockerfile
# Dockerfile.dev (projeto 1)
FROM ruby:3.2
RUN apt-get update && apt-get install -y git curl vim zsh
RUN sh -c "$(curl -fsSL oh-my-zsh.install)"
COPY . /app
CMD rails server
```

Multiplique por 5 projetos. Se você mudar de Bash para Zsh, precisa:

1. Editar 5 Dockerfiles
2. Rebuild 5 imagens
3. Esperar ~15 minutos

**Devobox:**

Uma imagem base para tudo:

```toml
# ~/.config/devobox/devobox.toml (global)
[build]
image_name = "devobox-img"

# mise.toml define ferramentas
# Todos os projetos usam essa imagem
```

Mudou de Bash para Zsh?

```bash
devobox rebuild
# Rebuilda 1 vez
# Todos os 5 projetos atualizam
```

---

#### Problema 3: Complexidade de Volumes

**Docker Compose:**

```yaml
volumes:
  - .:/app # Código
  - node_modules:/app/node_modules # Cache de deps
  - bundle:/usr/local/bundle # Cache de gems
```

Isso é necessário porque:

- Bind mount sobrescreve `node_modules`
- Precisa de volumes anônimos para cache
- YAML fica complexo

**Devobox:**

```bash
# Bind mount simples
~/code/meu-projeto → /home/dev/code/meu-projeto

# node_modules fica no container naturalmente
# Sem volumes extras necessários
```

---

#### Recursos Exclusivos do Devobox

Além de resolver os problemas acima, Devobox tem funcionalidades que Docker Compose simplesmente não oferece:

##### 1. 🔗 Cascata de Dependências entre Projetos

**Docker Compose:**

```bash
# Você precisa fazer manualmente:
cd ~/frontend && docker-compose up -d
cd ~/backend && docker-compose up -d
cd ~/auth-service && docker-compose up -d

# Ordem importa. Se esquecer um, quebra.
```

**Devobox:**

```toml
# ~/code/frontend/devobox.toml
[project]
name = "frontend"

[dependencies]
include_projects = ["../backend"]  # ← Cascata automática

# ~/code/backend/devobox.toml
[project]
name = "backend"

[dependencies]
include_projects = ["../auth-service"]  # ← Cascata de 2 níveis
```

```bash
devobox project up frontend

# Resultado:
# 🚀 Iniciando auth-service...     ← Nível 2 (dependência do backend)
# 🚀 Iniciando backend...          ← Nível 1 (dependência do frontend)
# 🚀 Iniciando frontend...         ← Projeto principal
# 💖 Verificando healthchecks...
# ✅ Todos saudáveis!
```

**Impacto:** Um comando para subir arquitetura inteira na ordem correta.

---

##### 2. 🎯 Workspace Multi-Projeto com Terminal Integrado

**Docker Compose:**

```bash
# Cada serviço em uma janela/aba separada
Terminal 1: docker-compose logs -f frontend
Terminal 2: docker-compose logs -f backend
Terminal 3: docker-compose exec frontend sh
Terminal 4: docker-compose exec backend sh

# Você gerencia isso manualmente
```

**Devobox:**

```bash
devobox project up frontend
```

**Resultado automático (Zellij session):**

```
┌─────────────────────────────────────┐
│ Tab 1: frontend-shell               │  ← Shell interativo do frontend
├─────────────────────────────────────┤
│ Tab 2: backend-shell                │  ← Shell interativo do backend
├─────────────────────────────────────┤
│ Tab 3: auth-service-shell           │  ← Shell interativo do auth
├─────────────────────────────────────┤
│ Tab 4: frontend-logs                │  ← Logs do frontend (via startup_command)
└─────────────────────────────────────┘

# Alt+n = próxima tab
# Alt+p = tab anterior
# Ctrl+o, d = detach (tudo continua rodando)
```

**Impacto:** Workspace completo configurado automaticamente. Detach e volte depois — tudo preservado.

---

##### 3. ⏱️ Healthcheck Ativo (CLI Espera Antes de Liberar)

**Docker Compose:**

```yaml
services:
  db:
    image: postgres:16
    healthcheck:
      test: ["CMD", "pg_isready"]
      interval: 5s
```

```bash
docker-compose up -d

# O que acontece:
Starting postgres... done    ← Container iniciou
                             ← Mas Postgres ainda está inicializando...

# Seu app tenta conectar:
$ rails db:migrate
Error: connection refused    ← Postgres ainda não está pronto!

# Você faz:
$ sleep 10 && rails db:migrate  ← Gambiarra
```

**Devobox:**

```toml
[services.pg]
type = "database"
image = "postgres:16"
healthcheck_command = "pg_isready -U postgres"
healthcheck_interval = "5s"
```

```bash
devobox -d

# O que acontece:
🚀 Iniciando pg...
💖 Verificando healthchecks...
  🩺 Aguardando pg ficar saudável... (tentativa 1/5)
  🩺 Aguardando pg ficar saudável... (tentativa 2/5)
  ✅ pg está saudável!               ← CLI só libera quando REALMENTE pronto
🚀 Entrando no devobox...

$ rails db:migrate
✓ Funciona na primeira vez!         ← Sem "connection refused"
```

**Impacto:** Nunca mais `sleep` ou race conditions no startup.

---

##### 4. 📊 Separação Semântica: Database vs Service

**Docker Compose:**

```yaml
services:
  postgres:
    image: postgres:16
  redis:
    image: redis:7
  mailhog:
    image: mailhog/mailhog
```

Todos são iguais. Você usa `docker-compose start` para tudo ou nada.

**Devobox:**

```toml
[services.pg]
type = "database"         ← Marcado como banco
image = "postgres:16"

[services.redis]
type = "database"         ← Marcado como banco
image = "redis:7"

[services.mailhog]
type = "generic"          ← Marcado como serviço genérico
image = "mailhog/mailhog"
```

**Controle granular:**

```bash
# Iniciar apenas bancos de dados
devobox db start
# ✓ pg iniciado
# ✓ redis iniciado
# ✗ mailhog NÃO foi iniciado

# Iniciar serviços genéricos
devobox service start
# ✓ mailhog iniciado

# Parar só os bancos
devobox db stop
```

**Impacto:** Controle fino sobre o que sobe. Útil para economizar recursos.

---

##### 5. 🔍 Auto-Descoberta de Projetos

**Docker Compose:**

```bash
# Você precisa navegar manualmente
cd ~/code/meu-projeto-x
docker-compose up

cd ~/code/meu-projeto-y
docker-compose up
```

**Devobox:**

```bash
devobox project list

# Resultado (escaneia ~/code automaticamente):
📁 Projetos encontrados em ~/code:

  • frontend      ~/code/frontend
  • backend       ~/code/backend
  • auth-service  ~/code/auth-service
  • legacy-app    ~/code/legacy-app

# Ativar qualquer um de qualquer lugar:
devobox project up backend

# ✓ Entra no diretório automaticamente
# ✓ Inicia serviços do backend
# ✓ Aplica configuração local
```

**Impacto:** Navegação zero. Trabalhe em qualquer projeto de qualquer lugar.

---

##### 6. 🚀 Orquestração de Startup Command

**Docker Compose:**

```yaml
services:
  web:
    image: ruby:3.2
    command: rails server
```

Se você quer rodar múltiplos comandos, precisa de:

- Script shell customizado
- Supervisord/PM2
- Ou rodar manualmente após `docker-compose up`

**Devobox:**

```toml
[project]
name = "backend"
startup_command = "rails server -p 3000"
```

```bash
devobox project up backend
```

**O que acontece:**

1. Hub container inicia
2. Serviços (Postgres, Redis) sobem e passam healthcheck
3. Zellij session criada
4. Tab principal executa: `rails server -p 3000` automaticamente
5. Logs aparecem na tab "backend-logs"

**Impacto:** Ambiente **completo** em um comando. Zero setup manual.

---

#### Veredito Final

| Característica                      | Docker Compose     | Devobox                 |
| ----------------------------------- | ------------------ | ----------------------- |
| **Permissões de arquivo**           | 🔴 Root owns       | 🟢 Você é dono          |
| **Consistência de ambiente**        | 🔴 N Dockerfiles   | 🟢 1 imagem base        |
| **Performance de rede**             | 🟡 Bridge NAT      | 🟢 Host network         |
| **Persistência de ambiente**        | 🔴 Efêmero         | 🟢 Pet persistente      |
| **Complexidade de config**          | 🟡 Médio-alto      | 🟢 Baixo                |
| **Healthchecks**                    | 🟡 Passivos        | 🟢 Ativos + bloqueantes |
| **Dependências entre projetos**     | 🔴 Manual          | 🟢 Cascata automática   |
| **Workspace multi-projeto**         | 🔴 DIY             | 🟢 Zellij integrado     |
| **Controle semântico (db/service)** | 🔴 Não tem         | 🟢 Nativo               |
| **Auto-descoberta de projetos**     | 🔴 Não tem         | 🟢 Nativo               |
| **Orquestração de startup**         | 🟡 Scripts manuais | 🟢 Declarativo          |
| **Flexibilidade**                   | 🟢 Total controle  | 🟡 Opinado              |
| **Multi-plataforma**                | 🟢 Linux/Mac/Win   | 🟡 Linux-first          |

#### Quando usar cada um?

**Use Docker Compose quando:**

- Você precisa de máxima flexibilidade (containers diferentes, redes customizadas)
- Equipe multi-plataforma (Windows devs precisam rodar)
- Deploy de produção (não é o caso de uso do Devobox)
- Orquestração pontual de serviços

**Use Devobox quando:**

- Você quer um workspace persistente
- Múltiplos projetos interconectados (microserviços)
- Quer evitar problemas de permissão e user namespace
- Prefere `localhost` sem configurar port forwarding
- Quer healthchecks que realmente bloqueiam até pronto
- Precisa de cascata automática de dependências
- Desenvolvimento local Linux-first

---

### Round 3: Devobox vs "App as a Service" Manual

#### Problema: Sincronia de Versões

**Cenário:**

- **Frontend** depende de **Backend**
- Backend usa Ruby 3.2.0

**Configuração manual:**

```dockerfile
# Backend Dockerfile
FROM ruby:3.2.0
COPY . /app
CMD rails server
```

**O que quebra:**

1. Dev atualiza Backend para Ruby 3.2.2
2. Commita `.tool-versions` ou `.ruby-version`
3. Esquece de atualizar Dockerfile
4. Frontend tenta subir Backend via Docker
5. ❌ Backend roda com Ruby 3.2.0 (imagem antiga)
6. ❌ Código espera 3.2.2
7. ❌ Comportamento inesperado ou crash

**Solução manual:**

1. Lembrar de atualizar Dockerfile também
2. Rebuild imagem
3. Push para registry
4. Frontend puxa nova imagem
5. Muito atrito!

#### Como Devobox Resolve

**Sem Dockerfiles customizados:**

Backend:

```toml
# ~/code/backend/.tool-versions (mise)
ruby 3.2.2
```

Frontend:

```toml
# ~/code/frontend/devobox.toml
[dependencies]
include_projects = ["../backend"]
```

**Execução:**

```bash
devobox project up frontend
```

**O que acontece:**

1. Devobox lê `backend/.tool-versions`
2. Vê: `ruby 3.2.2`
3. Roda `mise install ruby@3.2.2` no container
4. Mise baixa/instala Ruby 3.2.2
5. Backend inicia com versão correta

**Auto-cura:** Se dev atualizar `.tool-versions`, próxima execução já pega a nova versão.

---

## Parte 5: Tópicos Avançados

### SSH Agent Forwarding

Permite usar suas chaves SSH do host dentro do container.

#### Como Funciona

```
Host:
~/.ssh/id_ed25519 (chave privada)
       ↓
ssh-agent (daemon)
       ↓
$SSH_AUTH_SOCK (socket)

Container:
/run/host/ssh-agent.sock → bind mount → $SSH_AUTH_SOCK
                                               ↓
                                          git clone git@github.com:user/repo.git
```

#### Configuração

**No host:**

```bash
# 1. Iniciar agent
eval "$(ssh-agent -s)"

# 2. Adicionar chave
ssh-add ~/.ssh/id_ed25519

# 3. Verificar
ssh-add -l
```

**No container (automático):**

```bash
devobox
git clone git@github.com:user/private-repo.git
# ✓ Funciona! Usa a chave do host
```

**Segurança:**

- Chave privada NUNCA entra no container
- Apenas o socket é montado (read-only)
- Agent faz a assinatura criptográfica no host

---

### Ferramentas Customizadas via Mise

Mise gerencia versões de linguagens e ferramentas.

#### Instalação de Runtimes

```bash
# Node.js
mise install node@20
mise use node@20

# Ruby
mise install ruby@3.2

# Python
mise install python@3.11

# Rust
mise install rust@stable

# Múltiplos ao mesmo tempo
mise install node@20 ruby@3.2 python@3.11
```

#### Arquivo `.tool-versions`

```bash
# ~/code/meu-projeto/.tool-versions
node 20.11.0
ruby 3.2.2
python 3.11.5
```

Mise detecta automaticamente:

```bash
cd ~/code/meu-projeto
node --version
# v20.11.0 (usa a versão do .tool-versions)
```

#### Plugins

```bash
# Listar plugins disponíveis
mise plugins ls-remote

# Instalar plugin
mise plugin install terraform
mise install terraform@1.5.0
```

---

### Gerenciamento de Volumes

#### Onde os Dados Vivem

**Código:**

```bash
~/code/meu-projeto → /home/dev/code/meu-projeto
# Bind mount (read-write)
# Mudanças aparecem em ambos os lados instantaneamente
```

**Dados de serviços:**

```bash
# Postgres
/var/lib/containers/storage/volumes/pg-data

# Redis
/var/lib/containers/storage/volumes/redis-data
```

#### Backup

```bash
# Listar volumes
podman volume ls

# Inspecionar
podman volume inspect pg-data

# Backup
podman run --rm \
  -v pg-data:/data \
  -v $(pwd):/backup \
  alpine tar czf /backup/pg-backup.tar.gz /data

# Restore
podman run --rm \
  -v pg-data:/data \
  -v $(pwd):/backup \
  alpine tar xzf /backup/pg-backup.tar.gz -C /
```

---

### Performance Tuning

#### Filesystem: Btrfs/ZFS

Se usar Btrfs ou ZFS, desabilite Copy-on-Write:

```bash
# Btrfs
sudo chattr +C ~/.local/share/containers/storage

# ZFS
sudo zfs set compression=off pool/containers
```

**Ganho:** 30-50% mais rápido em I/O intensivo.

#### Cache de Build

Use build cache do Podman:

```bash
# Em ~/.config/devobox/Containerfile
RUN --mount=type=cache,target=/var/cache/apt \
    apt-get update && apt-get install -y nodejs
```

#### Shared Memory

Se rodar navegadores ou apps com muita memória compartilhada:

```bash
podman run --shm-size=2g ...
```

---

## Resumo

**Conceitos:**

- 4 Pilares: Higiene, Performance, Pet, Orquestração
- Hub & Spoke: Arquitetura de rede inteligente
- Pet vs Cattle: Persistência vs Descartabilidade

**Configuração:**

- Cascata de configs: Global → Local
- Healthchecks ativos: Serviços prontos antes de liberar shell
- Dependências: Projetos podem incluir outros projetos

**Workflows:**

- Diário: Detach para preservar estado
- Multi-projeto: Sessões Zellij isoladas
- Microsserviços: App as a Service com startup_command

**Comparações:**

- vs Local: Isolamento sem perder performance
- vs Docker Compose: Sem problemas de permissão, config unificada
- vs Manual: Auto-sincronização de versões

**Avançado:**

- SSH forwarding para Git
- Mise para gerenciar runtimes
- Otimizações de filesystem

---

**Próximos passos:**

➡️ **[Cookbook](COOKBOOK.md)** - Receitas práticas para cenários comuns
➡️ **[Arquitetura](ARCHITECTURE.md)** - Detalhes técnicos de implementação
