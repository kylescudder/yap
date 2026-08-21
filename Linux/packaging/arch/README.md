# Arch packages

## Production package

`PKGBUILD` builds an immutable, checksummed Linux source snapshot for Arch and Hyprland. Its package
name is `yap-dictation` because the AUR name `yap` belongs to YAP Prolog; the installed commands
remain `yap`, `yapctl`, and `yapd`.

The base package installs CPU-capable local transcription and cleanup, the GTK dashboard,
layer-shell indicator, tray adapter, and user services. It does not pull the CUDA toolchain.
NVIDIA users can install `ggml-cuda` as an optional accelerator:

```sh
makepkg --syncdeps --install --cleanbuild --clean
sudo pacman -S --needed ggml-cuda # optional

yap model install
yap doctor
yap setup hyprland
```

`yap setup hyprland` enables the daemon, overlay, and tray services and prints the commands for
user-owned Dictation and Command Mode bindings. The package never downloads a model as root and
never edits a new user's Hyprland config. Package upgrades leave models and user state untouched.

To publish an update to the AUR, bump `pkgver`/`pkgrel`, pin `_commit`, update the source checksum,
regenerate `.SRCINFO` with `makepkg --printsrcinfo`, and copy `PKGBUILD`, `.SRCINFO`, and
`yap.install` into the `yap-dictation` AUR repository.

## Development package

The development package builds the current checkout and lets pacman resolve Yap's runtime tools.
It exists so the first real Hyprland/CUDA validation happens through the same installation path a
user will see, without manually preparing dependencies for the disposable probe.

From this directory:

```sh
makepkg -p PKGBUILD.dev --syncdeps --install --clean --force
```

The development package installs the complete Linux application: diagnostics/model/setup client,
D-Bus daemon, control client, dashboard, overlay, tray, and three user services. After package
installation, model installation and Hyprland setup remain explicit user-scoped operations:

```sh
yap model install
yap doctor
yap setup hyprland
```

`whisper-cpp`, `llama-cpp`, `ggml-cpu`, the `ggml-cuda` accelerator, PipeWire/WirePlumber tools,
GTK, `wtype`, `wl-clipboard`, and the Hyprland portal backend are hard dependencies in this
development package because its hardware target is the validated RTX 3080 machine. The release
package keeps CUDA optional for CPU fallback.

`PKGBUILD.dev` is intentionally not an AUR submission: it reads the current checkout and keeps CUDA
mandatory so the validated RTX 3080 path remains exercised.
