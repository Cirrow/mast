import global from './src/store'
import { defineConfig } from 'astro/config'
import sitemap from '@astrojs/sitemap'
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
      
      sitemap({
          serialize(item) {
              // Extract the page path from the URL
              const url = new URL(item.url);
              let pathname = url.pathname;
              
              // Remove leading/trailing slashes
              pathname = pathname.replace(/^\/+|\/+$/g, '');
              
              // Remove the 'wiki/' prefix if it exists, since that's part of the content path
              // not the actual slug stored in global.lastMod
              const slugPath = pathname.replace(/^wiki\//, '');
              
              // Look up the lastMod date for this page
              const lastMod = global.lastMod[slugPath];
              
              console.log(`LORA: [SITEMAP DEBUG] URL: ${item.url}, pathname: ${pathname}, slugPath: ${slugPath}`);
              
              if (lastMod) {
                  console.log(`LORA: [SITEMAP] ${slugPath} -> ${lastMod.toISOString()}`);
                  return {
                      url: item.url,
                      lastmod: lastMod.toISOString(),
                      links: item.links
                  };
              
              } else {
                  console.warn(`LORA: [SITEMAP] No lastMod found for ${slugPath}, available keys:`, Object.keys(global.lastMod));
                  return {
                      url: item.url,
                      lastmod: new Date().toISOString(),
                      links: item.links
                  };
              }
          }
      }), 
      
      svelte(),
  
  ],

  image: {
    remotePatterns: [{ protocol: "https" }],
    domains: ["avatars.githubusercontent.com"]
  },

  adapter: node({
    mode: 'standalone',
  }),

  vite: {
    plugins: [tailwindcss()],
    resolve: {
        noExternal: ['bits-ui']
    }
  }
});