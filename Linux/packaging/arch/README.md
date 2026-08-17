# Arch development package

The development package builds the current checkout and lets pacman resolve Yap's runtime tools.
It exists so the first real Hyprland/CUDA validation happens through the same installation path a
user will see, without manually preparing dependencies for the disposable probe.

From this directory:

```sh
makepkg --syncdeps --install --clean --force --pfile PKGBUILD.dev
```

The development package installs the complete first dictation slice: the diagnostic/model/setup
client, D-Bus daemon, control client, user systemd unit, and D-Bus activation file. After package
installation, model installation and Hyprland setup remain explicit user-scoped operations:

```sh
yap model install
yap doctor
yap setup hyprland
```

`whisper-cpp`, the `ggml-cuda` backend, PipeWire audio tools, `wtype`, `wl-clipboard`, and the
Hyprland portal backend are hard dependencies in this development package because its first
hardware target is the validated RTX 3080 machine. The eventual release packaging will split CUDA
from the CPU-fallback base package.

`PKGBUILD.dev` is intentionally not an AUR submission: it reads the current checkout. The eventual
AUR `PKGBUILD` will build an immutable versioned release archive with a pinned checksum.
