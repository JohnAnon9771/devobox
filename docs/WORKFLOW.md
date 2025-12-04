# 🧭 O Workflow do Devobox: Guia de Bolso

> "Pare de configurar. Comece a codar."

Este guia explica **como se trabalha** com o Devobox no dia a dia. Esqueça a terminologia técnica de containers por um momento. Vamos focar no fluxo.

---

## O Modelo Mental: "O Escritório Virtual"

Imagine que seu computador é um prédio vazio.
O **Devobox** é o seu **Escritório Mobiliado**.

*   **O Hub (Seu Terminal):** É a sua cadeira e mesa. Tem seu computador, seu editor de texto, suas ferramentas (git, vim, node, ruby). Você senta aqui para trabalhar.
*   **Os Serviços (Infraestrutura):** São as máquinas de café, servidores e arquivos que ficam na sala ao lado. Eles precisam estar ligados para você trabalhar, mas você não fica "mexendo" neles o tempo todo.

---

## O Ciclo de Vida Diário

### 1. A Chegada (`Host -> Container`)

Você liga seu computador (Host). Você abre seu terminal.
Você quer começar a trabalhar no projeto **"Minha Loja"**.

**Comando:**
```bash
# No seu terminal normal (Host)
cd ~/code/minha-loja
devobox up
```

**O que acontece (A Mágica):**
1.  O Devobox lê o crachá do projeto (`devobox.toml`).
2.  Ele vê que a Loja precisa de **Postgres** e **Redis**.
3.  Ele vai na "sala ao lado" e liga o Postgres e o Redis.
4.  Ele espera eles esquentarem (Healthcheck: "Database ready!").
5.  **Ele te teletransporta para sua mesa (O Hub).**

Agora seu terminal mudou. Você não está mais no Host. Você está no **Devobox**.

### 2. O Trabalho (`Inside the Box`)

Agora você está "sentado na mesa".

**Comando:**
```bash
# Dentro do Devobox (Hub)
npm install
npm run dev
```

**O que acontece:**
*   Seu app sobe na porta 3000.
*   Ele conecta no Postgres (que o Devobox ligou pra você) via `localhost:5432`.
*   Você abre o navegador no seu Host e acessa `localhost:3000`.

**Importante:** Você está em um ambiente Linux perfeito, isolado, com as versões certas de Node/Ruby instaladas automaticamente.

### 3. O Fim do Dia (`Teardown`)

Você terminou. Quer fechar tudo para jogar um jogo ou economizar bateria.

**Comando:**
```bash
# Dentro do Devobox
exit
```

Se você usou `devobox up --auto-stop`:
*   Assim que você sai, o Devobox "apaga a luz".
*   Ele desliga o Postgres e o Redis automaticamente.
*   Seu computador Host volta a ficar 100% limpo e leve.

---

## Multi-Apps: Duas Maneiras de Trabalhar

Quando você tem múltiplos projetos (ex: Frontend e Backend), existem dois jeitos de trabalhar. Escolha a analogia que combina com seu momento:

### Modo 1: O Chef de Cozinha (Multitarefa)
*Quando você precisa editar o código de AMBOS os projetos ao mesmo tempo.*

*   **Cenário:** Você é um Chef (O Hub).
*   **Ação:** Você tem várias panelas no fogão.
    *   **Panela 1 (Terminal 1):** Frontend (`npm run dev`).
    *   **Panela 2 (Terminal 2):** Backend (`rails s`).
*   **Como é:** Você pula de um terminal para o outro. Você vê os logs de erro dos dois. Você para e reinicia os dois.
*   **Comando:** Abra duas abas e rode `devobox up` em cada pasta.

### Modo 2: O Pedido de Delivery (Dependência)
*Quando você está focado no Frontend e só precisa que o Backend "funcione".*

*   **Cenário:** Você está jantando em casa (Frontend).
*   **Ação:** Você quer sushi (Backend), mas não quer fazer o sushi.
*   **Como é:** O Devobox "pede" o Backend pra você. Ele chega numa caixa fechada (Container Isolado/Service).
    *   Você não vê a bagunça da cozinha do restaurante.
    *   Você não vê os logs do Backend (a menos que procure).
    *   Ele apenas "está lá" servindo dados na porta 3000.
*   **Comando:** Configure `include_projects` no `devobox.toml` do Frontend.

---

## Cenários Comuns (FAQ)

### "Quero trabalhar em dois projetos ao mesmo tempo."

Sem problemas. Abra duas abas no seu terminal (Host).

*   **Aba 1:** `cd ~/code/frontend` -> `devobox up`
*   **Aba 2:** `cd ~/code/backend` -> `devobox up`

Você terá dois terminais abertos no mesmo "Escritório" (Hub), mas em pastas diferentes.
O Devobox garantiu que o banco do Front e o banco do Back estão ligados.

### "Onde eu rodo os comandos?"

*   `devobox ...`: Comandos de **Gerenciamento**. Roda no **HOST**.
    *   *"Devobox, ligue o banco!"* (`db start`)
    *   *"Devobox, me coloque no shell!"* (`shell`)
    *   *"Devobox, limpe o lixo!"* (`cleanup`)

*   `git`, `npm`, `cargo`, `rails`: Comandos de **Trabalho**. Roda **DENTRO** (no shell do Devobox).

### "Preciso rodar uma API de terceiros."

Você está no Frontend e precisa que a API de Pagamentos esteja rodando, mas não quer abrir uma aba só pra ela.

1.  No `devobox.toml` do Frontend, adicione: `include_projects = ["../api-pagamentos"]`.
2.  No Host: `devobox up`.
3.  O Devobox liga o Postgres, o Redis **E** o container da API de Pagamentos.
4.  Você trabalha no Frontend. A API roda silenciosa em background.

---

## Resumo Visual

| Ação | Onde Roda? | Comando | Analogia |
| :--- | :---: | :--- | :--- |
| **Preparar o terreno** | Host | `devobox up` | Acender as luzes e ligar os servidores. |
| **Codar / Rodar App** | Hub (Container) | `npm run dev` | Sentar na mesa e trabalhar. |
| **Infraestrutura** | Spoke (Background) | (Automático) | O ar condicionado funcionando ao fundo. |
| **Dependência** | Spoke (Background) | (Automático) | O estagiário trabalhando na outra sala. |
