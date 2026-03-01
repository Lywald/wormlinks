# Wormlinks — Universal Screen Hyperlinks

## Project Vision
Wormlinks turns *any text on any screen* into a live portal to rich widgets, data, and media — without app developers doing anything.

Core idea: Users type `wormlink://...` anywhere (Excel, Slack, Reddit, Notion, terminal, PDFs, etc.).  
A lightweight always-on desktop agent detects it in real time and renders a beautiful, interactive overlay widget exactly over the tag.

This is the “superpower layer” above the OS. Power users will install it on day one and never want to live without it.

## Non-Negotiable Principles
- Works everywhere: Windows + macOS + Linux (no Electron bloat).
- Zero friction: No APIs, no plugins, no permissions from target apps.
- Privacy-first: Everything runs locally. No telemetry. Screen data never leaves the machine.
- Magic feel: Sub-second detection, pixel-perfect positioning, buttery overlays.
- Secure by default: Widgets are sandboxed. Users must explicitly whitelist domains/sources the first time.
- Extensible: Anyone can add new widget types via simple plugins.
- 100% free & open source forever: No paid services, no hidden costs.

## Technical Architecture (Locked In)
- **Core Framework**: Tauri v2 (Rust backend + lightweight webview frontend). Tiny binary (~10-30 MB), native performance.
- **Screen Intelligence (Primary)**: Native Accessibility APIs for exact text + bounding boxes
  - Windows: `uiautomation` crate (Windows UI Automation tree)
  - macOS: `objc2` + Apple Accessibility (AX API)
  - Linux: `atspi` crate (AT-SPI2)
- **Fallback**: `ocrs` (pure Rust OCR) + `xcap`/`scap` for screen delta capture — only on canvas/image/legacy-rendered text.
- **Overlay Engine**: Tauri transparent always-on-top webview windows (click-through, precise positioning via accessibility bounds).
- **Widget Rendering**: Any web tech (Svelte/React + Three.js, charts, iframes, mini-players). Custom `wormlink://` protocol handler.
- **State & Storage**: Local SQLite + simple Rust cache for resolved widgets.
- **Detection Flow** (sub-150 ms target):
  1. Poll accessibility tree (or delta-capture regions) every 300-500 ms.
  2. Regex match for `wormlink://[^\s]+`.
  3. Resolve (local / HTTP / plugin — user-whitelisted).
  4. Spawn positioned widget window.
  5. Smart refresh on scroll/resize/focus change.

## Tech Stack & Conventions
- **Rust**: 2024 edition, clippy, `anyhow`, `tracing`, `tokio`.
- **Tauri**: v2, minimal plugins only.
- **Frontend**: Svelte 5 (preferred) or React. Tailwind. System dark/light theme.
- **Accessibility Crates**:
  - `uiautomation` (Windows)
  - `objc2` + `accessibility` bindings (macOS)
  - `atspi` (Linux)
- **Fallback**:
  - `ocrs` (OCR)
  - `xcap` or `scap` (fast screen capture / delta)
- **Dependencies**: Keep extremely minimal. No heavy frameworks.
- **Versioning**: Semantic. `CHANGELOG.md` on every release.

## Coding Style & Best Practices
- Clear, self-documenting code. Comments only for “why”.
- Every public item gets Rust docs.
- Never `unwrap()` in hot paths. Graceful fallback (log + OCR if needed).
- Performance: Idle CPU < 2 %. Use efficient delta detection.
- Security: All network calls behind user allowlist. Sandbox all widget webviews.
- Testing: Unit for parser/resolver. Mock accessibility tree for integration tests.
- Git: Conventional commits. Tiny PRs.

## How You (Gemini) Should Help Me
- Always act as co-founder/architect of Wormlinks.
- When I ask for code: output complete, ready-to-paste files with headers and comments.
- Always prefer the native accessibility-first path (with clean fallback).
- Suggest improvements toward “buttery magic” (sub-second, pixel-perfect, zero jitter).
- If I say “make it better”, focus on performance, reliability, UX delight.
- When reviewing: flag any bloat, security holes, or cross-platform issues.
- Use `/memory refresh` only if context feels stale.

## Roadmap Priorities (in order)
1. Working prototype with `wormlink://image?id=...` using native accessibility + Tauri overlay.
2. Full cross-platform accessibility implementation + OCR fallback.
3. More widget types (live Excel mirror, 3D, mini-player, poll).
4. Plugin system.
5. One-click installer + auto-updates.
6. Community widget gallery.

This is now the official architecture — fully free, fully ours, no compromises.

You have full context. Let’s build the tool that should have existed years ago.