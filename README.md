# Yap

**Local-first voice dictation for macOS.** Hold a key, speak, and clean, formatted text is
inserted into whatever app you're in — Slack, Mail, your editor, anywhere you can type.

Everything runs **on-device**: no cloud, no accounts, no API keys, no subscription. Your voice
never leaves your Mac. It's a privacy-respecting take on Wispr Flow.

## Features

- **Dictate into any app** — global push-to-talk; the cleaned text is pasted at your cursor.
- **Hands-free mode** — double-tap the hotkey to lock a continuous session; press once to stop.
- **Command Mode** — hold a second key and speak an instruction (e.g. *"make this more formal"*,
  *"turn into bullet points"*, *"translate to Spanish"*). Selected text is rewritten in place;
  with nothing selected, text is generated at the cursor.
- **Custom hotkeys** — record any key or chord for push-to-talk and Command Mode.
- **Context-aware cleanup** — filler removal, punctuation, and tone matched to the app you're in
  (casual in chat, formal in email, code-preserving in editors), with a 5-level intensity slider.
- **Snippets** — spoken triggers expand to saved text.
- **Dictation history** — recent dictations, stored locally.
- **100% offline & private** — on-device transcription and cleanup.

## How it works

A two-stage on-device pipeline:

1. **Transcription** — [WhisperKit](https://github.com/argmaxinc/WhisperKit) running Whisper
   `large-v3-turbo` (CoreML / Apple Neural Engine).
2. **Cleanup** — Apple's on-device **Foundation Models** (macOS 26) rewrite the raw transcript:
   remove fillers/false starts, fix punctuation and casing, resolve self-corrections, and match
   the target app's tone.

The result is inserted at your cursor via the Accessibility API. Global hotkeys use a
`CGEventTap`; audio is captured with `AVAudioEngine`.

## Requirements

- **macOS 26+** recommended (on-device cleanup uses the Foundation Models framework; on macOS 14–15
  transcription still works and cleanup falls back to the raw transcript).
- **Apple Silicon** recommended.
- Full **Xcode** or the Command Line Tools to build.
- ~1.3 GB Whisper model is downloaded once on first run (then fully offline).

## Build & run

```sh
swift build              # compile
./Scripts/run.sh         # build a signed .app bundle and launch it
./Scripts/build-app.sh   # build the bundle without launching
./Scripts/reset.sh       # revoke permissions + wipe prefs for a clean first-run test
```

Yap is a menu-bar agent (no Dock icon) — look for the mic icon after launch. `build-app.sh`
auto-detects a code-signing identity so macOS permission grants persist across rebuilds.

### Permissions

On first launch Yap asks for three permissions (all local, all needed to dictate into any app):

- **Microphone** — capture your voice for on-device transcription
- **Accessibility** — read the focused field and insert text
- **Input Monitoring** — detect the push-to-talk hotkey system-wide

Manage them any time from the menu-bar icon → **Settings → Permissions**.

## Usage

- **Hold** your push-to-talk key (default **Right ⇧**) → speak → release → text is inserted.
- **Double-tap** it → hands-free lock; **press once** to stop.
- **Hold** your Command Mode key (default **Right ⌥**) → speak an instruction → transforms the
  selection (or generates at the cursor).
- Configure hotkeys, cleanup intensity, snippets, and permissions in **Settings**.

## Privacy

Dictation and cleanup run entirely on-device — there are no network calls in the dictation path.
The only network access is the one-time WhisperKit model download from Hugging Face.

## Auto-update

Releases ship via [Sparkle](https://sparkle-project.org). The app checks
`https://github.com/kylescudder/yap/releases/latest/download/appcast.xml`. Maintainers cut a
notarized, auto-updating release with:

```sh
./Scripts/release.sh            # build → Developer-ID sign → notarize → staple → Sparkle-sign → publish to GitHub Releases
```

(Requires a *Developer ID Application* certificate and a `YapNotary` notarytool keychain profile.)

## License

[MIT](LICENSE) © Kyle Scudder.

Built with [WhisperKit](https://github.com/argmaxinc/WhisperKit), Apple Foundation Models, and
[Sparkle](https://github.com/sparkle-project/Sparkle). Inspired by Wispr Flow.
