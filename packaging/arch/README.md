# Arch / Omarchy packaging

This directory contains the Arch package definition for the Linux GTK port of cmux. It targets Omarchy and other Arch Linux systems.

## Package contents

The package installs:

- `/usr/bin/cmux`
- `/usr/share/applications/cmux.desktop`
- `/usr/share/icons/hicolor/scalable/apps/cmux.svg`

Runtime dependencies:

- `gtk4`
- `libadwaita`
- `vte4`

Build/check dependencies:

- `base-devel`
- `cargo`
- `git`
- `pkgconf`
- `desktop-file-utils`

## Build dependencies

```bash
sudo pacman -S --needed base-devel cargo git gtk4 libadwaita vte4 pkgconf desktop-file-utils
```

## Build and install locally

From this repository:

```bash
cd packaging/arch
makepkg -si
```

Useful validation commands while iterating:

```bash
cd packaging/arch
makepkg --printsrcinfo > .SRCINFO
makepkg --cleanbuild --syncdeps --check
namcap PKGBUILD
```

`makepkg --check` runs the Rust workspace tests and validates `cmux.desktop` with `desktop-file-validate`.

## Omarchy install flow

Before AUR publication, install from a local checkout with `makepkg -si` as above.

Once published to the AUR, Omarchy users should be able to install it with:

```bash
omarchy pkg aur add cmux-git
```

## Runtime notes

The Linux port uses GTK4/libadwaita for the shell and VTE for PTY-backed terminal sessions. Agent launch entries currently cover shell, Claude, and Codex commands and expect those tools to be installed on the user's `PATH`.

The package intentionally builds with Cargo's checked-in lockfile via `--locked` so packaged builds use the same dependency resolution as CI.
