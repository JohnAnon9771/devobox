# Devobox - Hybrid Development Workstation

**Devobox** is a Rust-based CLI tool designed to create a "Hybrid Workstation" for Linux development. It solves the problem of "Dependency Hell" by isolating development environments in containers while maintaining native performance and ergonomics.

## 📂 Project Overview

*   **Type:** Rust CLI Application
*   **Core Technology:** Rust, Podman, Zellij
*   **Architecture Pattern:** Hub & Spoke (Singleton Workspace + Satellite Services)
*   **Key Philosophy:**
    *   **Host Hygiene:** Keep the host OS clean (only kernel, drivers, UI, editor).
    *   **Native Performance:** Uses Host Networking for the workspace and Bind Mounts for I/O.
    *   **Pet Container:** Treating the dev container as a persistent "pet" rather than ephemeral "cattle".

## 🏗️ Architecture

The project follows a **Clean Architecture** approach with strict separation of concerns:

### Layers
1.  **CLI (`src/cli/`):** Interface layer using `clap`. Handles user input and visual feedback.
    *   `main.rs`: Entry point.
    *   `runtime.rs`: Runtime commands (`shell`, `up`, `down`).
    *   `builder.rs`: Build logic (`init`, `build`).
2.  **Services (`src/services/`):** Business logic and orchestration.
    *   `orchestrator.rs`: Manages service lifecycles and healthchecks.
    *   `container_service.rs`: High-level container operations.
    *   `zellij_service.rs`: Manages terminal sessions.
3.  **Domain (`src/domain/`):** Core entities and interfaces.
    *   `container.rs`: Entities like `Service`, `ContainerSpec`, `ContainerState`.
    *   `traits.rs`: Abstractions like `ContainerRuntime`.
4.  **Infrastructure (`src/infra/`):** Implementation details.
    *   `podman_adapter.rs`: Implementation of `ContainerRuntime` using Podman CLI.
    *   `config.rs`: Configuration parsing (TOML/YAML).

### Networking Strategy
*   **Hub Container (Workspace):** Uses `--network host`. Shares IP with the host for zero-overhead performance and ease of access (localhost is localhost).
*   **Service Containers (DBs, etc.):** Use `bridge` network. Isolated, with explicit port mappings.

## 🛠️ Building & Running

### Prerequisites
*   **Rust:** Stable toolchain (`cargo`).
*   **Podman:** Must be installed and configured.
*   **Zellij:** Recommended for shell integration.

### Development Commands
*   **Build:**
    ```bash
    cargo build
    ```
*   **Run (Debug):**
    ```bash
    cargo run -- [COMMAND]
    # Example: cargo run -- status
    ```
*   **Run (Release):**
    ```bash
    cargo run --release -- [COMMAND]
    ```
*   **Test:**
    ```bash
    cargo test
    ```

### Installation (Local)
To install the binary locally for testing:
```bash
cargo build --release
install -Dm755 ./target/release/devobox ~/.local/bin/devobox
```

## 📝 Configuration Files

*   **`Cargo.toml`**: Project dependencies and metadata.
*   **`config/default_devobox.toml`**: Default global configuration.
*   **`config/default_services.yml`**: Example service definitions.
*   **`config/default_containerfile.dockerfile`**: The base image definition (Debian Bookworm + Mise + Tools).

## 🧩 Key Code Locations

*   **Entry Point:** `src/main.rs` - CLI definition and dispatch.
*   **Podman Interface:** `src/infra/podman_adapter.rs` - Where the actual `podman` commands are constructed and executed. Look here to understand how containers are created/started.
*   **Orchestrator:** `src/services/orchestrator.rs` - Logic for starting services and waiting for healthchecks.
*   **Configuration:** `src/infra/config.rs` - Logic for cascading configuration (Global -> Local -> Project).

## ⚠️ Important Considerations

*   **Synchronous Execution:** The application primarily uses synchronous execution for simplicity, relying on `std::process::Command`.
*   **Linux Focused:** The tool is heavily optimized for Linux (Arch Linux specifically mentioned in docs) and Podman.
*   **Singleton Pattern:** The code assumes a single "Hub" container named `devobox`.
