# Docs Site (optional)

A public docs site, built from this repo's own `docs/` content, deployed to GitHub Pages. Optional — only bootstrap this if the project needs public-facing docs (a self-hosted app with real users, a plugin ecosystem, etc.). Skip it for internal tools and single-user apps.

The deploy workflow (`.github/workflows/docs-site.yml`) is already in this repo and inert: the job is guarded on `docs-site/package.json` existing, so pushes touching `docs/**` (release notes, ADRs, etc.) or `docs-site/**` do nothing until you actually bootstrap `docs-site/`.

## Stack

Astro + React + Tailwind + MDX, deployed as a static site via GitHub Pages.

## Bootstrap

```bash
npm create astro@latest docs-site -- --template minimal --install --no-git
cd docs-site
npx astro add react mdx --yes
npm install @tailwindcss/vite tailwindcss clsx tailwind-merge lucide-react
```

`docs-site/astro.config.mjs`:

```js
import { defineConfig } from 'astro/config'
import react from '@astrojs/react'
import mdx from '@astrojs/mdx'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  site: 'https://{{GITHUB_USERNAME}}.github.io',
  base: '/{{REPO_NAME}}',
  integrations: [react(), mdx()],
  vite: { plugins: [tailwindcss()] },
})
```

## Wiring `docs/releases/` in as content

Read this repo's `docs/releases/*.md` directly rather than duplicating them into `docs-site/` — one source of truth for release notes.

`docs-site/src/content.config.ts`:

```ts
import { defineCollection, z } from 'astro:content'
import { glob } from 'astro/loaders'

const releases = defineCollection({
  loader: glob({ pattern: '*.md', base: '../docs/releases' }),
  schema: z.object({}), // extend as docs/releases/TEMPLATE.md's format stabilizes
})

export const collections = { releases }
```

## Suggested pages

- `src/pages/index.astro` — landing page.
- `src/pages/getting-started.astro` — install/run instructions (mirrors `CLAUDE.md`'s Dev Workflow).
- `src/pages/releases/[...slug].astro` — renders the `releases` collection.
- `src/pages/deploy/*` — deployment instructions per target, if the project ships more than one (e.g. Docker, manual, nginx).

## GitHub Pages setup (one-time, per repo)

Repo Settings → Pages → Source: "GitHub Actions". The workflow handles the rest on every push to `main` that touches `docs/` or `docs-site/`.
