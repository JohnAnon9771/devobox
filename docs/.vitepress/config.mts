import { defineConfig } from 'vitepress'

// https://vitepress.dev/reference/site-config
export default defineConfig({
  title: "Devobox",
  description: "Estação de Trabalho Híbrida para Desenvolvimento no Linux",
  lang: 'pt-BR',
  cleanUrls: true,
  lastUpdated: true,

  head: [
    ['link', { rel: 'icon', href: '/favicon.ico' }],
    ['meta', { name: 'theme-color', content: '#3eaf7c' }],
  ],

  themeConfig: {
    // https://vitepress.dev/reference/default-theme-config
    logo: '/logo.svg',
    siteTitle: 'Devobox',

    nav: [
      { text: 'Início', link: '/' },
      { text: 'Início Rápido', link: '/getting-started/' },
      { text: 'Guia', link: '/guide/' },
      { text: 'Cookbook', link: '/cookbook/' },
      { text: 'Arquitetura', link: '/architecture/' },
    ],

    sidebar: [
      {
        text: 'Começando',
        items: [
          { text: 'O que é Devobox?', link: '/#por-que-devobox' },
          { text: 'Início Rápido', link: '/getting-started/' },
          { text: 'Instalação', link: '/getting-started/#instalacao' },
        ]
      },
      {
        text: 'Documentação',
        items: [
          { text: 'Guia Completo', link: '/guide/' },
          { text: 'Receitas (Cookbook)', link: '/cookbook/' },
          { text: 'Arquitetura', link: '/architecture/' },
        ]
      }
    ],

    socialLinks: [
      { icon: 'github', link: 'https://github.com/JohnAnon9771/devobox' }
    ],

    footer: {
      message: 'Lançado sob a licença MIT.',
      copyright: 'Copyright © 2024-presente Devobox Contributors'
    },

    search: {
      provider: 'local'
    },
    
    editLink: {
      pattern: 'https://github.com/JohnAnon9771/devobox/edit/main/docs/:path',
      text: 'Editar esta página no GitHub'
    }
  }
})
