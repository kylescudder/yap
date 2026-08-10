# Yap

Local-first voice dictation for macOS — a privacy-respecting clone of Wispr Flow.
Hold a hotkey, speak, and clean formatted text is inserted into whatever app you're in.
**Everything runs on-device: no cloud, no API keys, no cost.**

- Research: [`docs/RESEARCH.md`](docs/RESEARCH.md)
- Build plan: [`docs/IMPLEMENTATION_PLAN.md`](docs/IMPLEMENTATION_PLAN.md)

## Status: Phase 1 — core dictation loop (mock engines)

The full loop is wired end-to-end and runs today:

**Hold Right ⇧ → speak → release → text is pasted at your cursor in any app.**

- `HotkeyManager` — global push-to-talk via `CGEventTap` (hold Right Shift)
- `AudioRecorder` — `AVAudioEngine` capture, resampled to mono 16 kHz Float
- `DictationOverlay` — non-activating floating HUD (never steals focus) with a live level meter
- `TextInserter` — clipboard → synthetic ⌘V → restore (explicit Command-only flags)
- `DictationController` — orchestrates capture → transcribe → clean → insert
- Plus Phase 0: menu-bar agent, permission onboarding, settings with engine pickers

Transcription/cleanup still use the **mock engines**, so dictating pastes a placeholder like
`(mock transcript — N samples)`. That confirms hotkey, capture, overlay, and injection all work.
**Next: drop in WhisperKit (Stage 1) and MLX / Foundation Models (Stage 2).**

## Requirements

- macOS 14+ (developed on macOS 26, Apple Silicon)
- Swift 6 toolchain (Command Line Tools is enough for Phase 0)
- Full **Xcode** will be needed later for WhisperKit / MLX / FoundationModels

## Build & run

```sh
swift build                # compile
./Scripts/run.sh           # build .app bundle + launch (mic icon appears in the menu bar)
./Scripts/build-app.sh     # build the bundle without launching
```

The app has no Dock icon — it lives in the menu bar. On first launch it opens the
permissions onboarding.

> Ad-hoc code signing changes identity per build, so macOS may re-prompt for permissions
> after a rebuild. That resolves once the app is signed with a stable Developer ID (Phase 6).

## Layout

```
Sources/Yap/
  main.swift              # entry point (menu-bar agent)
  App/                    # AppDelegate, MenuBarController
  Permissions/            # PermissionsManager + onboarding UI
  Settings/               # AppSettings store + settings UI
  Pipeline/               # Transcriber / Cleaner protocols + mocks
  Support/                # logging
Bundle/                   # Info.plist, entitlements (for .app assembly)
Scripts/                  # build-app.sh, run.sh
docs/                     # research + implementation plan
```
