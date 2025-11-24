# 📦 Devobox

**Estação de Trabalho Híbrida para Desenvolvimento no Linux**

![Arquitetura Devobox](docs/architecture.png)

> _Desenvolva sem poluir seu sistema, sem perder performance e sem reinventar o ambiente a cada projeto._

## 🎯 O Problema

O **Devobox** é uma resposta de engenharia para o dilema moderno do desenvolvimento no Linux: **"Como manter meu sistema limpo e estável sem sacrificar o desempenho e a ergonomia do desenvolvimento nativo?"**

Este projeto não é apenas "rodar containers". É criar uma **Estação de Trabalho Híbrida** que resolve 4 problemas fundamentais do desenvolvimento moderno.

---

## 🏛️ Os 4 Pilares do Devobox

### 1. 🧹 Higiene Absoluta do Host (O Fim do "Dependency Hell")

No Arch Linux (Rolling Release), as bibliotecas do sistema (`openssl`, `libicu`, `glibc`) atualizam constantemente.

**O Problema:**
Se você instala Ruby/Node/Python direto no seu Host, um `pacman -Syu` pode quebrar seu ambiente de desenvolvimento numa segunda-feira de manhã porque a versão do OpenSSL mudou e o Ruby antigo não compila mais.

**A Solução Devobox:**
Isolar **100%** das runtimes de linguagem (Ruby, Node, Rust, Go) e bibliotecas de sistema dentro de uma "caixa de vidro".

- Seu Arch Host fica apenas com: Kernel, Drivers, Interface Gráfica, Editor e Navegador
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

### 4. 💾 Eficiência de Recursos (O Modelo "Shared Services")

Desenvolvedores que trabalham em microserviços ou múltiplos projetos costumam ter vários arquivos `docker-compose.yml` espalhados.

**O Problema:**
Se você subir 3 projetos, você terá 3 instâncias de Postgres e 3 de Redis rodando, consumindo 1GB+ de RAM desnecessariamente.

**A Solução Devobox:**
Centralizar a infraestrutura.

- **Um** Postgres. **Um** Redis
- Todos os seus projetos usam o mesmo banco (apenas com nomes de databases diferentes)
- Isso economiza bateria e RAM, permitindo que você desenvolva em hardware mais modesto (ou abra mais abas no Chrome 😁)

---

## 📋 Requisitos

- **Podman** instalado no sistema
- **Linux** (otimizado para Arch Linux)
- `~/.local/bin` no seu PATH

## 🚀 Instalação

### Método 1: Instalar via Release (Recomendado)

```bash
# 1. Baixar a última release
curl -L https://github.com/JohnAnon9771/devobox/releases/latest/download/devobox-linux-x86_64.tar.gz -o devobox.tar.gz

# 2. Extrair o binário
tar xzf devobox.tar.gz

# 3. Mover para ~/.local/bin
mkdir -p ~/.local/bin
mv devobox-linux-x86_64 ~/.local/bin/devobox
chmod +x ~/.local/bin/devobox

# 4. Garantir que ~/.local/bin está no PATH
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# 5. Verificar instalação
devobox --version

# 6. Configurar ambiente (setup automático)
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

O comando `devobox init` cuida de toda a preparação do ambiente:

1. Copia os templates (`Containerfile` e `databases.yml`) para `~/.config/devobox`
2. Constrói a imagem base com todas as ferramentas definidas em `mise.toml`
3. Cria os containers (devobox + bancos definidos no `databases.yml`)

**Ainda mais fácil:** Se você executar `devobox` sem fazer o setup, ele detecta automaticamente e executa o `init` para você!

## 🛠️ Comandos

### 🎯 Comandos Essenciais (Uso Diário)

```bash
# Abrir shell de desenvolvimento (comando padrão)
devobox                    # Abre o shell (auto-setup se necessário)
devobox -d                 # Abre o shell com bancos de dados
devobox --with-dbs         # Forma longa de -d

# Comandos alternativos
devobox shell              # Shell sem bancos
devobox dev                # Shell com bancos (equivale a -d)

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
devobox up                 # Sobe devobox + todos os bancos
devobox start              # Alias de 'up'
devobox down               # Para todos os containers
devobox stop               # Alias de 'down'

# Ver status
devobox status             # Lista todos os containers e estados
```

### 🔧 Comandos Avançados

```bash
# Shell com opções especiais
devobox --auto-stop        # Para tudo ao sair (economiza recursos)
devobox -d --auto-stop     # Com bancos + auto-stop
devobox shell --with-dbs   # Shell com bancos (forma explícita)

# Reconstruir com opções
devobox rebuild --skip-cleanup   # Reconstrói sem limpar cache
```

**🎯 Dicas de Uso:**

- O comando `devobox` (sem argumentos) é o jeito mais rápido de começar a trabalhar
- Ele mapeia automaticamente seu diretório atual: `cd ~/code/projeto1 && devobox` já te coloca em `/home/dev/code/projeto1`
- Na primeira execução, faz setup automático - você não precisa se preocupar com nada!

**⚡ Modo Auto-Stop:**

O flag `--auto-stop` encerra **todos os containers** automaticamente quando você sai do shell, liberando recursos do sistema:

```bash
$ devobox -d --auto-stop
🔌 Iniciando pg...
🔌 Iniciando redis...
🚀 Entrando no devobox (workdir Some("/home/dev/code/myproject"))

# [Você trabalha normalmente]

$ exit
🧹 Encerrando todos os containers...
  💤 Parando devobox... ✓
  💤 Parando pg... ✓
  💤 Parando redis... ✓
✅ Containers encerrados
```

**Quando usar `--auto-stop`:**

- ✅ Sessões rápidas de desenvolvimento
- ✅ Quando quer economizar RAM/CPU automaticamente
- ✅ Máquinas com recursos limitados
- ✅ Trabalho em múltiplos projetos no mesmo dia

**Quando NÃO usar `--auto-stop`:**

- ❌ Sessões longas com múltiplas entradas/saídas
- ❌ Quando vai voltar ao shell logo em seguida
- ❌ Múltiplas sessões shell simultâneas (terminais diferentes)

### 🗄️ Gerenciamento de Bancos de Dados

```bash
# Iniciar todos os bancos
devobox db start

# Iniciar banco específico
devobox db start pg
devobox db start redis

# Parar todos os bancos
devobox db stop

# Parar banco específico
devobox db stop pg

# Reiniciar bancos
devobox db restart [pg|redis]

# Ver status dos bancos
devobox db status
```

### 🧹 Limpeza de Recursos

O Devobox inclui comandos de limpeza para remover recursos não utilizados do Podman e liberar espaço em disco:

```bash
# Limpar tudo (containers parados, imagens não utilizadas, volumes órfãos e cache de build)
devobox cleanup

# Limpar apenas containers parados
devobox cleanup --containers

# Limpar apenas imagens não utilizadas
devobox cleanup --images

# Limpar apenas volumes órfãos
devobox cleanup --volumes

# Limpar apenas cache de build
devobox cleanup --build-cache

# Combinações (limpar containers e imagens)
devobox cleanup --containers --images
```

**Limpeza Automática:**

- O comando `devobox rebuild` **executa limpeza automática** antes de construir
- Remove containers parados, imagens não utilizadas e cache de build
- **Preserva volumes de dados** dos bancos de dados
- Use `--skip-cleanup` para pular a limpeza automática

**O que cada operação remove:**

| Operação        | Comando Podman              | O que remove                               |
| --------------- | --------------------------- | ------------------------------------------ |
| `--containers`  | `podman container prune -f` | Containers parados                         |
| `--images`      | `podman image prune -af`    | Imagens não utilizadas e dangling          |
| `--volumes`     | `podman volume prune -f`    | Volumes órfãos (não anexados a containers) |
| `--build-cache` | `podman builder prune -af`  | Cache de build (layers intermediárias)     |

> **💡 Dica:** Execute `devobox cleanup` periodicamente para manter seu sistema limpo e liberar espaço em disco.

## 📁 Estrutura de Diretórios

### No Repositório (antes da instalação)

```
devobox/
├── config/
│   ├── Containerfile    → Definição da imagem
│   └── databases.yml    → Bancos de dados de exemplo (YAML)
├── docs/
│   └── architecture.png → Diagrama de arquitetura
└── Cargo.toml           → Crate Rust do CLI `devobox`
```

### Pós-Instalação

```
~/code/                  → Seus projetos (mapeado para /home/dev/code)
~/.config/devobox/       → Configuração instalada
  ├── Containerfile      → Definição da imagem
  └── databases.yml      → Bancos de dados em YAML
~/.local/bin/
  └── devobox            → Binário Rust
```

**Importante:** Seus projetos devem estar em `~/code` para serem acessíveis dentro do container.

## 🗄️ Bancos de Dados

### PostgreSQL 16

```yaml
Host: localhost
Porta: 5432
Usuário: dev
Senha: devpass
Database padrão: dev_default
```

```bash
# Conexão via CLI
psql -h localhost -U dev -d dev_default

# Connection string para apps
postgresql://dev:devpass@localhost:5432/dev_default
```

### Redis 7

```yaml
Host: localhost
Porta: 6379
Senha: (sem autenticação)
```

```bash
# Conexão via CLI
redis-cli

# Connection string para apps
redis://localhost:6379
```

## 🔧 Stack Tecnológico

### Container Base: Arch Linux Latest

**Ferramentas de Desenvolvimento:**

- `base-devel` - Compiladores (gcc, make, etc)
- `git`, `curl`, `wget`, `openssh`
- `vim`, `man-db`

**Bibliotecas do Sistema:**

- `libffi`, `zlib`, `openssl`, `readline`
- `ncurses`, `libyaml`, `gdbm`

**Clientes de Banco:**

- `postgresql-libs` (libpq)
- `redis`

**Processamento de Mídia:**

- `imagemagick` - Manipulação de imagens
- `vips` - Processamento de imagens de alta performance

**Ferramentas de Rede:**

- `iputils`, `iproute2`, `bind-tools`

**Gerenciador de Runtime:**

- **[Mise](https://mise.jdx.dev/)** - Gerenciador de versões (sucessor do asdf)
  - Node.js, Ruby, Python, Go, Rust, Elixir, etc
  - Instalação automática baseada em `.tool-versions` ou `.mise.toml`

## 📝 Workflow Completo

### Fluxo Simplificado (Recomendado)

```bash
# 1. Primeira vez: fazer setup
devobox init
# Ou apenas: devobox (faz auto-setup)

# 2. Navegar para seu projeto
cd ~/code/meu-projeto

# 3. Abrir shell com bancos de dados
devobox -d
# Ou: devobox (sem bancos)

# 4. Trabalhar normalmente dentro do container
npm install
bundle install
rails db:migrate
rails server

# 5. Sair
exit
```

### Fluxo Completo com Customizações

```bash
# 1. Definir ferramentas (no host)
# Edite ~/.config/devobox/mise.toml
# [tools]
# node = "20.11.0"
# ruby = "3.2.2"

# 2. Reconstruir ambiente (aplica mudanças)
devobox rebuild

# 3. Navegar para seu projeto
cd ~/code/meu-projeto

# 4. Iniciar ambiente com bancos e auto-stop
devobox -d --auto-stop

# 5. Instalar dependências do projeto
npm install
bundle install

# 6. Criar database no Postgres
createdb meu_projeto_dev

# 7. Rodar migrações/seeds
rails db:migrate
npm run migrate

# 8. Desenvolver normalmente
rails server
# ou
npm run dev

# 9. Sair do container (auto-stop para tudo)
exit
```

## 🏗️ Arquitetura Técnica

### Containers Criados

1. **devobox** - Container principal de desenvolvimento
   - Imagem: `devobox-img` (Arch Linux customizado)
   - Usuário: `dev` (não-root)
   - Network: `--network host` (performance máxima)
   - Volumes:
     - `~/code:/home/dev/code` (bind mount - projetos)
   - Segurança: `--userns=keep-id` (preserva UID/GID do host)

2. **postgres** - PostgreSQL 16
   - Estado padrão: Parado (start sob demanda)
   - Network: Bridge (port mapping `-p 5432:5432`)
   - Porta: 5432
   - Dados: Persistem entre restarts, perdidos no rebuild

3. **redis** - Redis 7 Alpine
   - Estado padrão: Parado (start sob demanda)
   - Network: Bridge (port mapping `-p 6379:6379`)
   - Porta: 6379
   - Dados: Persistem entre restarts, perdidos no rebuild

### Volumes Persistentes

**Volume do Mise (`devobox-mise`):**

O Devobox usa um volume nomeado para o diretório de instalações do Mise (`~/.local/share/mise`):

- ✅ **Ruby, Node, Python compilados persistem** entre rebuilds
- ✅ Economiza tempo - não precisa recompilar tudo a cada rebuild
- ✅ Automático - criado na primeira execução
- ⚠️ Se quiser limpar: `podman volume rm devobox-mise`

**Exemplo de ganho de tempo:**

```bash
# Primeiro build: instala Ruby (demora ~10min)
devobox rebuild

# Segundo rebuild: Ruby já está instalado (instantâneo!)
devobox rebuild
```

### Decisões de Design

**Por que `--network host` (apenas no devobox)?**

- O container **devobox** usa `--network host` para performance máxima
- Postgres e Redis usam **bridge networking** com port mapping (`-p`)
- Isso permite que aplicações no devobox acessem `localhost:5432` e `localhost:6379` diretamente
- Simplifica configuração: `DATABASE_URL=postgresql://dev:devpass@localhost:5432/mydb`
- Elimina latência de bridge networking para o ambiente de desenvolvimento

**Por que montar `~/.ssh` do host?**

- Permite usar Git via SSH sem configurar chaves dentro do container
- Montado como **read-only** (`:ro`) por segurança
- Suas chaves SSH do host funcionam automaticamente no container
- Facilita push/pull em repositórios privados

**Por que `--userns=keep-id`?**

- Arquivos criados no container pertencem ao seu usuário no host
- Evita problemas de permissão com `git`, `npm`, `bundle`
- UID/GID dentro do container = UID/GID no host

**Por que `--security-opt label=disable`?**

- Desativa SELinux labeling (compatibilidade com diferentes distros)
- Necessário para bind mounts funcionarem corretamente

**Por que containers separados para DBs?**

- Permite gerenciamento granular (start/stop individual)
- Facilita upgrade de versões (ex: Postgres 16 → 17)
- Economiza recursos (inicia apenas o que precisa)

**Persistência de Dados:**

- ✅ **Ferramentas Mise**: Persistem em volume nomeado `devobox-mise` (instalações sobrevivem ao rebuild!)
- ✅ **Projetos**: Persistem via bind mount `~/code` (seus arquivos no host)
- ✅ **Chaves SSH**: Compartilhadas do host via bind mount read-only
- ⚠️ **Histórico bash**: NÃO persiste (perdido ao executar `rebuild`)
- ⚠️ **Bancos de dados**: Persistem entre restarts (`down`/`up`), mas são **perdidos** ao executar `rebuild`
- 💡 **Dica**: Para persistência permanente de dados de banco, declare volumes nomeados no `databases.yml`

## ⚙️ Customização

### Fluxo de Customização

```bash
# 1. Instalar apenas os arquivos de configuração
devobox install

# 2. Editar as configurações
vim ~/.config/devobox/Containerfile    # Adicionar ferramentas
vim ~/.config/devobox/mise.toml        # Configurar versões
vim ~/.config/devobox/databases.yml    # Configurar bancos

# 3. Construir com as mudanças
devobox rebuild
```

### Adicionar Ferramentas ao Container

Edite `~/.config/devobox/Containerfile`:

```dockerfile
RUN pacman -S --noconfirm \
    postgresql-libs redis imagemagick vips \
    sua-ferramenta-aqui
```

Depois reconstrua:

```bash
devobox rebuild
```

### Adicionar Novos Bancos de Dados

1. Edite `~/.config/devobox/databases.yml` e adicione entradas YAML com `name`, `image`, `ports` e `env` (opcionais):

```yaml
databases:
  - name: mongodb
    image: docker.io/mongo:7
    ports: ["27017:27017"]
    env: []
```

2. Reconstrua:

```bash
devobox rebuild
```

### Personalizar Prompt

O prompt padrão é:

```
[devobox] ~/code/projeto $
```

Para customizar, edite `~/.config/devobox/Containerfile`:

```dockerfile
RUN echo 'PS1="[\e[1;35m\]dev\[\e[0m\]] \w \$ "' >> ~/.bashrc
```

## 🐛 Troubleshooting

### Container não inicia

```bash
# Verificar logs
podman logs devobox

# Forçar reconstrução
podman rm -f devobox postgres redis
devobox rebuild
```

### Permissões de arquivo incorretas

O Devobox usa `--userns=keep-id` para preservar seu UID/GID. Se encontrar problemas:

```bash
# Dentro do container, verificar UID
id

# No host, deve ser o mesmo
id
```

### Bancos de dados não conectam

```bash
# Verificar se estão rodando
devobox db status

# Ver logs do Postgres
podman logs postgres

# Ver logs do Redis
podman logs redis

# Reiniciar
devobox db restart
```

### Mise não encontra ferramentas

As ferramentas devem ser definidas no `mise.toml` antes do build.

1. Edite `~/.config/devobox/mise.toml`
2. Rode `devobox rebuild`

### Performance lenta de I/O

Se você estiver usando um filesystem com CoW (Btrfs, ZFS):

```bash
# Desabilitar CoW no diretório de volumes do Podman
sudo chattr +C ~/.local/share/containers/storage
```

## 🎓 Filosofia de Uso

O Devobox transforma seu "Inner Loop" (ciclo código → teste → debug) em um **produto profissional**.

**O que você NÃO precisa mais fazer:**

- ❌ Instalar múltiplas versões de Ruby/Node via RVM/NVM no host
- ❌ Debugar conflitos de biblioteca após `pacman -Syu`
- ❌ Rodar 5 instâncias de Postgres para 5 projetos
- ❌ Poluir seu sistema com dependências de compilação

**O que você GANHA:**

- ✅ Sistema host limpo e estável
- ✅ Ambiente de desenvolvimento reproduzível
- ✅ Performance nativa (zero overhead de VM)
- ✅ Gerenciamento centralizado de serviços
- ✅ Facilidade para resetar ambiente (1 comando)

---

**Desenvolvido para profissionais que valorizam controle, performance e higiene do sistema.**
