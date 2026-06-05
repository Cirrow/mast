import global from './src/store'
import { defineConfig } from 'astro/config'
import { processAllPages } from './src/utils/pages-processor'
import { fetchWikiContent, getAllPages } from './src/utils/git-service'

import svelte from '@astrojs/svelte';
import node from '@astrojs/node'

import tailwindcss from '@tailwindcss/vite';


export default defineConfig({
  site: 'https://lorearchive.org',
  build: {},
  output: 'server',

  redirects: {
      "/": "/wiki/home"
  },

  integrations: [
      
      svelte(),
  
  ],

  image: {
    remotePatterns: [{ protocol: "https" }],
    domains: ["avatars.githubusercontent.com"]
  },

  adapter: node({
    mode: 'standalone'
  }),

  vite: {
    plugins: [tailwindcss()],
    resolve: {
        noExternal: ['bits-ui']
    },
    server: {
        proxy: {
            '/api': 'http://localhost:3000'
        }
    }
  }
});