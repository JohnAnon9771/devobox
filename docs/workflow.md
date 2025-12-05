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