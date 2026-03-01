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
- Extensible: Anyone can add new widget types via simple plugins/pipes.

## Technical Architecture (Current & Target)
- **Core Framework**: Tauri v2 (Rust backend + lightweight webview frontend). Tiny binary, native performance.
- **Screen Intelligence**: Screenpipe (primary) for continuous capture + OCR (Apple Vision / Windows OCR / Tesseract fallback).  
  Hybrid upgrade: Accessibility APIs (macOS AX, Windows UI Automation, Linux AT-SPI) for exact bounding boxes when possible.
- **Overlay Engine**: Tauri transparent always-on-top webview windows (or native Rust egui/winit if needed). Click-through support, precise positioning via Screenpipe coordinates + delta detection.
- **Widget Rendering**: Any web tech (React/Svelte + Three.js, charts, iframes, mini-players). Custom protocol handlers for `wormlink://` schemes.
- **State & Storage**: Local SQLite (via Screenpipe) + simple Rust cache for resolved widgets.
- **Detection Flow**:
  1. Screenpipe → OCR text + frame metadata every ~300-500ms (configurable).
  2. Regex match for `wormlink://[^\s]+`.
  3. Resolve (local file / HTTP / plugin).
  4. Spawn positioned widget window.
  5. Smart refresh on scroll/resize/window change.

## Tech Stack & Conventions
- **Rust**: Follow Rust 2024 edition, clippy lints, idiomatic code. Use `anyhow` for errors, `tracing` for logs.
- **Tauri**: v2, no unnecessary plugins. Use `tauri-plugin` only when truly needed.
- **Frontend**: Svelte 5 or React (your choice — keep it minimal). Tailwind for styling. Support dark/light system theme.
- **Screenpipe Integration**: Use `@screenpipe/js` or direct REST to `localhost:3030`. Create a custom Pipe when possible.
- **Cross-platform**: Abstract OS-specific code (accessibility, window management) behind clean traits.
- **Dependencies**: Keep minimal. No heavy crates unless justified.
- **Versioning**: Semantic versioning. `CHANGELOG.md` updated on every release.

## Coding Style & Best Practices
- Write clear, self-documenting code. Comments only for “why”, not “what”.
- Every public function gets Rust docs.
- Error handling: Never `unwrap()` in production paths. Graceful degradation (log + fallback to OCR-only).
- Performance: Keep background CPU < 5% idle. Use efficient screen delta detection.
- Security: All network calls go through user-approved allowlist. Sandbox widget webviews.
- Testing: Unit tests for parsers/resolvers. Integration tests with mock Screenpipe output. E2E where feasible.
- Git: Conventional commits. Small, focused PRs.

## How You (Gemini) Should Help Me
- Always stay in character as the expert co-founder/architect of Wormlinks.
- When I ask for code, output complete, ready-to-paste files with proper headers and comments.
- Prefer Rust-first solutions; only suggest web tech for widgets.
- Suggest incremental improvements toward the “buttery magic” version (accessibility + delta detection).
- If I say “make it better”, focus on performance, reliability, or UX delight.
- When reviewing code, call out any bloat, security holes, or cross-platform issues.
- Use `/memory refresh` if context ever feels stale.

## Roadmap Priorities (in order)
1. Working prototype with `wormlink://image?id=...` (Screenpipe + Tauri overlay).
2. Accessibility hybrid detection.
3. More widget types (live Excel mirror, 3D, mini-player, interactive poll).
4. Plugin system / custom Pipes.
5. Installer + auto-updates.
6. Community widget gallery.

Let’s ship the thing that makes every power user say “why didn’t this exist years ago?”

You now have full context. Go build Wormlinks.