# Roadmap Técnico e Melhorias Arquiteturais

Este documento registra pontos de evolução técnica identificados para elevar o nível de robustez do `devobox`.

## 1. Migração para Async Runtime (Tokio)
*   **Estado Atual:** O código é síncrono. Se precisarmos subir 5 containers, eles sobem um após o outro. A CLI trava enquanto o comando roda.
*   **O Problema:** Baixa performance em operações em lote e UX travada.
*   **Solução Proposta:**
    *   Adicionar `tokio` como dependência.
    *   Converter a trait `ContainerRuntime` para usar métodos `async fn`.
    *   Utilizar `futures::join_all` para operações paralelas (ex: `devobox up` sobe tudo de uma vez).

## 2. Interação Nativa via API (Remover Shell Out)
*   **Estado Atual:** O adaptador executa `Command::new("podman")` e faz parsing de texto da saída (stdout).
*   **O Problema:** Extremamente frágil. Se o Podman mudar uma mensagem de texto ou a formatação de uma tabela, o `devobox` quebra. Falha também se o sistema estiver em outro idioma.
*   **Solução Proposta:**
    *   Utilizar a Socket API do Podman (compatível com Docker API).
    *   Usar uma crate client (como `podman-api` ou `bollard` configurada para socket unix) para receber respostas tipadas (Structs) em vez de Strings.

## 3. Observabilidade e Logs Estruturados
*   **Estado Atual:** Debug é feito via `println!` ou erros do `anyhow`.
*   **Solução Proposta:**
    *   Integrar a crate `tracing` e `tracing-subscriber`.
    *   Permitir flags como `--verbose` ou `--json-logs` para facilitar o debug de usuários finais sem recompilar.

## 4. Abstração de Configuração
*   **Melhoria:** Permitir que o `devobox` leia configurações existentes de padrões de mercado, como `compose.yaml` ou `.devcontainer/devcontainer.json`, para facilitar a migração de usuários.

## 5. Integração Avançada com Host (Socket Passthrough)
*   **Objetivo:** Zero configuração de credenciais dentro do container.
*   **Estratégia:**
    *   **SSH Agent Forwarding:** Montar o socket `SSH_AUTH_SOCK` do host. Isso permite git clone via SSH sem copiar chaves privadas para dentro do container.
    *   **GPG Agent Forwarding:** Montar socket GPG para permitir assinatura de commits (`git commit -S`) transparente.
    *   **Docker/Podman Socket:** Opcional, para cenários de "Docker-in-Docker".

## 6. Persistência Inteligente (Data Layer)
*   **Problema:** Ao reconstruir a imagem (`devobox rebuild`), ferramentas instaladas manualmente via `mise` ou `apt` são perdidas.
*   **Solução:**
    *   Utilizar **Named Volumes** para diretórios de ferramentas (`/home/dev/.local/share/mise`, `/home/dev/.cargo`).
    *   Isso separa a camada de "Sistema Operacional" (Imagem descartável) da camada de "Ferramentas do Usuário" (Volume persistente).

## 7. Suporte a Aplicações Gráficas (GUI & GPU)
*   **Objetivo:** Rodar editores (Zed, VS Code) e ferramentas visuais diretamente do container com performance nativa no Linux.
*   **Estratégia:**
    *   **Wayland/X11 Sockets:** Detectar e montar sockets gráficos (`/run/user/1000/wayland-0` ou `/tmp/.X11-unix`).
    *   **GPU Passthrough:** Mapear dispositivos `/dev/dri` para permitir aceleração de hardware (essencial para Alacritty, Zed, WezTerm).
    *   **Shared Fonts:** Montar diretórios de fontes do host para que a GUI do container fique visualmente consistente.
