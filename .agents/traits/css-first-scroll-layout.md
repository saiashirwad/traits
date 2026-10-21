---
name: css-first-scroll-layout
purpose: Keep scroll-dependent presentation reliable through native layout rather than synchronizing JavaScript with scrolling.
subject: Scroll-dependent UI layout, positioning, and visibility.
requirement: Prefer DOM placement, appropriate containing blocks, and CSS
  positioning for scroll-dependent UI layout such as sticky bars. Do not add
  JavaScript scroll listeners, visibility atoms, or reveal/hide logic when CSS
  can reliably express the intended behavior. Use scripted behavior only when
  the requirement cannot be achieved reliably through layout and styling.
applicability: |-
  Applies when implementing scroll-dependent UI layout, sticky positioning, or visibility behavior.
  Exception: When every scripted scroll-dependent behavior under assessment implements a requirement that DOM placement, layout, and CSS cannot reliably express.
paths:
  - "*.css"
  - "*.html"
  - "*.tsx"
  - "*.jsx"
  - "*.ts"
  - "*.mts"
  - "*.cts"
  - "*.js"
  - "*.mjs"
  - "*.cjs"
---
```diff
-window.addEventListener("scroll", () => header.classList.toggle("fixed", window.scrollY > 100))
+/* Place the header in its intended scroll container. */
+header { position: sticky; top: 0; }
```
