# Linux parity and release gates

The Linux port is an internal alpha until every required row in this document is complete. The
Arch package may be built locally for testing, but `yap-dictation` must not be published to the AUR
before the release gates at the end of this document pass.

Parity means the same user-visible capability as the macOS application, implemented in a way that
fits Linux and Hyprland. It does not require copying an Apple-specific mechanism where the Linux
platform has a better native convention.

## Intentional platform differences

- Hyprland hotkeys remain in the user's compositor configuration. Yap exposes press and release
  commands and never claims a modifier or rewrites a user-owned binding.
- Installation and updates belong to `pacman`/an AUR helper. Yap does not ship an in-app updater.
- Linux uses PipeWire, Wayland, D-Bus, and a locally hosted model instead of Apple frameworks.

## Capability matrix

| Capability | macOS | Linux status | Linux release requirement |
| --- | --- | --- | --- |
| Hold-to-dictate | Complete | Complete | Preserve press/release behavior in real applications |
| Quick-tap cancellation | Complete | Complete | Automated state tests and a real hotkey test |
| Double-tap hands-free mode | Complete | Core complete | Validate through a user-owned Hyprland binding |
| Local transcription | WhisperKit | Complete with whisper.cpp | CPU baseline, optional acceleration, no network in the dictation path |
| Text insertion | Accessibility paste | Complete with `wtype` | Clipboard fallback and multiline/non-ASCII validation |
| Recording overlay | Live waveform pill | State-driven pill complete | Non-focus-stealing recording, locked, processing, and error states |
| Live microphone level | Complete | Implemented; hardware validation pending | Drive the recording waveform from captured audio levels |
| Tray/menu application | Complete | Implemented; desktop validation pending | Open status, settings, history, snippets, diagnostics, and quit/restart actions |
| First-run onboarding | Complete | Implemented; clean-install validation pending | GUI health/setup flow with model and binding guidance |
| Settings | Complete | Implemented | Persist and edit all portable preferences |
| Command Mode | Complete | Implemented; end-to-end validation pending | Local instruction processing, selection capture, and replacement |
| Context-aware cleanup | Apple Foundation Models | Implemented with local Qwen3; quality validation pending | Fully local Linux model with equivalent cleanup levels and fallback behavior |
| Per-application tone | Complete | Implemented | Focused-application context feeds cleanup without retaining private content |
| Output polishing | Complete | Implemented | Port preamble stripping, courtesy trimming, and empty-output handling |
| Snippets | Complete | Implemented | Local persistent CRUD, expansion, and GUI |
| Dictation history | Complete | Implemented | Local capped history, clear/copy actions, and GUI |
| Audio handling | Off/lower/pause | Implemented; playback validation pending | PipeWire-native off/lower/pause behavior with reliable restoration |
| Permissions and health | Onboarding/settings | `yap doctor` complete | Surface actionable health in the GUI without recording content |
| Model lifecycle | First-run download | Implemented; full download validation pending | GUI install/progress, verification, recovery, and disk-use visibility |
| Privacy | Local-only | Implemented; final audit pending | Cleanup and commands local; audit logs/reports for captured content |
| Packaging | Signed app/DMG | Production manifest complete; package lifecycle covered in CI | Clean install, upgrade, uninstall, CPU-only, and NVIDIA validation |
| Updates | Sparkle | Intentional difference | Document `pacman`/AUR-helper updates |
| User documentation | Complete | Draft complete; clean-system walkthrough pending | Linux install, setup, usage, privacy, troubleshooting, update, and uninstall guide |

## Module seams

The daemon owns session truth, persistence, and the dictation pipeline. It publishes one status
interface over the user session bus. The overlay, tray, settings window, CLI, and tests are adapters
at that seam; they must not reimplement timing or pipeline policy.

Platform-specific capture, insertion, focused-application context, media control, transcription,
and language-model work stay behind narrow daemon interfaces. Pure cleanup, snippets, history, and
output-polishing policy should remain independent of GTK, D-Bus, PipeWire, and process launching.

## AUR release gates

Publication is allowed only when all of the following are true:

Validate the production package on a real Arch/Hyprland desktop. Automated checks cover the state
machine, package contents, and privacy invariants; hands-on use must still cover the hotkey, overlay,
insertion, Command Mode, GPU and CPU inference, clean onboarding, and package lifecycle paths.

1. Every required capability above is complete or recorded as an intentional platform difference.
2. The complete dictation and Command Mode paths work from user-owned Hyprland bindings.
3. The overlay and GUI have been exercised on a real multi-workspace Hyprland session without
   stealing focus.
4. Dictation, cleanup, commands, snippets, and history remain local and captured user content is
   absent from normal logs and diagnostic reports.
5. Automated tests and the Arch package workflow pass.
6. Clean install, upgrade, uninstall, CPU fallback, and NVIDIA acceleration have been validated.
7. The user-facing Linux README is complete and has been followed successfully from a clean system.
8. Only after gates 1–7 pass is `yap-dictation` published to the AUR.
