# Yap for Linux

The Linux port is a native Arch/Hyprland implementation beside the Swift/AppKit app. The user guide
is in the repository [README](../README.md); this document records the durable module boundaries.

`yap-core` owns the session machine: hold/release behavior, quick-tap cancellation, double-tap
locking, Command Mode, timer generations, and ignored edges. It contains no PipeWire, D-Bus, GTK,
filesystem, compositor, or model process code.

`yap-linux-daemon` owns one user's runtime truth. Its adapters capture private 16 kHz mono audio,
publish numeric RMS levels, transcribe through a persistent loopback whisper.cpp server, clean or
transform through a persistent loopback llama.cpp server, apply deterministic polishing and
snippets, insert through Wayland, and remove audio. Focused-app collection retains only a sanitized
application class; Command Mode selection stays in memory and the clipboard is restored.

The daemon also owns the private atomic XDG store for settings, snippets, and the newest 200
successful dictations. D-Bus exposes typed session state and JSON snapshots to thin clients;
hotkey timing and pipeline policy are never reimplemented in GTK or shell code.

`yap` provides offline diagnostics, explicit model installation/repair, dashboard launch, and
Hyprland service setup. `yapctl` sends user-owned compositor press/release edges. `yap-overlay` is a
non-focus-stealing layer-shell surface, `yap-ui` is the GTK4 dashboard, and `yap-tray` is the
StatusNotifier menu adapter.

`packaging/arch/PKGBUILD.dev` builds the checkout for pre-release hardware validation. The
production `PKGBUILD` consumes an immutable checksummed GitHub archive, keeps CUDA optional, and
installs all three user services. The disposable feasibility probe remains under `Prototypes/` and
is not packaged.
