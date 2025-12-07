# 🧩 Orquestração de Microsserviços com Devobox

Uma das capacidades mais poderosas do Devobox é tratar **outros projetos (Apps)** como se fossem serviços de infraestrutura (como um Redis ou Postgres).

Isso permite que você desenvolva no **Projeto A** enquanto o **Projeto B** (uma API de dependência) roda automaticamente ao lado, em seu próprio container isolado.

## O Padrão "App as a Service"

Em vez de abrir dois terminais e rodar `rails s` em ambos, você define o Projeto B como um serviço no `services.yml` do Projeto A.

### Exemplo Prático: Frontend (Vue) consumindo Backend (Rails)

Imagine que você está trabalhando no frontend (`~/code/my-frontend`) e precisa que a API (`~/code/my-api`) esteja rodando.

#### 1. Estrutura de Pastas
```
~/code/
├── my-frontend/   <-- Você está aqui (Pet Container)
│   └── services.yml
└── my-api/        <-- Dependência (Vai rodar como container auxiliar)
```

#### 2. Configuração (`~/code/my-frontend/services.yml`)

Adicione uma entrada `generic` que usa a imagem do Devobox para rodar o código da API.

```yaml
services:
  # Infraestrutura básica (ex: Banco da API)
  - name: postgres
    type: database
    image: docker.io/postgres:16
    ports: ["5432:5432"]
    env: ["POSTGRES_PASSWORD=dev"]

  # --- A Mágica Acontece Aqui ---
  - name: backend-api
    type: generic
    # Usa a imagem base do Devobox para ter acesso a Ruby, Node, Mise, etc.
    image: devobox-img:latest
    
    # Monta o código da API dentro deste container
    volumes:
      - "${HOME}/code/my-api:/app"
    
    # Define onde os comandos vão rodar
    # ATENÇÃO: A propriedade 'working_dir' ainda não é suportada nativamente no YAML simplificado,
    # então fazemos 'cd' no comando.
    
    # Comando de inicialização
    # 1. Garante que as ferramentas (Ruby/Node) estão instaladas
    # 2. Instala dependências (gems)
    # 3. Sobe o servidor em uma porta ESPECÍFICA (diferente do frontend)
    command: >
      /bin/bash -c "
      cd /app &&
      mise install &&
      bundle check || bundle install &&
      bin/rails s -p 3001 -b 0.0.0.0
      "
    
    # Expõe a porta 3001 para você acessar via localhost:3001
    ports: ["3001:3001"]
    
    # Healthcheck: O Devobox só libera seu shell quando a API responder
    healthcheck_command: "curl -f http://localhost:3001/up"
    healthcheck_interval: "5s"
    healthcheck_timeout: "2s"
    healthcheck_retries: 10
```

### Como Funciona

Ao rodar `devobox up` na pasta `my-frontend`:

1.  **Postgres** sobe.
2.  **backend-api** sobe em paralelo.
    *   Ele instala as versões corretas do Ruby/Node definidas no `.tool-versions` da API.
    *   Roda `bundle install` automaticamente.
    *   Inicia o servidor na porta 3001.
3.  O Devobox aguarda o `curl` na porta 3001 dar sucesso.
4.  Seu shell abre no `my-frontend`.

Você programa no Frontend apontando para `http://localhost:3001` como se a API estivesse rodando nativamente ou em produção.

### Dicas Importantes

1.  **Conflito de Portas:** Sempre suba os apps dependentes em portas alternativas (3001, 3002, 8081...), deixando as portas padrão (3000, 8080) livres para o projeto que você está editando ativamente.
2.  **Binding:** Certifique-se de que o servidor da dependência faça bind em `0.0.0.0` e não apenas `127.0.0.1` (localhost), caso contrário o port mapping do Docker não funcionará.
3.  **Logs:** Se a API falhar, você pode ver o que aconteceu com:
    ```bash
    podman logs backend-api -f
    ```
