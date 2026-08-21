# Yap

**Local-first voice dictation for macOS and Arch Linux.** Hold a key, speak, and clean,
formatted text appears in the focused application. There are no accounts, API keys, cloud speech
services, or subscription.

## Features

- Dictation in any editable app, quick-tap cancellation, and double-tap hands-free mode.
- Command Mode transforms selected text from a spoken instruction, or generates at the cursor.
- Five cleanup levels, courtesy trimming, and tone matched to email, chat, code, or notes.
- Persistent snippets, capped dictation history, and off/lower/pause playback behavior.
- A non-focus-stealing recording indicator, dashboard, health checks, and tray controls.
- Local transcription and cleanup after the explicit one-time model download.

## Arch Linux and Hyprland

The first Linux release supports x86_64 Arch Linux in a Hyprland Wayland session. CPU inference is
the baseline; NVIDIA acceleration is optional. The two pinned models use about 2.9 GiB in total.

### Install

Once the package is published to the AUR:

```sh
paru -S yap-dictation
```

Until then, build the release package from a checkout:

```sh
git clone https://github.com/kylescudder/yap.git
cd yap/Linux/packaging/arch
makepkg --syncdeps --install --cleanbuild --clean
```

For NVIDIA acceleration, install the shared ggml CUDA backend. CPU-only systems should skip this:

```sh
sudo pacman -S --needed ggml-cuda
```

Complete user-scoped setup—never run these commands with `sudo`:

```sh
yap model install
yap setup hyprland
yap doctor
```

`yap model install` downloads pinned Whisper and Qwen GGUF files from Hugging Face, verifies their
SHA-256 hashes, and repairs a corrupt Yap-owned model when asked to run again. Dictation is offline
after that step.

### Hyprland bindings

Yap deliberately leaves hotkeys in your compositor configuration. A binding must send both edges:

```ini
# Example keys only—choose bindings that fit your configuration.
bind  = SUPER, D, exec, yapctl press dictation
bindr = SUPER, D, exec, yapctl release dictation

bind  = SUPER ALT, D, exec, yapctl press command
bindr = SUPER ALT, D, exec, yapctl release command
```

For Hyprland's Lua configuration, bind the same four commands through your existing `hl.bind`
helpers and mark the two release commands as release-edge bindings. `yap setup hyprland` and the
dashboard print copyable command pairs; Yap never rewrites a user-owned Hyprland file.

### Linux usage

- Hold the Dictation binding, speak, then release to transcribe, clean, and insert.
- Quickly tap and release to cancel without transcription.
- Double-tap Dictation for hands-free recording; press once more to stop.
- Select text, hold Command Mode, and speak an instruction such as “make this more concise.” With
  no selection, Command Mode writes the requested text at the cursor.
- Run `yap gui`, launch **Yap** from the app launcher, or use the tray icon for health, settings,
  history, snippets, restart, and quit controls.

Settings and history are stored privately under `${XDG_DATA_HOME:-~/.local/share}/yap`. Temporary
WAV files and logs use `$XDG_RUNTIME_DIR/yap`, are user-only, and captured audio is removed after
processing. Selection text is held in memory only; the clipboard is restored unless it changed
concurrently.

### Linux troubleshooting

```sh
yap doctor
yap gui
systemctl --user status yap.service yap-overlay.service yap-tray.service
journalctl --user -u yap.service -b --no-pager
```

The model-server logs contain runtime/backend diagnostics but no transcript:

```sh
tail -n 80 "$XDG_RUNTIME_DIR/yap/whisper-server.log"
tail -n 80 "$XDG_RUNTIME_DIR/yap/llama-server.log"
```

Package updates belong to pacman or your AUR helper, for example `paru -Syu`. To uninstall while
leaving private models/settings available for a later reinstall:

```sh
systemctl --user disable --now yap.service yap-overlay.service yap-tray.service
paru -Rns yap-dictation
```

Delete `${XDG_DATA_HOME:-~/.local/share}/yap` yourself only if you also want to erase models,
settings, snippets, and history.

## macOS

Yap's macOS application uses WhisperKit on Apple Silicon and Apple's on-device Foundation Models
for cleanup. macOS 26+ is recommended; on macOS 14–15 transcription works with passthrough cleanup.

```sh
swift build
./Scripts/run.sh         # build a signed app bundle and launch it
./Scripts/build-app.sh   # build without launching
./Scripts/reset.sh       # clean first-run permission test
```

On first launch, grant Microphone, Accessibility, and Input Monitoring. Then hold the default
Right Shift binding to dictate, double-tap for hands-free mode, or hold Right Option for Command
Mode. The menu-bar icon opens settings, history, and snippets.

macOS releases update through Sparkle. Maintainers create a notarized release with
`./Scripts/release.sh` and a configured Developer ID certificate and `YapNotary` keychain profile.

## How it stays local

On Linux, `whisper-server` and `llama-server` bind only to loopback. On macOS, WhisperKit and
Foundation Models run in-process. Normal dictation, cleanup, Command Mode, snippets, and history do
not make network requests. Network access is limited to explicit model installation and normal
package/update tooling.

## License

[MIT](LICENSE) © Kyle Scudder.

Built with WhisperKit, whisper.cpp, llama.cpp, Qwen3, Apple Foundation Models, PipeWire, GTK, and
Wayland.
