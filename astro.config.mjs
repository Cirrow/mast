import { defineConfig } from 'astro/config'

import tailwindcss from '@tailwindcss/vite';
import svelte from '@astrojs/svelte';


export default defineConfig({
  build: {},
  output: 'static',

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
    }
  },

  integrations: [svelte()],

});