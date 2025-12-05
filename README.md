# 📦 Devobox

**Estação de Trabalho Híbrida para Desenvolvimento no Linux**

![Arquitetura Devobox](docs/architecture.png)

> _Desenvolva sem poluir seu sistema, sem perder performance e sem reinventar o ambiente a cada projeto._

[🧭 Novo no Devobox? Comece pelo Guia de Workflow (Como trabalhar)](docs/workflow.md)

## 🎯 O Problema

O **Devobox** é uma resposta de engenharia para o dilema moderno do desenvolvimento no Linux: **"Como manter meu sistema limpo e estável sem sacrificar o desempenho e a ergonomia do desenvolvimento nativo?"**

Este projeto não é apenas "rodar containers". É criar uma **Estação de Trabalho Híbrida** que resolve 4 problemas fundamentais do desenvolvimento moderno.

[🥊 Devobox vs. Docker Compose vs. Local: Entenda as diferenças](docs/comparison.md)

---

## 🏛️ Os 4 Pilares do Devobox

### 1. 🧹 Higiene Absoluta do Host (O Fim do "Dependency Hell")

No Arch Linux (Rolling Release), as bibliotecas do sistema (`openssl`, `libicu`, `glibc`) atualizam constantemente.

**O Problema:**
Se você instala Ruby/Node/Python direto no seu Host, um `pacman -Syu` pode quebrar seu ambiente de desenvolvimento numa segunda-feira de manhã porque a versão do OpenSSL mudou e o Ruby antigo não compila mais.

**A Solução Devobox:**
Isolar **100%** das runtimes de linguagem (Ruby, Node, Rust, Go) e bibliotecas de sistema dentro de uma "caixa de vidro".

- Seu Host fica apenas com: Kernel, Drivers, Interface Gráfica, Editor e Navegador
- O resto (gems, node_modules, compiladores) fica contido
- Se o container quebrar, você recria (`devobox rebuild`). Seu PC continua intacto

### 2. ⚡ Performance Nativa (Sem Camadas de Virtualização)

Muitas soluções Docker (como Docker Desktop no Mac/Windows) rodam dentro de uma Máquina Virtual oculta, tornando o acesso aos arquivos lento.

**O Problema:**
Rodar `bundle install` ou `npm install` em volumes Docker tradicionais pode ser extremamente lento.

**A Solução Devobox:**
Aproveitar o Linux para usar **Bind Mounts nativos** e **Network Host**.

- **I/O:** O container lê os arquivos na mesma velocidade que o Host. Zero overhead
- **Rede:** Ao usar `--network host`, removemos a ponte de rede (NAT). O container usa a placa de rede do seu PC. O `localhost` do container **é** o `localhost` do seu PC. Isso elimina a complexidade de mapear portas (`-p 3000:3000`)

### 3. 🐕 Ergonomia de "Pet" vs. "Cattle"

A filosofia Docker tradicional trata containers como gado (descartáveis e efêmeros). Para desenvolvimento, isso é inadequado.

**O Problema:**
Em ambientes Docker Compose puros, toda vez que você derruba o container, você perde o histórico do terminal (Ctrl+R), seus aliases temporários, e tem que reinstalar ferramentas de debug.

**A Solução Devobox:**
Criar um **"Container de Estimação" (Pet Container)**.

- Define suas ferramentas em `mise.toml`
- O container é imutável e reprodutível
- Se comporta como um **segundo computador** que está sempre lá, mas com configuração declarativa

### 4. 💾 Eficiência e Controle Granular

Desenvolvedores que trabalham em microserviços ou múltiplos projetos costumam ter vários arquivos `docker-compose.yml` espalhados.

**O Problema:**
- Rodar 3 instâncias de Postgres para 3 projetos diferentes consome RAM desnecessariamente.
- Erros de "Connection Refused" porque a aplicação sobe antes do banco estar pronto.

**A Solução Devobox (v0.5.0+):**
- **Orquestrador com Healthchecks:** O Devobox espera ativamente até que seus serviços estejam **realmente prontos**.
- **Separação Banco vs. Serviço:** Distinção clara entre infraestrutura persistente (Postgres, Redis) e serviços auxiliares (Mailhog, Mocks).
- **Configuração em Cascata:** Configurações globais para o dia a dia e locais para projetos específicos.
- **Dependências entre Projetos:** Um projeto pode importar automaticamente a infraestrutura de outro.

---

## 📋 Requisitos

- **Podman** instalado no sistema
- **Linux** (otimizado para Arch Linux)
- `~/.local/bin` no seu PATH

## 🚀 Instalação

### Método 1: Instalar via Release (Recomendado)

```bash
# Instalar
curl -L https://github.com/JohnAnon9771/devobox/releases/latest/download/devobox-linux-x86_64 -o ~/.local/bin/devobox && chmod +x ~/.local/bin/devobox

# Adicionar ao PATH (se necessário)
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc && source ~/.bashrc

# Configurar ambiente
devobox init
```

### Método 2: Compilar do Código Fonte

```bash
# 1. Clonar repositório
git clone https://github.com/JohnAnon9771/devobox.git
cd devobox

# 2. Compilar
cargo build --release

# 3. Instalar
install -Dm755 ./target/release/devobox ~/.local/bin/devobox

# 4. Configurar ambiente (setup automático)
devobox init
```

### Após a Instalação

O comando `devobox init` cuida de tudo:
1. Cria configs em `~/.config/devobox`.
2. Constrói a imagem base com ferramentas do `mise.toml`.
3. Instala ferramentas de IA globalmente.
4. Prepara os containers de serviço.

**Ainda mais fácil:** Se você executar `devobox` sem fazer o setup, ele detecta e executa o `init` automaticamente!

## 🛠️ Comandos

### 🎯 Comandos Essenciais (Uso Diário)

```bash
# Abrir shell de desenvolvimento (comando padrão)
devobox                    # Abre o shell (auto-setup se necessário)
devobox -d                 # Abre o shell com TODOS os serviços (bancos + genéricos) iniciados
devobox --with-dbs         # Forma longa de -d

# Comandos alternativos
devobox shell              # Shell sem iniciar serviços automaticamente
devobox dev                # Shell com serviços (equivale a -d)

# Gerenciar ambiente
devobox init               # Setup inicial completo (install + build)
devobox install            # Apenas instala configs (sem build)
devobox rebuild            # Reconstrói imagem e containers
devobox build              # Alias de 'rebuild'
devobox status             # Ver status de todos os containers
```

### 🗄️ Gerenciamento de Containers

```bash
# Subir/Parar containers
devobox up                 # Sobe tudo (Pet + Bancos + Serviços + Dependências)
devobox start              # Alias de 'up'
devobox down               # Para todos os containers
devobox stop               # Alias de 'down'

# Filtros de Inicialização
devobox up --dbs-only      # Sobe apenas o que é 'type: database'
devobox up --services-only # Sobe apenas o que é 'type: generic'

# Ver status
devobox status             # Lista todos os containers e estados
```

### 🔧 Comandos Avançados

```bash
# Shell com opções especiais
devobox --auto-stop        # Para tudo ao sair (economiza recursos)
devobox -d --auto-stop     # Com serviços + auto-stop

# Reconstruir com opções
devobox rebuild --skip-cleanup   # Reconstrói sem limpar cache
```

**⚡ Modo Auto-Stop:**

O flag `--auto-stop` encerra **todos os containers** automaticamente quando você sai do shell. Ideal para economizar bateria e RAM em sessões rápidas.

```bash
$ devobox -d --auto-stop
🚀 Iniciando todos os serviços...
  🔌 Iniciando pg... ✓
💖 Verificando healthchecks...
  🩺 Aguardando pg ficar saudável... ✅ Saudável!
🚀 Entrando no devobox...

# [Você trabalha...]

$ exit
🧹 Encerrando todos os containers...
✅ Containers encerrados
```

### 🎛️ Controle Granular: Bancos vs. Serviços

O Devobox permite diferenciar entre **Bancos de Dados** (pesados, persistentes) e **Serviços Genéricos** (leves, auxiliares).

#### Gerenciar Bancos (`type: database`)

```bash
devobox db start           # Inicia todos os bancos
devobox db start pg        # Inicia apenas o Postgres
devobox db stop            # Para todos os bancos
devobox db restart         # Reinicia bancos
devobox db status
```

#### Gerenciar Serviços Genéricos (`type: generic`)

```bash
devobox service start      # Inicia todos os serviços genéricos (ex: mailhog, redis-cache)
devobox service start queue # Inicia apenas a fila
devobox service stop       # Para serviços genéricos
devobox service restart
devobox service status
```

### 🧹 Limpeza de Recursos

O Devobox inclui comandos de limpeza para manter seu sistema enxuto:

```bash
# Limpar tudo (containers parados, imagens não utilizadas, volumes órfãos e cache)
devobox cleanup

# Limpezas específicas
devobox cleanup --containers
devobox cleanup --images
devobox cleanup --volumes
devobox cleanup --build-cache

# Opção nuclear (CUIDADO!)
devobox cleanup --nuke  # Remove TUDO do Podman no sistema. Comece do zero.
```

## 📁 Configuração e Estrutura

### Configuração Global vs. Local

O Devobox suporta uma configuração em cascata:

1.  **Global (`~/.config/devobox/`):** Configuração padrão para seu "Pet Container".
2.  **Local (`./devobox.toml`):** Configuração específica do projeto.

### Exemplo de `devobox.toml` Local

Use para declarar dependências de outros projetos:

```toml
# ~/code/frontend/devobox.toml

[project]
name = "meu-frontend"

[dependencies]
# O Devobox vai ler o services.yml desses caminhos e subir tudo junto!
include_projects = [
    "../backend-api",
    "../auth-service"
]

[container]
workdir = "/home/dev/code/frontend"
```

### Arquivo `services.yml`

Agora suporta **Tipos** e **Healthchecks**:

```yaml
services:
  # Banco de Dados (Controlado por 'devobox db')
  - name: pg
    type: database
    image: docker.io/postgres:16
    ports: ["5432:5432"]
    env:
      - POSTGRES_PASSWORD=dev
    healthcheck_command: "pg_isready -U dev"
    healthcheck_interval: "5s"
    healthcheck_timeout: "3s"
    healthcheck_retries: 5

  # Serviço Genérico (Controlado por 'devobox service')
  # Se 'type' for omitido, é 'generic' por padrão
  - name: mailhog
    type: generic
    image: docker.io/mailhog/mailhog:latest
    ports: ["1025:1025", "8025:8025"]
```

## 🔧 Stack Tecnológico

### Container Base: Debian Bookworm

**Ferramentas:**
- `build-essential`, `git`, `curl`, `wget`, `openssh`, `vim`

**Gerenciador de Runtime:**
- **[Mise](https://mise.jdx.dev/)** - Gerencia versões de linguagens (Node, Rust, Python, etc) globalmente dentro do container.

**IA Integration:**
- Ferramentas como `@anthropic-ai/claude-code` e `@google/gemini-cli` instaladas globalmente.

## 📚 Casos de Uso Avançados

### Orquestração de Microsserviços ("App as a Service")

Você sabia que pode usar o Devobox para subir automaticamente outros projetos dos quais você depende?

Imagine que você está trabalhando no Frontend (`my-frontend`) e precisa que a API (`my-api`) esteja rodando. Você pode configurar o `my-api` para rodar como um container auxiliar, gerenciado automaticamente pelo Devobox.

[➡️ Leia o guia completo de Microsserviços](docs/microservices.md)

## 🐛 Troubleshooting

### Container não inicia
```bash
podman logs devobox
devobox rebuild
```

### Permissões de arquivo
O Devobox usa `--userns=keep-id` para mapear seu UID do host, evitando problemas de `permission denied` em arquivos criados dentro do container.

### Performance lenta de I/O
Se usar Btrfs/ZFS, desabilite Copy-on-Write para o diretório do Podman:
```bash
sudo chattr +C ~/.local/share/containers/storage
```

---

**Desenvolvido para profissionais que valorizam controle, performance e higiene do sistema.**
