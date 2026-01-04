# 🏗️ Arquitetura do Devobox

> Documentação técnica para contribuidores e desenvolvedores curiosos.

---

## Índice

1. [Visão Geral](#visão-geral)
2. [Arquitetura em Camadas](#arquitetura-em-camadas)
3. [Padrão Hub & Spoke](#padrão-hub--spoke)
4. [Sistema de Configuração](#sistema-de-configuração)
5. [Ciclo de Vida dos Containers](#ciclo-de-vida-dos-containers)
6. [Arquitetura de Rede](#arquitetura-de-rede)
7. [Referências de Código](#referências-de-código)
8. [Decisões Arquiteturais](#decisões-arquiteturais)

---

## Visão Geral

### Filosofia de Design

Devobox é um **gerenciador de ambientes de desenvolvimento containerizados** construído em Rust que equilibra:

- **Simplicidade:** Zero-config para casos comuns
- **Performance:** Rede host e bind mounts diretos
- **Higiene:** Containers isolados sem poluir o OS
- **Ergonomia:** CLI intuitivo com feedback visual

### Tech Stack

| Componente           | Tecnologia    | Versão | Propósito                       |
| -------------------- | ------------- | ------ | ------------------------------- |
| **Runtime**          | Rust          | 1.70+  | Performance, segurança de tipos |
| **Container Engine** | Podman        | 4.0+   | Daemonless, rootless            |
| **CLI Framework**    | Clap          | 4.5    | Parsing de argumentos           |
| **Config**           | TOML          | -      | Configuração declarativa        |
| **Base Image**       | Debian Trixie | Stable | OS do container                 |
| **Version Manager**  | Mise          | Latest | Gerenciar runtimes              |
| **Shell Prompt**     | Starship      | Latest | Prompt moderno                  |

### Arquitetura Clean

Devobox segue **Clean Architecture** com separação de responsabilidades:

```
┌─────────────────────────────────┐
│ CLI Layer (src/cli/)            │  ← Interface do usuário
├─────────────────────────────────┤
│ Service Layer (src/services/)   │  ← Lógica de negócio
├─────────────────────────────────┤
│ Domain Layer (src/domain/)      │  ← Entidades core
├─────────────────────────────────┤
│ Infrastructure (src/infra/)     │  ← Podman, config, I/O
└─────────────────────────────────┘
```

---

## Arquitetura em Camadas

### CLI Layer (`src/cli/`)

**Responsabilidades:**

- Parsing de comandos com Clap
- Validação de inputs
- Feedback visual (spinners, progress)
- Orquestração de workflows

**Arquivos principais:**

| Arquivo      | Responsabilidade                           | Linhas |
| ------------ | ------------------------------------------ | ------ |
| `main.rs`    | Entry point, definição de comandos         | ~200   |
| `runtime.rs` | Comandos: shell, up, down, status, project | ~400   |
| `builder.rs` | Comandos: init, build, rebuild             | ~200   |
| `setup.rs`   | Setup inicial de config                    | ~100   |

**Comandos implementados:**

```rust
// src/main.rs
enum Commands {
    Init,           // Setup inicial completo
    Build,          // Reconstrói ambiente
    Shell,          // Abre shell no container
    Up,             // Inicia containers
    Down,           // Para containers
    Status,         // Ver status
    Db,             // Gerenciar bancos (start, stop, restart)
    Service,        // Gerenciar serviços (start, stop, restart)
    Project,        // Gerenciar projetos (list, up, info)
    Cleanup,        // Limpar recursos
}
```

---

### Service Layer (`src/services/`)

**Responsabilidades:**

- Orquestração de serviços
- Healthchecks ativos
- Workflow de start/stop
- Cleanup e manutenção

**Arquivos principais:**

| Arquivo                | Responsabilidade           | Linhas |
| ---------------------- | -------------------------- | ------ |
| `orchestrator.rs`      | Orquestração, healthchecks | ~300   |
| `container_service.rs` | Lifecycle de containers    | ~200   |
| `system_service.rs`    | Build, cleanup             | ~150   |
| `zellij_service.rs`    | Sessões Zellij por projeto | ~180   |

**Fluxo de orquestração:**

```rust
// src/services/orchestrator.rs
pub fn start_services(&self, services: &[Service]) -> Result<()> {
    // 1. Criar containers (se não existem)
    // 2. Iniciar containers em paralelo
    // 3. Aguardar healthchecks
    // 4. Reportar sucesso ou erro
}

pub fn wait_for_healthy(&self, name: &str, retries: u32) -> Result<()> {
    for attempt in 1..=retries {
        match self.runtime.get_container_health(name)? {
            Healthy => return Ok(()),
            _ => sleep(interval),
        }
    }
    Err(timeout_error)
}
```

---

### Domain Layer (`src/domain/`)

**Responsabilidades:**

- Definir entidades core
- Enums e tipos de valor
- Traits e abstrações
- Regras de negócio

**Arquivos principais:**

| Arquivo        | Conteúdo                               | Linhas |
| -------------- | -------------------------------------- | ------ |
| `container.rs` | Service, ContainerSpec, ContainerState | ~150   |
| `traits.rs`    | ContainerRuntime trait                 | ~50    |
| `project.rs`   | Project, ProjectConfig                 | ~100   |

**Entidades principais:**

```rust
// src/domain/container.rs
pub enum ContainerState {
    Running,
    Stopped,
    NotCreated,
}

pub enum ServiceKind {
    Database,   // Postgres, MySQL, MongoDB
    Generic,    // Redis, Mailhog, etc (default)
}

pub struct Service {
    pub name: String,
    pub image: String,
    pub kind: ServiceKind,
    pub ports: Vec<String>,
    pub env: Vec<String>,
    pub healthcheck_command: Option<String>,
    // ...
}

pub struct ContainerSpec<'a> {
    pub name: &'a str,
    pub image: &'a str,
    pub network: Option<&'a str>,     // "host" ou "bridge"
    pub userns: Option<&'a str>,      // "keep-id"
    pub security_opt: Option<&'a str>,// "label=disable"
    // ...
}
```

```rust
// src/domain/traits.rs
pub trait ContainerRuntime {
    fn create_container(&self, spec: &ContainerSpec) -> Result<()>;
    fn start_container(&self, name: &str) -> Result<()>;
    fn stop_container(&self, name: &str) -> Result<()>;
    fn get_container_state(&self, name: &str) -> Result<ContainerState>;
    fn get_container_health(&self, name: &str) -> Result<ContainerHealthStatus>;
    fn exec_shell(&self, name: &str, workdir: Option<&str>) -> Result<()>;
}
```

---

### Infrastructure Layer (`src/infra/`)

**Responsabilidades:**

- Implementar traits de domínio
- Integração com Podman CLI
- Parsing de configuração
- I/O com filesystem

**Arquivos principais:**

| Arquivo                | Responsabilidade              | Linhas |
| ---------------------- | ----------------------------- | ------ |
| `podman_adapter.rs`    | Implementa ContainerRuntime   | ~500   |
| `config.rs`            | Loading e validação de config | ~600   |
| `project_discovery.rs` | Descoberta de projetos        | ~150   |

**Implementação Podman:**

```rust
// src/infra/podman_adapter.rs
pub struct PodmanAdapter;

impl ContainerRuntime for PodmanAdapter {
    fn create_container(&self, spec: &ContainerSpec) -> Result<()> {
        let mut cmd = Command::new("podman");
        cmd.arg("create")
           .arg("--name").arg(spec.name)
           .arg("--image").arg(spec.image);

        if let Some(network) = spec.network {
            cmd.arg("--network").arg(network);
        }

        if let Some(userns) = spec.userns {
            cmd.arg("--userns").arg(userns);  // "keep-id"
        }

        for port in spec.ports {
            cmd.arg("-p").arg(port);
        }

        // Executa comando
        let output = cmd.output()?;
        // Valida resultado
    }

    fn get_container_health(&self, name: &str) -> Result<ContainerHealthStatus> {
        let output = Command::new("podman")
            .args(&["inspect", "--format", "{{.State.Health.Status}}", name])
            .output()?;

        match output.stdout.trim() {
            "healthy" => Ok(Healthy),
            "unhealthy" => Ok(Unhealthy),
            "starting" => Ok(Starting),
            _ => Ok(Unknown),
        }
    }
}
```

---

## Padrão Hub & Spoke

### Conceito

Arquitetura inspirada em redes: **Hub (cubo) + Spokes (raios)**.

```
                ┌──────────────┐
                │   Host PC    │
                └──────┬───────┘
                       │
              ┌────────▼────────┐
              │   HUB (devobox) │  ← Workspace de desenvolvimento
              │   - Code        │  ← Network: host
              │   - Tools       │  ← Singleton
              │   - Shell       │
              └────────┬────────┘
                       │
      ┌────────────────┼────────────────┐
      │                │                │
  ┌───▼───┐        ┌───▼───┐       ┌───▼───┐
  │  PG   │        │ Redis │       │ Mailh │  ← Spokes (satélites)
  │ :5432 │        │ :6379 │       │ :8025 │  ← Network: bridge
  └───────┘        └───────┘       └───────┘  ← Port mapping
```

### Hub Container

**Características:**

- Nome: `devobox` (fixo, singleton)
- Network: `--network host`
- User namespace: `--userns=keep-id`
- Volumes: `~/code:/home/dev/code` (bind mount RW)
- Persistente (não é recriado)

**Criação:**

```rust
// src/cli/builder.rs
let hub_spec = ContainerSpec {
    name: "devobox",
    image: &image_name,
    network: Some("host"),
    userns: Some("keep-id"),
    security_opt: Some("label=disable"),
    workdir: Some("/home/dev"),
    volumes: &volumes,  // ["/home/user/code:/home/dev/code"]
    // ...
};
```

**Por que host network?**

- Zero overhead de NAT
- `localhost:3000` funciona direto
- Simplicidade de port access

**Lifecycle:**

```rust
// src/cli/runtime.rs
pub fn shell() -> Result<()> {
    let state = runtime.get_container_state("devobox")?;

    match state {
        NotCreated => initialize(),  // Cria pela primeira vez
        Stopped => start(),           // Apenas inicia
        Running => exec_shell(),      // Reusa! (Singleton)
    }
}
```

---

### Spoke Containers

**Características:**

- Network: `bridge` (padrão)
- Port mapping: explícito (`-p 5432:5432`)
- Isolados do Hub
- Lifecycle: managed pelo orchestrator

**Criação:**

```rust
// src/cli/builder.rs
for service in &config.services {
    let spec = service.to_container_spec();
    runtime.create_container(&spec)?;
}
```

**Por que bridge network?**

- Isolamento de serviços
- Controle explícito de portas
- Segurança por padrão

---

## Sistema de Configuração

### Cascata de Resolução

```
1. Defaults (hardcoded)
   ↓ merge
2. Global (~/.config/devobox/devobox.toml)
   ↓ merge
3. Local (./devobox.toml)
   ↓
Final Config
```

**Implementação:**

```rust
// src/infra/config.rs
pub fn load_app_config(local_path: Option<&Path>) -> Result<AppConfig> {
    // 1. Carregar config global
    let mut config = load_global_config()?;

    // 2. Se existe local, merge
    if let Some(path) = local_path {
        let local = load_local_config(path)?;
        config = merge_configs(config, local);
    }

    // 3. Validar e retornar
    validate_config(&config)?;
    Ok(config)
}
```

### Validação de Serviços

```rust
// src/infra/config.rs
fn validate_service_name(name: &str) -> Result<()> {
    // Regras:
    // - Deve começar com alfanumérico
    // - Pode conter: alfanumérico, underscore, ponto, hífen
    // - Não pode ser vazio

    let re = Regex::new(r"^[a-zA-Z0-9][a-zA-Z0-9_.-]*$")?;
    if !re.is_match(name) {
        return Err(ValidationError);
    }
    Ok(())
}

fn check_duplicate_services(services: &[Service]) -> Result<()> {
    let mut seen = HashSet::new();
    for service in services {
        if !seen.insert(&service.name) {
            return Err(DuplicateServiceError(service.name.clone()));
        }
    }
    Ok(())
}
```

### Resolução de Dependências

```rust
// src/infra/config.rs
pub fn resolve_all_services(
    project_path: &Path,
    visited: &mut HashSet<PathBuf>
) -> Result<Vec<Service>> {
    // Previne ciclos
    if visited.contains(project_path) {
        return Ok(vec![]);
    }
    visited.insert(project_path.to_path_buf());

    let config = load_config(project_path)?;
    let mut services = config.services.clone();

    // Recursivamente carregar dependências
    for include_path in &config.dependencies.include_projects {
        let dep_path = resolve_path(project_path, include_path)?;
        let dep_services = resolve_all_services(&dep_path, visited)?;
        services.extend(dep_services);
    }

    Ok(services)
}
```

---

## Ciclo de Vida dos Containers

### Máquina de Estados

```
NotCreated ──create──> Stopped ──start──> Running
     ↑                    │                  │
     │                    │                  │
     └────────── remove ──┴──────────────────┘
```

### Padrão Singleton (Hub)

```rust
// src/cli/runtime.rs
pub fn ensure_hub_running() -> Result<()> {
    let state = runtime.get_container_state("devobox")?;

    match state {
        NotCreated => {
            // Primeira vez: criar + iniciar
            runtime.create_container(&hub_spec)?;
            runtime.start_container("devobox")?;
        }
        Stopped => {
            // Já existe: apenas iniciar
            runtime.start_container("devobox")?;
        }
        Running => {
            // Já rodando: nada a fazer
        }
    }

    Ok(())
}
```

**Benefício:** Container é reutilizado, não recriado. Estado preservado.

### Orquestração de Serviços

```rust
// src/services/orchestrator.rs
pub fn start_services(&self, services: &[Service]) -> Result<()> {
    for service in services {
        // 1. Ensure container exists
        let state = self.runtime.get_container_state(&service.name)?;
        if state == NotCreated {
            self.runtime.create_container(&service.to_spec())?;
        }

        // 2. Start if not running
        if state != Running {
            self.runtime.start_container(&service.name)?;
        }

        // 3. Wait for healthcheck (if defined)
        if service.healthcheck_command.is_some() {
            self.wait_for_healthy(&service.name, service.healthcheck_retries)?;
        }
    }

    Ok(())
}
```

---

## Arquitetura de Rede

### Estratégia Híbrida

| Container    | Network | Razão                     |
| ------------ | ------- | ------------------------- |
| **Hub**      | Host    | Performance, simplicidade |
| **Services** | Bridge  | Isolamento, segurança     |

### Host Network (Hub)

```bash
# Comando Podman gerado
podman create --network host --name devobox ...
```

**Implicações:**

- Container compartilha IP com host
- `localhost:3000` no container = `localhost:3000` no host
- Sem NAT, sem overhead
- Menos isolamento de rede

**Trade-off aceito:** Em desenvolvimento local, performance > isolamento extremo.

---

### Bridge Network (Services)

```bash
# Comando Podman gerado
podman create --name pg -p 5432:5432 postgres:16
```

**Implicações:**

- Container tem IP próprio na bridge
- Port mapping explícito (`-p HOST:CONTAINER`)
- Isolado do Hub
- Mais seguro por padrão

---

### User Namespace Mapping

```bash
--userns=keep-id
```

**O que faz:**

- Mapeia UID do container → UID do host
- Exemplo: User `dev` (UID 1000) no container = User `joao` (UID 1000) no host
- Arquivos criados no container pertencem a você no host

**Sem keep-id:**

```
# Arquivo criado no container
-rw-r--r-- 1 root root 245 ... arquivo.rb

# Precisa de sudo para editar
```

**Com keep-id:**

```
# Arquivo criado no container
-rw-r--r-- 1 joao joao 245 ... arquivo.rb

# Você é o dono!
```

---

## Referências de Código

### Arquivos-Chave

| Arquivo                              | Propósito                                   | LoC  |
| ------------------------------------ | ------------------------------------------- | ---- |
| **`src/main.rs`**                    | Entry point, CLI definitions                | ~200 |
| **`src/cli/runtime.rs`**             | Runtime commands (shell, up, down, project) | ~400 |
| **`src/cli/builder.rs`**             | Build commands (init, build)                | ~200 |
| **`src/services/orchestrator.rs`**   | Service orchestration, healthchecks         | ~300 |
| **`src/services/zellij_service.rs`** | Zellij session management                   | ~180 |
| **`src/domain/container.rs`**        | Core entities                               | ~150 |
| **`src/domain/project.rs`**          | Project entities                            | ~100 |
| **`src/infra/podman_adapter.rs`**    | Podman CLI integration                      | ~500 |
| **`src/infra/config.rs`**            | Config loading, validation                  | ~600 |
| **`src/infra/project_discovery.rs`** | Project discovery in ~/code                 | ~150 |

### Funções Importantes

**Container creation:**

```rust
// src/infra/podman_adapter.rs:create_container()
impl ContainerRuntime for PodmanAdapter {
    fn create_container(&self, spec: &ContainerSpec) -> Result<()> { ... }
}
```

**Service startup:**

```rust
// src/services/orchestrator.rs:start_services()
pub fn start_services(&self, services: &[Service]) -> Result<()> { ... }
```

**Config loading:**

```rust
// src/infra/config.rs:load_app_config()
pub fn load_app_config(local_path: Option<&Path>) -> Result<AppConfig> { ... }
```

**Project discovery:**

```rust
// src/infra/project_discovery.rs:discover_projects()
pub fn discover_projects(code_dir: &Path) -> Result<Vec<Project>> { ... }
```

**Healthcheck waiting:**

```rust
// src/services/orchestrator.rs:wait_for_healthy()
pub fn wait_for_healthy(&self, name: &str, retries: u32) -> Result<()> { ... }
```

---

## Decisões Arquiteturais

### Por que Podman em vez de Docker?

**Razões:**

1. **Daemonless:** Não precisa de daemon em background
2. **Rootless:** Roda sem privilégios de root (mais seguro)
3. **OCI-compliant:** Compatível com padrão aberto
4. **CLI compatível:** Comandos idênticos ao Docker (`podman run` ≈ `docker run`)
5. **User namespaces:** Melhor suporte nativo

**Trade-off:** Menos comum que Docker, mas superior para uso local.

---

### Por que Rust?

**Razões:**

1. **Performance:** Binário nativo, startup rápido
2. **Segurança:** Sistema de tipos evita bugs comuns
3. **Cross-platform:** Compila para Linux/Mac/Windows
4. **Ecossistema:** Cargo, crates.io, Clap, serde

**Trade-off:** Curva de aprendizado mais íngreme que Go ou Python.

---

### Por que Host Network para Hub?

**Razões:**

1. **Performance:** Zero overhead de NAT
2. **Simplicidade:** Não precisa mapear portas
3. **Compatibilidade:** Apps funcionam como desenvolvimento nativo

**Trade-off:** Menos isolamento de rede. Aceitável para dev local.

---

### Por que Bridge Network para Services?

**Razões:**

1. **Isolamento:** Bancos não poluem namespace do Hub
2. **Controle:** Port mapping explícito
3. **Segurança:** Default deny

**Trade-off:** Pequeno overhead de NAT. Aceitável para serviços auxiliares.

---

### Por que TOML (e não apenas YAML)?

**Razões:**

1. **Type-safe:** Mais fácil parsear com serde
2. **Menos ambíguo:** YAML tem muitas pegadinhas (indentação, yes/no como boolean)
3. **Familiar:** Usado por Cargo, Rust toolchain

**Suporte YAML:** Mantido para compatibilidade com services legados.

---

### Por que Clean Architecture?

**Razões:**

1. **Testabilidade:** Camadas independentes são fáceis de testar
2. **Manutenibilidade:** Mudanças de infraestrutura não afetam domínio
3. **Flexibilidade:** Fácil trocar Podman por Docker se necessário

**Trade-off:** Mais código (traits, abstrações). Vale a pena para projeto desse porte.

---

## Contribuindo

### Setup de Desenvolvimento

```bash
# Clonar repo
git clone https://github.com/JohnAnon9771/devobox.git
cd devobox

# Build
cargo build

# Run
cargo run -- status

# Test
cargo test

# Instalar localmente
cargo build --release
install -Dm755 ./target/release/devobox ~/.local/bin/devobox
```

### Estrutura de Branches

- `main` - estável, releases
- `develop` - desenvolvimento ativo
- `feature/*` - novas features
- `fix/*` - bug fixes

### Checklist de PR

- [ ] Tests passam (`cargo test`)
- [ ] Código formatado (`cargo fmt`)
- [ ] Linter passa (`cargo clippy`)
- [ ] Documentação atualizada
- [ ] Changelog atualizado

---

**Para mais informações:**

- [Guia Completo](GUIDE.md) - Conceitos e workflows
- [Cookbook](COOKBOOK.md) - Receitas práticas
- [Getting Started](../getting-started/) - Tutorial inicial
