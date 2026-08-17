# Yap for Linux

This directory contains the durable Linux implementation. It deliberately lives beside the stable
Swift/AppKit application rather than trying to compile Apple frameworks on Linux.

The first module is `yap-core`, a platform-independent session machine. Its single transition
interface owns hold-to-talk, quick-tap cancellation, double-tap locking, Command Mode, late timers,
and ignored release edges. PipeWire, Whisper, Wayland, AT-SPI, storage, D-Bus, and GTK adapters will
remain outside that seam.

The `yap-linux-daemon` package owns the per-user session and exposes a deliberately small D-Bus
interface at `com.yap.Yap.Dictation1`: send a press/release edge, cancel a recording, or read the
current phase and last runtime error. The daemon timestamps edges itself with one monotonic clock.
`PipelineRuntime` is the adapter seam for PipeWire capture and transcription/insertion. The first
production adapter records 16 kHz mono WAV data into private runtime storage, sends it only to a
persistent loopback `whisper-server`, deletes the recording, and inserts non-empty text with
`wtype`. Command Mode remains explicitly unavailable.

The `yap-cli` package provides two thin clients. `yap doctor` is the stable interface that absorbs
useful capability checks from `Prototypes/LinuxHyprlandSpike`; its default check is read-only and
offline, classifies hard blockers separately from degradations and expected setup, and supports
JSON output. `yapctl` sends Hyprland hotkey edges to the daemon. The disposable shell harness itself
will not ship in the final package.

For pre-release Arch testing, `packaging/arch/PKGBUILD.dev` builds the current checkout, installs
`yap`, `yapd`, `yapctl`, D-Bus activation, and the per-user systemd unit, and declares all runtime
dependencies for the target NVIDIA machine. `yap model install` performs the sole explicit network
setup step and verifies the pinned model before use. `yap setup hyprland` backs up the main Lua
configuration, installs the Right-Super edge bindings, starts the daemon, and reloads Hyprland.

The final AUR package will replace the checkout-based manifest with a checksummed release archive
and split hardware acceleration choices cleanly so CPU-only systems do not require CUDA.
