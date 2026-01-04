---
# https://vitepress.dev/reference/default-theme-home-page
layout: home

hero:
  name: "Devobox"
  text: "Estação de Trabalho Híbrida"
  tagline: Desenvolva no Linux sem poluir seu sistema, sem perder performance e sem reinventar o ambiente a cada projeto.
  actions:
    - theme: brand
      text: Guia de Início
      link: /getting-started/
    - theme: alt
      text: Ver no GitHub
      link: https://github.com/JohnAnon9771/devobox

features:
  - title: Higiene Absoluta
    details: Isole 100% das runtimes. Seu host fica limpo apenas com Kernel, Drivers e GUI. Updates do sistema nunca mais quebrarão seu ambiente.
    icon: 🧹
  - title: Performance Nativa
    details: Zero overhead de VM. I/O na velocidade do SSD e rede host 100% nativa. Esqueça a lentidão do Docker Desktop.
    icon: ⚡
  - title: Filosofia "Pet"
    details: Um container persistente que lembra do seu histórico e ferramentas. Use como um segundo computador que nunca quebra.
    icon: 🐕
  - title: Orquestração Inteligente
    details: Suba serviços na ordem certa com healthchecks ativos. Gerencie dependências entre projetos automaticamente.
    icon: 🧠
---

# Instalação Rápida

## Requisitos
- **Podman** instalado
- **Linux** (otimizado para Arch, funciona em Ubuntu/Fedora)

## Instalar via Release

```bash
curl -L https://github.com/JohnAnon9771/devobox/releases/latest/download/x86_64-unknown-linux-gnu.tar.gz -o devobox.tar.gz
tar -xzf devobox.tar.gz
chmod +x devobox
mv devobox ~/.local/bin/devobox
devobox init
```

[Ver guia de instalação detalhado](./getting-started/)

---

# Por que Devobox?

> "Desenvolva sem poluir seu sistema, sem perder performance e sem reinventar o ambiente a cada projeto."

Devobox cria um **segundo computador dentro do seu Linux** — isolado, persistente e rápido. É a união perfeita entre a higiene dos containers e a conveniência do desenvolvimento local.

[Leia mais sobre a arquitetura](./architecture/)
