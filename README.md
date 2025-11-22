# 📦 Devobox

**Estação de Trabalho Híbrida para Desenvolvimento no Linux**

![Arquitetura Devobox](docs/architecture.png)

> _"Eu uso Arch Linux atualizado, mas meu ambiente de desenvolvimento é congelado, estável, reproduzível e não interfere no meu sistema operacional, rodando na velocidade máxima do hardware."_

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

- Persiste suas ferramentas instaladas via Mise
- Mantém o container rodando (não é destruído a cada uso)
- Se comporta como um **segundo computador** que está sempre lá, pronto para você trabalhar, mas que pode ser resetado se necessário

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

```bash
git clone <seu-repo>
cd devobox
./install.sh
```

O instalador irá:

1. Verificar se Podman está instalado
2. Copiar arquivos de configuração para `~/.config/devobox`
3. Criar link simbólico em `~/.local/bin/devobox`
4. Construir os containers (devobox, postgres, redis)

## 🛠️ Comandos

### Ambiente de Desenvolvimento

```bash
# Entrar no ambiente de desenvolvimento
devobox shell
# ou
devobox enter  # alias

# Entrar no ambiente com bancos de dados já iniciados
devobox shell --with-dbs

# Subir tudo em background
devobox up

# Parar tudo (libera RAM)
devobox down

# Ver status dos containers
devobox status

# Reconstruir a imagem do zero
devobox rebuild
```

> **💡 Dica:** O comando `shell` mapeia automaticamente seu diretório atual. Se você executar `devobox shell` de dentro de `~/code/projeto1`, você já inicia em `/home/dev/code/projeto1` dentro do container!

### Gerenciamento de Bancos de Dados

```bash
# Iniciar todos os bancos
devobox db start

# Iniciar banco específico
devobox db start postgres
devobox db start redis

# Parar todos os bancos
devobox db stop

# Parar banco específico
devobox db stop postgres

# Reiniciar bancos
devobox db restart [postgres|redis]

# Ver status dos bancos
devobox db status
# ou
devobox db ls  # alias
```

## 📁 Estrutura de Diretórios

### No Repositório (antes da instalação)

```
devobox/
├── bin/
│   └── devobox          → Script CLI
├── config/
│   ├── Containerfile    → Definição da imagem
│   └── Makefile         → Build dos containers
├── docs/
│   └── architecture.png → Diagrama de arquitetura
└── install.sh           → Instalador
```

### Pós-Instalação

```
~/code/                  → Seus projetos (mapeado para /home/dev/code)
~/.config/devobox/       → Configuração instalada
  ├── Containerfile      → Definição da imagem
  ├── Makefile           → Build dos containers
  └── devobox            → Script CLI
~/.local/bin/
  └── devobox            → Symlink para ~/.config/devobox/devobox
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

```bash
# 1. Navegar para seu projeto (no host)
cd ~/code/meu-projeto

# 2. Iniciar ambiente com bancos (já começa no diretório correto)
devobox shell --with-dbs

# 3. Configurar runtimes com Mise (dentro do container)
mise use node@20.11.0
mise use ruby@3.2.2

# 4. Instalar dependências
npm install
bundle install

# 5. Criar database no Postgres
createdb meu_projeto_dev

# 6. Rodar migrações/seeds
rails db:migrate
npm run migrate

# 7. Desenvolver normalmente
rails server
# ou
npm run dev

# 8. Sair do container
exit

# 9. Parar serviços para economizar RAM (opcional)
devobox down
```

## 🏗️ Arquitetura Técnica

### Containers Criados

1. **devobox** - Container principal de desenvolvimento
   - Imagem: `devobox-img` (Arch Linux customizado)
   - Usuário: `dev` (não-root)
   - Network: `--network host` (performance máxima)
   - Volumes:
     - `~/code:/home/dev/code` (bind mount - projetos)
     - `devobox_mise:/home/dev/.local/share/mise` (volume nomeado - ferramentas Mise)
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

### Decisões de Design

**Por que `--network host` (apenas no devobox)?**

- O container **devobox** usa `--network host` para performance máxima
- Postgres e Redis usam **bridge networking** com port mapping (`-p`)
- Isso permite que aplicações no devobox acessem `localhost:5432` e `localhost:6379` diretamente
- Simplifica configuração: `DATABASE_URL=postgresql://dev:devpass@localhost:5432/mydb`
- Elimina latência de bridge networking para o ambiente de desenvolvimento

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

- ✅ **Ferramentas Mise**: Persistem via volume `devobox_mise` (sobrevivem a `rebuild`)
- ✅ **Projetos**: Persistem via bind mount `~/code` (seus arquivos no host)
- ⚠️ **Histórico bash**: NÃO persiste (perdido ao executar `rebuild`)
- ⚠️ **Bancos de dados**: Persistem entre restarts (`down`/`up`), mas são **perdidos** ao executar `rebuild`
- 💡 **Dica**: Para persistência permanente de dados de banco, adicione volumes nomeados no Makefile

## ⚙️ Customização

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

1. Edite `~/.config/devobox/Makefile` e adicione:

```makefile
@echo "🔥 Criando MongoDB (Parado)..."
@podman create --name mongodb \
    -p 27017:27017 \
    docker.io/mongo:7
```

2. Edite `~/.config/devobox/devobox` e atualize:

```bash
DATABASES=("postgres" "redis" "mongodb")
```

3. Reconstrua:

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

```bash
# Dentro do container
mise doctor

# Forçar reinstalação
mise install
```

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
