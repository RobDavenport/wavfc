# GitHub Pages Deployment for wavfc Demo

**Date:** 2026-02-21
**Status:** Approved

## Goal

Deploy the demo-wasm application to GitHub Pages so the library can be validated and showcased via a public URL.

## Approach

GitHub Actions workflow with artifact upload (no `gh-pages` branch). On every push to `main`, the workflow:

1. Checks out code
2. Installs Rust toolchain + `wasm-pack`
3. Builds `demo-wasm` with `wasm-pack build --target web --release`
4. Assembles a flat site directory with HTML/CSS/JS + WASM pkg
5. Uploads as a Pages artifact
6. Deploys to GitHub Pages

## Files Changed

| File | Action | Purpose |
|------|--------|---------|
| `.github/workflows/deploy-pages.yml` | Create | CI workflow |
| `demo-wasm/www/main.js` | Edit | Fix import path for flat site layout |

## Site Structure (deployed)

```
/
├── index.html
├── style.css
├── main.js
├── .nojekyll
└── pkg/
    ├── wavfc_demo.js
    ├── wavfc_demo_bg.wasm
    └── (other wasm-pack output)
```

## Post-deploy Setup

User must enable GitHub Pages with "GitHub Actions" source in repo settings.

## Implementation Plan

### Step 1: Fix main.js import path

Change the WASM import in `demo-wasm/www/main.js` from `'../pkg/wavfc_demo.js'` to `'./pkg/wavfc_demo.js'` so it works in the flat deployed layout.

### Step 2: Create GitHub Actions workflow

Create `.github/workflows/deploy-pages.yml` with:
- Trigger: push to `main`
- Permissions: `pages: write`, `id-token: write`, `contents: read`
- Concurrency group to cancel in-flight deploys
- Jobs:
  - **build**: Install Rust, wasm-pack, build, assemble site, upload artifact
  - **deploy**: Deploy Pages artifact

### Step 3: Verify locally

Run `wasm-pack build` in demo-wasm and confirm the site loads with the new import path.
