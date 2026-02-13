# Font Localization Audit — Tairseach

**Date:** 2026-02-12  
**Task:** Audit and localize all external frontend dependencies  
**Status:** ✅ **COMPLETE** — Zero external network requests

---

## Changes Made

### 1. Downloaded Font Files Locally
All Google Fonts families have been downloaded as TTF files:

**Location:** `src/assets/fonts/`

- **Cinzel** (Display font):
  - `cinzel-400.ttf` (45 KB)
  - `cinzel-500.ttf` (45 KB)
  - `cinzel-600.ttf` (45 KB)
  - `cinzel-700.ttf` (45 KB)

- **Cormorant Garamond** (Body font):
  - `cormorant-garamond-400.ttf` (283 KB)
  - `cormorant-garamond-500.ttf` (284 KB)
  - `cormorant-garamond-600.ttf` (284 KB)
  - `cormorant-garamond-400-italic.ttf` (286 KB)
  - `cormorant-garamond-500-italic.ttf` (286 KB)

- **JetBrains Mono** (Monospace font):
  - `jetbrains-mono-400.ttf` (110 KB)
  - `jetbrains-mono-500.ttf` (110 KB)
  - `jetbrains-mono-600.ttf` (110 KB)

**Total:** 12 font files, ~1.7 MB

---

### 2. Created Local Font Declarations
**File:** `src/assets/styles/fonts.css`

All fonts now use `@font-face` with local file paths:
```css
@font-face {
  font-family: 'Cinzel';
  src: url('../fonts/cinzel-400.ttf') format('truetype');
  /* ... */
}
```

---

### 3. Updated Import Chain
**File:** `src/assets/styles/naonur-theme.css`

**Before:**
```css
@import url('https://fonts.googleapis.com/css2?family=Cinzel:...');
```

**After:**
```css
@import './fonts.css';
```

---

### 4. Removed Preconnect Hints
**File:** `index.html`

Removed:
```html
<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
```

---

## Build Verification

### ✅ Fonts Bundled with Content Hashes
Build output shows all fonts properly bundled:
```
dist/assets/cinzel-400-D9Ck3daX.ttf
dist/assets/cinzel-500-CHU7Y7dN.ttf
dist/assets/cinzel-600-Dc04s2rB.ttf
dist/assets/cinzel-700-BnVU4aZj.ttf
dist/assets/cormorant-garamond-400-DQv4XaSs.ttf
dist/assets/cormorant-garamond-500-Pgl8-Oo5.ttf
dist/assets/cormorant-garamond-600-CJWdwruJ.ttf
dist/assets/cormorant-garamond-400-italic-D_ufOZIm.ttf
dist/assets/cormorant-garamond-500-italic-B6MJOsWb.ttf
dist/assets/jetbrains-mono-400-B6W8R_vR.ttf
dist/assets/jetbrains-mono-500-9KJuWOdP.ttf
dist/assets/jetbrains-mono-600-BcXjrrhU.ttf
```

### ✅ No External URLs in Build Output
- ✅ `index.html` — Clean, no preconnect links
- ✅ `assets/*.css` — All font URLs point to local `/assets/*.ttf` files
- ✅ No `googleapis.com` or `gstatic.com` references

### ✅ Build Succeeds
```
✓ built in 947ms
dist/assets/index-juSypUAc.css   42.90 kB │ gzip: 8.28 kB
dist/assets/index-DPMCjtKb.js   227.73 kB │ gzip: 74.43 kB
```

---

## Remaining URLs (Non-Loaded)

The following URLs appear in source but are **not** loaded dependencies:

1. **User-facing documentation links** in `src/views/GoogleSettingsView.vue`:
   - Links to Google Cloud Console (opens in external browser on user click)
   - These are `<a href="...">` tags with `target="_blank"`, not loaded assets

2. **Placeholder text** in `src/components/config/ProviderCard.vue`:
   - `placeholder="http://localhost:11434/v1"` — form placeholder, not loaded

3. **Embedded SVG data URLs**:
   - All SVG backgrounds use `data:image/svg+xml` URIs (embedded, not fetched)

4. **Comments**:
   - Tailwind CSS license comment
   - Vite config comment pointing to docs

---

## Dependencies Check

### NPM Dependencies (from `package.json`)
All dependencies are properly bundled by Vite:

**Runtime:**
- `@tauri-apps/api` — bundled
- `@tauri-apps/plugin-shell` — bundled
- `pinia` — bundled
- `vue` — bundled
- `vue-router` — bundled

**Build:**
- `tailwindcss` — processed at build time
- `vite` — build tool (not in bundle)
- `typescript` — compile time only

### ✅ No CDN Dependencies
- No `unpkg`, `jsdelivr`, `skypack`, or similar CDN imports
- No externalized dependencies in `vite.config.ts`
- All npm packages bundled into output JS

---

## Conclusion

**Tairseach is now a fully self-contained desktop application.**

✅ **Zero network requests for UI assets**  
✅ **All fonts bundled locally with content hashing**  
✅ **Instant load times — no CDN latency**  
✅ **Works completely offline**  

The build produces a hermetically sealed bundle. When launched, the app loads entirely from local filesystem with zero external HTTP requests for fonts, CSS, or JavaScript.

*The threshold is sealed. The fonts are home.* ☁️
