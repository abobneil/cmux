# Linux / Omarchy Port Plan

This document is the source of truth for porting cmux from the macOS Swift/AppKit application to a native Linux application that works well on Omarchy (Arch Linux + Hyprland/Wayland). It defines the target stack, milestones, acceptance criteria, packaging path, and the intended PR sequence.

## Goals

- Provide a native Linux desktop application for cmux users on Omarchy/Arch.
- Keep the Linux port independent from the macOS Swift/AppKit implementation while preserving shared product concepts: workspaces, panes, sessions, agent commands, and notifications.
- Make the app buildable with ordinary Linux tooling and packageable for Arch/AUR.
- Keep the first usable Linux release focused on a reliable terminal/session workflow rather than full macOS feature parity.

## Target stack

- Language: Rust
- UI: GTK4 + libadwaita
- Terminal: VTE initially; evaluate Ghostty/libghostty integration once its embeddable Linux API is stable enough for packaging
- Notifications: FreeDesktop notifications through `notify-rust`
- Config paths: XDG Base Directory
  - Config: `~/.config/cmux`
  - State: `~/.local/state/cmux` or `~/.local/share/cmux` when state APIs require it
  - Cache: `~/.cache/cmux`
- Packaging: Arch `PKGBUILD`, then AUR package, installable on Omarchy with `omarchy pkg aur add cmux`
- Display target: Wayland-first, with behavior tested under Hyprland/Omarchy

## Why Rust + GTK4

Rust gives cmux a native, memory-safe Linux core with strong process, PTY, async, and packaging support. GTK4/libadwaita provides a native Wayland-friendly UI that fits GNOME/FreeDesktop conventions and works well under Hyprland. This avoids an Electron port and avoids trying to reuse macOS-only AppKit code.

## Non-goals for the first Linux milestone

- Full feature parity with the macOS application
- Reusing Swift/AppKit UI code
- Shipping an Electron shell
- Depending on private macOS APIs or Xcode
- Implementing every macOS keyboard shortcut before the first usable terminal/session release

## Architecture plan

### Crates

- `linux/cmux-core`: platform-independent Linux domain logic
  - session/workspace/pane model
  - agent command metadata
  - XDG state persistence
  - notification/event abstractions
- `linux/cmux-gtk`: GTK4/libadwaita application shell
  - main window and navigation
  - VTE terminal embedding
  - session list/sidebar UI
  - notification integration

### Data model

The Linux port should model the same user-visible concepts as macOS:

- Workspace: named collection of sessions/panes.
- Session: terminal-backed unit of work with title, working directory, shell, and agent metadata.
- Pane: future split/layout unit; initially a single terminal surface is acceptable.
- Agent command: launchable command preset for Claude, Codex, or future agents.
- Agent status: idle/running/waiting-for-input/failed/done where detectable.

### Persistence

- Use XDG paths only; do not write state into the repository or macOS paths.
- Store state in a versioned format so migrations can be tested.
- State writes should be atomic where practical.
- Corrupt state should not prevent the app from launching; preserve or rename bad files for debugging.

### Notifications

- Use FreeDesktop notifications.
- Notifications should include enough session/workspace context to route the user back to the relevant item.
- Waiting-for-input detection should be modeled in core code and surfaced through GTK notifications.

### Packaging

- Add Arch packaging in `packaging/arch/`.
- Install a `.desktop` file, icon, and executable.
- Document system dependencies and Omarchy install commands.
- Do not publish to AUR until the Linux app can launch a terminal session reliably.

## Milestones and acceptance criteria

### M0: Repository structure and build skeleton

Required work:

- Add a Rust workspace under `linux/`.
- Add a minimal GTK4/libadwaita app crate.
- Add a core crate for non-UI Linux domain logic.
- Add CI checks for formatting, clippy, and tests where Linux GTK dependencies are available.
- Keep macOS code untouched.

Acceptance criteria:

- `cargo fmt --all -- --check` passes from `linux/`.
- `cargo clippy --workspace --all-targets -- -D warnings` passes from `linux/`.
- `cargo test --workspace` passes from `linux/`.
- The GTK app starts and shows an application window on a Linux desktop with GTK dependencies installed.

### M1: Core terminal shell

Required work:

- Create a main window with a vertical workspace/session sidebar.
- Embed a terminal widget using VTE.
- Spawn the user's shell through a PTY.
- Support opening multiple terminal sessions.
- Track the selected session in UI state.

Acceptance criteria:

- A user can launch the Linux app and get an interactive shell.
- A user can create at least two sessions and switch between them.
- Closing the app terminates or detaches child processes according to a documented policy.

### M2: Agent/session model

Required work:

- Model cmux workspaces, panes, agent commands, session state, and titles in Rust.
- Persist state using XDG paths.
- Add unit tests for session state and config migration.
- Ensure UI receives immutable snapshots or explicit actions rather than directly owning mutable global state where avoidable.

Acceptance criteria:

- Session/workspace state survives app restart.
- Migration tests cover at least one older state shape before any migration is introduced.
- Invalid/corrupt state is handled with a user-visible fallback rather than a crash.

### M3: Notifications and agent hooks

Required work:

- Implement FreeDesktop notifications.
- Port agent hook detection and status updates.
- Support Claude/Codex-style waiting-for-input notifications with context.
- Keep notification detection testable without requiring a live desktop notification daemon.

Acceptance criteria:

- Agent status transitions are represented in the core model.
- Waiting-for-input can trigger a desktop notification on Linux.
- Unit tests cover notification decision logic without depending on a GUI session.

### M4: Omarchy packaging

Required work:

- Add a real `packaging/arch/PKGBUILD`.
- Add `.desktop` file and icon installation.
- Document Omarchy install steps.
- Add package validation notes for local `makepkg` usage.

Acceptance criteria:

- The package builds on Arch with documented dependencies installed.
- The installed app appears in the desktop launcher.
- The installed binary can launch the GTK app.

### M5: Ghostty compatibility evaluation

Required work:

- Continue reading Ghostty config for theme/font/color compatibility where practical.
- Evaluate replacing VTE with Ghostty/libghostty if Linux embedding is viable and distributable on Arch.
- Document findings, blockers, and packaging implications.

Acceptance criteria:

- The repository records whether Ghostty embedding is viable for the Linux port at the time of evaluation.
- If not viable, VTE remains the supported terminal backend and the reason is documented.

## Proposed PR sequence

1. Add this Linux/Omarchy port plan.
2. Add Rust workspace and GTK4/libadwaita app skeleton.
3. Add terminal abstraction and VTE-backed prototype.
4. Add session/workspace domain model and persistence.
5. Add notifications and agent hook service.
6. Add Arch/Omarchy packaging.
7. Add Linux CI scoping so macOS-only checks do not block Linux-only work while required checks still complete.
8. Iterate toward feature parity once the Linux app can launch, persist sessions, notify, and install cleanly on Arch/Omarchy.

## Review checklist for Linux-port PRs

- Does the PR keep macOS behavior unchanged unless explicitly stated?
- Does Linux state use XDG paths?
- Are user-facing Linux strings prepared for localization once localization infrastructure exists for the port?
- Are tests behavioral rather than grep-style source checks?
- Are UI rows/lists fed by snapshots and explicit actions instead of broad mutable store references?
- Does packaging avoid hard-coded local paths?
- Does CI preserve required-check semantics for skipped platform-specific work?

## Risks and follow-ups

- VTE may not match Ghostty behavior exactly; track compatibility differences before promising parity.
- Hyprland/Omarchy behavior can differ from GNOME; test Wayland paths directly.
- AUR publishing should wait until the application provides a reliable terminal/session baseline.
- Required GitHub checks must not be skipped in a way that leaves Linux-only PRs permanently pending.
