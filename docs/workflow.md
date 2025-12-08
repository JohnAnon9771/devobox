# 🧭 Workflow Devobox com Zellij

> **"Seu terminal agora é imortal."**

Este guia explica como utilizar o **Zellij** (multiplexador de terminal nativo do Devobox) para transformar seu fluxo de trabalho de efêmero para persistente.

---

## 🚀 Por que Zellij?

O Devobox adota o Zellij como gerenciador de janelas padrão por três motivos:

1.  **Persistência (Pet Container):** Se você fechar o terminal do seu Host, seus processos (servidores, builds, editores) continuam rodando.
2.  **Organização:** Permite ter múltiplas abas e painéis para monitorar diferentes serviços sem sair do contexto.
3.  **Modernidade:** Escrito em Rust, rápido e com interface amigável (barra de status com atalhos).

---

## 🎹 Cheatsheet Básico

O Zellij usa a tecla `Alt` (Option no Mac) como modificador principal.

| Ação | Atalho |
| :--- | :--- |
| **Novo Painel** (Split) | `Alt + n` |
| **Navegar Painéis** | `Alt + Setas` |
| **Nova Aba** | `Alt + t` |
| **Navegar Abas** | `Alt + n` (Próxima) / `Alt + p` (Anterior) |
| **Detach** (Sair mantendo rodando) | `Ctrl + o`, depois `d` |
| **Scroll** | `Ctrl + s` (Use setas ou `PageUp/Down`) |
| **Sair do Scroll** | `Esc` |

> **Dica:** Olhe sempre para a barra verde no rodapé do terminal. Ela mostra os atalhos disponíveis no modo atual.

---

## 🔄 Fluxo Diário Recomendado

### 1. Iniciando o Dia
Quando você roda `devobox` ou `devobox shell`, ele automaticamente:
- Verifica se já existe uma sessão chamada `devobox`.
- Se existir: **Conecta você a ela** (você vê exatamente o que deixou ontem).
- Se não existir: **Cria uma nova**.

```bash
# No seu terminal (Kitty, iTerm, Alacritty)
devobox
# BOOM! Você está dentro do Zellij no Linux.
```

### 2. Trabalhando (Multitarefa)
Em vez de abrir múltiplas janelas no seu terminal do Host:

1.  **Aba 1 (Editor):** Abra seu Neovim/Helix/Emacs.
2.  **Aba 2 (Servidor):** Pressione `Alt + t`, renomeie (`Alt + r`) para "Server" e rode `cargo run` ou `npm start`.
3.  **Aba 3 (Git):** Pressione `Alt + t` e use para comandos git rápidos.

### 3. Conectando a Microsserviços (Satellites)
Se você está rodando em modo orquestração (com apps dependentes rodando ao lado), use painéis para monitorá-los:

**Cenário:** Você está no `frontend` e quer ver logs da `api` (que está rodando como serviço satélite).

1.  Crie um novo painel: `Alt + n` (ou Aba `Alt + t`).
2.  Veja os logs do container vizinho:
    ```bash
    podman logs -f minha-api
    ```
3.  Precisa rodar uma migration na API?
    ```bash
    podman exec -it minha-api bash
    # Agora você está dentro do container da API!
    ```

### 4. Encerrando o Dia (O Pulo do Gato)
Não feche seus servidores. Não mate seus builds.

Apenas faça **Detach**:
- Pressione `Ctrl + o` (Entra no modo "Session").
- Pressione `d` (Detach).

Você voltará para o shell do seu Host (Mac/Linux). O Devobox continua rodando em background, consumindo recursos mínimos se estiver ocioso, mas pronto para ação.

Quando voltar amanhã e rodar `devobox`, tudo estará lá.

---

## 📁 Trabalhando com Múltiplos Projetos (Novo em v0.5.0+)

O Devobox agora suporta **sessões Zellij dedicadas por projeto**, permitindo que você trabalhe em múltiplos projetos simultaneamente sem misturar contextos.

### O Conceito

Cada projeto em `~/code` pode ter sua própria:
- Sessão Zellij isolada
- Conjunto de serviços (databases, caches, etc.)
- Variáveis de ambiente específicas

### Fluxo de Trabalho

#### 1. Descobrir Projetos Disponíveis
```bash
# Dentro do devobox
devobox project list
```

Mostra todos os projetos em `~/code` que têm um arquivo `devobox.toml`.

#### 2. Ativar um Projeto
```bash
# Dentro do devobox
devobox project up meu-frontend
```

Isso vai:
1. ✅ Iniciar os serviços específicos do projeto
2. ✅ Carregar as variáveis de ambiente configuradas
3. ✅ Criar/anexar uma sessão Zellij dedicada (`devobox-meu-frontend`)
4. ✅ Mudar para o diretório do projeto

#### 3. Trabalhar no Projeto
Agora você está dentro de uma **sessão Zellij isolada** para esse projeto:
- Abra seu editor: `nvim .`
- Rode o servidor: `npm start`
- Execute testes: `cargo test`

**Tudo fica dentro da sessão do projeto!**

#### 4. Trocar de Projeto
Quer trabalhar em outro projeto? Simples:

1. Saia da sessão Zellij atual: `Ctrl + o`, depois `d`
2. Você volta ao shell principal do devobox
3. Ative outro projeto: `devobox project up meu-backend`

Agora você tem **duas sessões Zellij rodando em paralelo**:
- `devobox-meu-frontend` (com seu servidor Next.js rodando)
- `devobox-meu-backend` (com sua API Rails rodando)

#### 5. Ver Contexto Atual
```bash
devobox project info
```

Mostra:
- Em qual contexto você está (Host ou Container)
- Qual projeto está ativo
- Sessões Zellij em execução

### Estrutura de um Projeto

```bash
~/code/meu-projeto/
├── devobox.toml           # Configuração do projeto
├── services.yml           # Serviços específicos (opcional)
└── src/                   # Código do projeto
```

**Exemplo de `devobox.toml`:**
```toml
[project]
env = ["NODE_ENV=development", "API_URL=http://localhost:3001"]

[dependencies]
services_yml = "services.yml"
```

**Exemplo de `services.yml`:**
```yaml
services:
  - name: app-db
    type: database
    image: postgres:16
    ports: ["5433:5432"]
    env:
      - POSTGRES_PASSWORD=dev
      - POSTGRES_DB=myapp
```

### Exemplo Prático: Frontend + Backend

**Projeto Frontend (`~/code/frontend/devobox.toml`):**
```toml
[project]
env = ["NEXT_PUBLIC_API_URL=http://localhost:3001"]

[dependencies]
services_yml = "services.yml"
```

**Projeto Backend (`~/code/backend/devobox.toml`):**
```toml
[project]
env = ["RAILS_ENV=development", "DATABASE_URL=postgresql://localhost/myapp"]

[dependencies]
services_yml = "services.yml"
```

**Fluxo de trabalho:**
```bash
# Dentro do devobox
devobox project up backend
# Agora você está no backend, com Postgres rodando
# rails server

# Detach: Ctrl+o, d
devobox project up frontend
# Agora você está no frontend, com Next.js
# npm run dev

# Quer voltar ao backend?
# Ctrl+o, d (sai do frontend)
devobox project up backend
# Volta à sessão do backend (servidor Rails ainda rodando!)
```

### Vantagens

✅ **Isolamento**: Cada projeto tem sua sessão Zellij separada
✅ **Persistência**: Servidores continuam rodando quando você troca de projeto
✅ **Organização**: Não precisa lembrar em qual aba está cada projeto
✅ **Eficiência**: Serviços compartilhados (ex: Redis) não duplicam

---

## 💡 Dicas Avançadas

### Sincronização com Clipboard
O Zellij gerencia a área de transferência. Para copiar texto:
1.  Selecione com o mouse (se seu terminal suportar).
2.  Ou entre em modo Scroll (`Ctrl + s`) e selecione.

### Layouts (Futuro)
O Zellij suporta layouts pré-definidos. No futuro, você poderá criar um arquivo `layout.kdl` na raiz do seu projeto para que o Devobox abra já com as abas e comandos certos para aquele projeto.

---

**Resumo:**
- **Host Terminal:** Apenas uma janela/aba.
- **Devobox (Zellij):** Onde a mágica acontece (Abas, Panes, Persistência).