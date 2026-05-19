# Ghostty compatibility evaluation for the Linux port

This document records the M5 evaluation of whether the Linux GTK port should replace VTE with Ghostty/libghostty as its terminal backend.

## Decision

Keep VTE as the supported Linux terminal backend for now.

Ghostty remains important for compatibility goals, but embedding Ghostty directly in the Linux GTK application is not currently the packaging-safe path for the first Linux release. The port should instead continue to use VTE while sharing practical Ghostty-compatible configuration behavior where possible.

## Current backend

The Linux app uses:

- GTK4/libadwaita for the application shell
- VTE for PTY-backed terminal widgets
- Rust domain logic in `cmux-core`

This backend is available through Arch packages and works with normal GTK/Wayland packaging expectations.

## Ghostty embedding status

Ghostty is optimized around its own application/runtime. The cmux macOS application currently relies on a bundled GhosttyKit flow, but that does not directly translate into an Arch-packaged Linux GTK widget backend.

At the time of this evaluation, the Linux port does not have a stable, documented, packaging-friendly Ghostty/libghostty GTK embedding API that can be consumed as an ordinary Arch dependency. Adopting Ghostty as the Linux backend now would likely require vendoring or building a large terminal stack inside the package and carrying integration risk before the Linux port has reached its first usable release.

## Packaging implications

Using VTE keeps the package simple:

- runtime dependency: `vte4`
- build dependency: normal Rust/Cargo tooling plus GTK development packages
- no bundled terminal renderer artifacts
- no fork-specific Ghostty build requirements

Using Ghostty now would add unresolved questions:

- whether a stable embeddable Linux ABI/API exists for this use case
- how to package that API on Arch/AUR without vendoring fragile build outputs
- how to integrate rendering/event loops with GTK without duplicating Ghostty app behavior
- how to keep the package reproducible and maintainable for Omarchy users

## Compatibility work to keep

Even while VTE remains the backend, the Linux port should continue to pursue compatibility where practical:

- read common Ghostty config values for font family, font size, theme, cursor style, and color palette where they map cleanly to GTK/VTE
- avoid promising exact Ghostty rendering behavior while VTE is the backend
- document known differences between VTE and Ghostty as users report them
- keep terminal backend boundaries in the code so a future Ghostty backend can be evaluated without rewriting session/domain logic

## Re-evaluation criteria

Revisit Ghostty embedding when all of the following are true:

1. Ghostty exposes a stable Linux embeddable API suitable for use from a GTK application.
2. The API can be consumed from Rust without relying on private or app-internal interfaces.
3. The dependency can be packaged on Arch/AUR in a reproducible way.
4. The renderer/event-loop integration does not regress typing latency, focus behavior, or Wayland compatibility.
5. The Linux port already has the VTE baseline working: launch, multiple sessions, persistence, notifications, and Arch packaging.

## Follow-up tasks

- Add a small Ghostty config reader for safe, backend-agnostic settings.
- Add documentation for VTE/Ghostty behavior differences once the app is dogfooded on Omarchy.
- Keep `cmux-core` free of terminal-widget dependencies so backend replacement remains possible.
