import { defineConfig } from 'astro/config'
import react from '@astrojs/react'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  site: 'https://t2vi.github.io',
  base: '/spoolbook',
  integrations: [react()],
  vite: {
    plugins: [tailwindcss()],
  },
})
