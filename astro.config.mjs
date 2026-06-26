import { defineConfig } from 'astro/config'

import tailwindcss from '@tailwindcss/vite';
import svelte from '@astrojs/svelte';
import node from '@astrojs/node';


export default defineConfig({
  site: 'https://lorearchive.org',
  build: {},
  output: 'server',

  redirects: {
      "/": "/wiki/home"
  },

  image: {
    remotePatterns: [{ protocol: "https" }],
    domains: ["avatars.githubusercontent.com"]
  },

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
  },

  integrations: [svelte()],

  adapter: node({
    mode: 'standalone'
  })
});