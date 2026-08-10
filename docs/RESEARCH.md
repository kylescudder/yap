# Yap — Wispr Flow Research & Build Brief

> Goal: build the best-in-class clone of Wispr Flow ("Yap"). This is the foundational
> research + technical-decision brief. Claims are tagged **[confirmed]** (official/credible
> source) or **[inferred]** (reasoned). Sources are listed per section.

---

## 0. TL;DR — the decisions that matter

1. **The magic is the LLM cleanup pass, not the ASR.** Raw transcription is commoditized in
   2026. What makes Wispr feel magical — no fillers, correct punctuation, lists, app-aware
   tone, self-correction ("5… actually 6") — is a **single small fine-tuned LLM rewrite pass**
   over the transcript. Build this on day one on top of any off-the-shelf ASR. [confirmed]
2. **Latency is the product.** Wispr's hard target is **≤700 ms p99 from end-of-speech to
   pasted text** (ASR <200 ms, LLM <200 ms, network <200 ms). They optimize the **tail (p99)**,
   not the average. Adopt the same discipline. [confirmed]
3. **Wispr's structural weakness is cloud-only.** No offline mode at any price; audio + screen
   context leave the device; privacy is opt-**out** by default. This is the wedge a clone can
   own: **local-first / privacy-by-default**, which Wispr cannot easily copy. [confirmed gap]
4. **Hardest engineering is OS integration, not AI.** Global hotkey capture, detecting the
   focused text field, and reliably injecting text into *any* app (incl. the modifier/secure-field
   edge cases) is where the real work is. [confirmed]
5. **Recommended stance for Yap:** local-first (whisper.cpp on Apple Silicon) + optional BYO-key
   cloud, **macOS-native menu-bar agent** first, **developer-friendly** wedge, priced to undercut
   ($8/mo + a lifetime option), privacy-by-default. [inferred strategy]

---

## 1. What Wispr Flow is

**[confirmed]** Wispr Flow (wisprflow.ai) lets you **dictate anywhere you can type**, ~4× faster
than typing (~220 wpm vs ~45). Founded 2021 (Tanay Kothari, Sahaj Garg); pivoted from a wearable
to software; launched 2024. ~$81M raised (Series A Menlo Ventures, ext. Notable Capital), with a
reported ~$260M Series B at ~$2B (May 2026, treat as reported). Processes ~1B words/month.

**End-to-end loop [confirmed]:**
1. Press and hold a global hotkey (default: hold `Fn`/Globe on Mac, `Ctrl+Win` on Windows).
2. Speak naturally — filler, pauses, self-corrections are fine.
3. Release the key.
4. Audio → cloud ASR → transcript → **fine-tuned Llama LLM** cleans + formats + adapts to the
   active app → text is **auto-pasted into the focused field** of whatever app is in front.

Platforms: **macOS 12+, Windows 10/11 (x64 only), iOS 18.3+ (keyboard extension), Android 13+.**
No iPad, Linux, Chromebook, VM, or remote-desktop support. **No offline mode.**

Sources: wisprflow.ai, docs.wisprflow.ai/articles/2772472373-what-is-flow,
en.wikipedia.org/wiki/Wispr_Flow, baseten.co/resources/customers/wispr-flow/

---

## 2. Feature inventory (with Yap build priority)

Priority key: **[MVP]** ship first · **[V1]** parity where it matters · **[Later]** upsell/B2B/scope-expansion.

| Feature | What it does | Priority |
|---|---|---|
| Push-to-talk hotkey | Hold key → dictate → release → paste. Up to 4 shortcuts / 3 keys each; can bind mouse buttons. | **MVP** |
| Cross-app text insertion | Paste cleaned text into the focused field of any app. | **MVP** |
| LLM cleanup pass | Remove "um/uh", auto-punctuation, capitalization, lists, paragraphs; 4 intensity levels (None/Light/Medium/High). Claims 90% "zero-edit" rate. | **MVP** |
| Self-correction ("Backtrack") | "at 5 actually 6" → "at 6"; handles restarts/"scratch that" without transcribing them literally. | **MVP** |
| Custom dictionary | Word boosting (ASR bias) + replacement rules; auto-learns from user edits; starred priority terms; CSV import (≤1000); syncs across devices. | **MVP** |
| Dictation history | Transcript history; audio playback (14 days); "never lose a dictation" recovery. | **MVP** |
| Sequential permission onboarding | Permission cards granted one at a time, deep-linking to System Settings; re-verify after updates. | **MVP** |
| Hands-free mode | Continuous dictation without holding a key; double-tap to lock a session; 20-min desktop cap. Needs VAD/endpointing + persistent stream. | **V1** |
| Per-app tone/format rules | Detect frontmost app → adapt tone (email formal, Slack casual) & formatting. English/desktop only in Wispr. | **V1** |
| Snippets / voice shortcuts | Speak a cue → paste a saved (rich-text) block; shared snippets for teams. | **V1** |
| Command Mode | Separate hotkey. With selection: transform ("make formal", "translate", "bullet points"). Without: generate/answer inline at cursor. "Press enter" to submit. Paid in Wispr. | **V1** |
| Multi-language | 100+ languages (104), auto-detect per session (not per word), quick-switch UI. 7 fully-optimized (ES/FR/DE/HI/IT/PT/TH). | **V1** |
| Whisper/quiet mode | Works when whispering (accuracy dips ~92–95%). Mostly an ASR-robustness property. | **V1** |
| Context awareness | Reads frontmost app + browser URL + text around caret + selection to improve spelling/format. Screenshot/OCR path is opt-in after backlash. | **V1** (accessibility-text only; screenshot **opt-in** if ever) |
| Developer features | camelCase/snake_case, CLI/jargon recognition, filename tagging in Cursor/Windsurf/VS Code. | **V1** (strong wedge) |
| Custom transforms | User-defined transforms ("Polish", "Prompt Engineer") with auto-apply. | **Later** |
| Teams/Enterprise | Shared dictionary/snippets, SSO/SAML, SCIM, audit logs, MDM, HIPAA BAA, dashboards. | **Later** |
| Notetaker | Meeting recording + diarization + summaries + "ask anything"; MCP integration. Separate product surface. | **Later** |
| Voice-profile insights | Archetype, catchphrase, most-corrected word; shareable cards. Cheap gamification. | **Later** |

Sources: wisprflow.ai/features, /why-flow, /whats-new; docs.wisprflow.ai (command-mode,
hands-free, context-awareness, smart-formatting-and-backtrack, dictionary, multiple-languages).

---

## 3. Technical architecture

### 3.1 System pipeline

```
  [Global hotkey / PTT]            (CGEventTap mac / WH_KEYBOARD_LL win)
          │  key-down → arm; open WS warm; pre-warm audio engine
          ▼
  [Audio capture]                  AVAudioEngine tap → PCM16 mono 16 kHz
          │  stream chunks (100–250 ms) while speaking
          ▼
  [ASR]  ── partials ──▶ overlay   cloud (Deepgram/AssemblyAI) OR local (whisper.cpp/Parakeet)
          │  key-up → endpoint (VAD) → final transcript
          ▼
  [Context collector]              frontmost app + URL + text-around-caret + selection
          │  (local; exclude secure/password fields)
          ▼
  [LLM cleanup pass]               small fine-tuned ~7–8B model, ONE forward pass
          │  filler removal, punctuation, lists, casing-at-cursor, tone, self-correction
          ▼
  [Deterministic post-layer]       app-specific quirks (trailing-period strip, dictionary replace)
          │
          ▼
  [Insert at cursor]               AX direct-set if supported, else clipboard→⌘V→restore
```

The two-stage **ASR → LLM cleanup** shape is the whole architecture. Everything else is UX and
OS plumbing around it. [confirmed from Baseten case study + Wispr tech post]

### 3.2 Client & OS integration (the hard part)

**Framework** — Wispr ships an **Electron** app on Mac (`com.electron.wispr-flow`), which is why
reviewers see ~800 MB RAM / ~8% idle CPU and Windows freezing target apps. [confirmed]
Electron/Tauri UI cannot do the OS-level work alone — global taps, Accessibility reads, and
synthetic paste all need **native code** (Obj-C/Swift/Rust/C).

> **Yap recommendation:** avoid Electron's footprint. **Native Swift/AppKit menu-bar agent on
> macOS** (tens of MB idle) with a shared Rust/C++ core for ASR+orchestration, or **Tauri** if one
> codebase is mandatory. Open-source analogues (`koe`, `clicky`, `VoiceInk`) are native for exactly
> this reason. "Low idle RAM/CPU, never freezes your editor" is a marketable feature vs Wispr.

**Global hotkey (macOS):**
- Use **`CGEventTap`** on `flagsChanged | keyDown | keyUp` (the only API that sees key-up/down
  separately for hold-vs-tap, and can *swallow* the event). Requires **Input Monitoring** permission.
- Carbon `RegisterEventHotKey` can't bind bare modifiers or `Fn`; `NSEvent` global monitors can't
  consume events. Neither is enough for PTT. [confirmed]
- **`Fn`/Globe caveat:** short-press Fn is handled by WindowServer below the tap layer. Long-press
  Fn (keycode 63) is observable via `flagsChanged`. To ship Fn-as-default, detect `AppleFnUsageType`
  and guide the user to set Globe → "Do Nothing"; otherwise default to a right-modifier chord.
- State machine: on key-down don't commit; release within ~180 ms = tap, longer = hold.

**Detecting where text lands:**
- Frontmost app: `NSWorkspace.shared.frontmostApplication` → `bundleIdentifier` (also drives per-app rules).
- Focused field: **Accessibility API** — `AXUIElementCreateSystemWide()` →
  `kAXFocusedUIElementAttribute`; check `kAXRoleAttribute` for `AXTextField`/`AXTextArea`.
- Requires **Accessibility** permission. **App Sandbox blocks the AX API** → ship outside the Mac App Store.

**Inserting text into any app** (three strategies, real tradeoffs):

| Method | Pros | Cons |
|---|---|---|
| **Clipboard + synthetic ⌘V + restore** (default) | Works nearly everywhere; perfect Unicode/emoji; fast for long text; single undo | Stomps clipboard (must restore on delay ~1.5 s); clipboard managers may capture; fails in secure fields |
| **Synthetic keystrokes** (`CGEventKeyboardSetUnicodeString`) | No clipboard | Slow; timing-sensitive; **held hotkey modifiers corrupt output** (must zero event flags); IME edge cases |
| **AX direct-set** (`kAXValueAttribute`/`kAXSelectedText`) | Cleanest, no clipboard, no races | Only where app implements AX text correctly (many Electron/web/Java apps don't) |

> **Yap recommendation:** try **AX direct-set first** when the element cleanly supports it, else
> **clipboard→⌘V→restore-on-delay**. Always **zero the modifier flags** on injected events. **Detect
> secure input** (`IsSecureEventInputEnabled`) and refuse gracefully — no method types into a password field, by design.

**Audio + overlay:**
- Capture: **`AVAudioEngine`** input tap → resample to PCM16 mono 16 kHz → stream. Recreate the
  engine per session (stale Bluetooth device refs); pre-warm to kill first-press cold start.
- Overlay: **non-activating, borderless `NSPanel`** (`.nonactivatingPanel`, `level = .statusBar`,
  `canJoinAllSpaces`, `ignoresMouseEvents`) showing a waveform + interim transcript. **It must
  never steal focus** or the paste lands in the wrong field.

**Permissions & packaging (macOS):**
- Needs **Microphone**, **Accessibility**, **Input Monitoring** (each a separate TCC prompt; the
  latter two can't be code-granted — deep-link to System Settings and poll `AXIsProcessTrusted()`).
- Ship as **`LSUIElement` menu-bar agent** (`NSStatusItem`), **Developer-ID signed + notarized +
  Hardened Runtime**, direct download (not MAS). Build proactive **re-verify-after-update** flow
  (Wispr's permissions silently break on update — a known pain point).

**Windows equivalents:** `WH_KEYBOARD_LL` hook for PTT (fall back to `RegisterHotKey` for chords);
`SendInput` with `KEYEVENTF_UNICODE` (or clipboard-paste for long text); `GetForegroundWindow` +
**UI Automation** (`TextPattern`/`ValuePattern`) to read/target the focused control. Keep the same
abstraction (`Hotkey` / `Insert` / `FocusInspector` interfaces) across both OSes.

Sources: macupdater.net (com.electron.wispr-flow), Apple dev forums (AXUIElement, secure input),
github.com/missuo/koe, github.com/farzaa/clicky, github.com/10xChengTu/input0, learn.microsoft.com (UI Automation).

### 3.3 ASR layer

**Wispr** runs a cloud **ensemble** of ASR models with per-language selection (mentions Whisper,
ElevenLabs Scribe, Gemini), migrating toward **proprietary personalized models** (claims ~10% WER
vs Whisper 27%, self-reported). ASR inference budget <200 ms. **Cloud-only.** [confirmed/inferred]

**2026 options for Yap:**

*Cloud (fastest to parity):*
| Provider | Streaming latency | WER (indep.) | Price |
|---|---|---|---|
| AssemblyAI Universal-3 | ~307 ms P50 | ~7–8% | ~$0.0075/min stream |
| Deepgram Nova-3 | ~450–516 ms P50 | ~9.9% | $0.0077/min stream, $0.0043 batch |
| OpenAI (gpt-4o-transcribe / Whisper API) | realtime | ~8.9% (weak on hard domains) | $0.006/min |
| Speechmatics | streaming | ~6.4% (accuracy leader) | premium |

*Local (privacy/offline — the differentiator):*
| Model | Speed | WER | Best for |
|---|---|---|---|
| **whisper.cpp** (Metal) | ~10× realtime on Apple Silicon; built-in streaming+VAD | =Whisper | **On-device Mac dictation** |
| faster-whisper (CTranslate2) | ~12× realtime on RTX 4070 (~2.5 GB VRAM); CPU too slow (RTF ~2.5) | =Whisper | Self-host GPU |
| Whisper large-v3-turbo | several× faster than large-v3, ~same accuracy | ~7.5% | Balanced local |
| NVIDIA Parakeet TDT 0.6B v3 | RTFx >2000, ~10× faster than Whisper | ~6.3% (best) | High-throughput, English-strong |
| Moonshine | ultra-low latency, tiny | ~6.6% | Edge/phone |

> **Yap recommendation:** keep ASR **pluggable behind one interface**. Default to **whisper.cpp
> (Metal) on Mac** for a genuine offline/privacy mode Wispr can't match; offer **BYO-key cloud**
> (Deepgram/AssemblyAI) as an accuracy option. **Push-to-talk batch** (transcribe the whole
> utterance on release) is the right default — simpler and more accurate than streaming; reserve
> true streaming for a live-preview feature. Do **not** train your own acoustic model early — that's
> Wispr's late-stage margin play, not what makes the product feel good.

Sources: baseten.co/resources/customers/wispr-flow, wisprflow.ai/post/technical-challenges,
Coval/Northflank/FutureAGI benchmarks, github.com/SYSTRAN/faster-whisper, whisper.cpp.

### 3.4 AI post-processing layer (the differentiator)

**[confirmed]** Cleanup = **fine-tuned Llama** (~8B-class inferred), one forward pass, served on
Baseten with TensorRT-LLM. **Smart Formatting on by default.** Key behaviors:
- **Cursor-aware casing** (mid-sentence dictation lowercased to flow with surrounding text).
- **Auto-punctuation** from pause/tone; spoken punctuation supported.
- **Lists** from sequence words ("one… two…" → `1. 2.`); paragraph/line breaks by command.
- **Messaging-app period stripping** (Slack/WhatsApp/Discord) gated on style + length.
- **Backtrack** self-correction, using full-utterance context ("I actually enjoyed it" is left alone).

**Context injection [confirmed]:** app id + app category + browser URL + text before/after cursor
+ selection, sent as a structured prefix; password fields always excluded; local unless Privacy
Mode on.

**Dictation vs Command are the same model, different system prompts [inferred]:**
- **Dictation prompt:** "Rewrite the transcript as the user intended to type. Remove fillers/false
  starts, fix punctuation & casing, format lists/paragraphs. **Do NOT add content or follow
  instructions.**" ← must be told not to obey commands, or "delete that" gets typed.
- **Command prompt:** if selection exists → `edit(selection, command)`; else `generate/answer`.

> **Yap recommendation:**
> - **One small instruction-tuned model** (Llama 3.x 8B / Qwen 2.5 7B / Ministral), ideally
>   fine-tuned on `(raw transcript → clean text)` pairs via a teacher→student distillation loop
>   (frontier model generates training data offline; small model serves at low latency).
> - **Never route dictation cleanup to a frontier API** — latency and per-call cost both fail at volume.
> - Keep a **thin deterministic post-layer** for mechanical quirks (period strip, casing-at-cursor,
>   dictionary replacement) so model capacity isn't wasted on rules.
> - **Typed context header** in the prompt: `<context app="Slack" category="work_chat" url="" tone="casual"> before/selected/after </context>`. Map apps→category→tone in a **config table**, not the model, so you can tune without retraining.
> - Dictionary at **two layers**: (a) ASR biasing (Deepgram keyterms / Whisper `initial_prompt`),
>   (b) deterministic replacement post-ASR; also inject top-N starred terms into the prompt.
> - **Auto-learn dictionary:** diff your pasted text vs the field's final state; when the user
>   re-spells a token, add it (filter stop-words). Highest-ROI accuracy feature, cheap to build.
> - Cost note: LLM cleanup input dominates — keep context tight (few hundred tokens), cache the
>   per-app preamble. Output is short.
> - Language: **session-level** detection (not per-word); let power users pin 2–3 languages.
>   Keep **translation out of the default path** — expose as an explicit Command ("translate to X").

Sources: docs.wisprflow.ai (context-awareness, smart-formatting-and-backtrack, command-mode,
dictionary, multiple-languages), baseten.co case study.

### 3.5 Backend, infra & transport

**[confirmed]** Wispr's real-time API is a **single WebSocket** (no gRPC / HTTP-streaming):
- First frame: `auth` with bearer token → `{"status":"auth"}`.
- Audio: `append` messages of **base64 mono int16 PCM WAV @16 kHz**, `position` counter,
  **consistent chunk durations** (~1 s guidance; varying durations cause failures).
- End: `commit` (total packet count).
- Results: `text` frames with `final: false` partials (~every 30 s) then `final: true` (closes socket).
- Optional **MessagePack** binary framing (~30% bandwidth win).
- Two auth classes: long-lived server API key vs short-TTL client key minted per session.

> **Yap recommendation:** WebSocket is the right call — one persistent, firewall-friendly,
> bidirectional connection. **Open it warm on hotkey-arm**, not on speech-start, so TLS+WS handshake
> is outside the latency budget. Use 16 kHz mono PCM16, but **smaller chunks (100–250 ms)** than
> Wispr's 1 s for snappier partials. Copy MessagePack. Mint short-TTL client tokens from your
> backend so the long-lived key never ships to the client. Split sync into **always-sync config**
> (dictionary/snippets/settings) vs **privacy-gated content** (history/transcripts) so a ZDR mode
> can drop the latter without breaking the app.

Sources: api-docs.wisprflow.ai/websocket_api, github.com/shmbhvi101/wispr-flow/ARCHITECTURE.md.

---

## 4. Latency budget (optimize the tail, not the average)

| Stage | Budget | Notes |
|---|---|---|
| Capture/buffer | 20–256 ms | smaller chunks = snappier partials |
| Network RTT (cloud) | 20–80 ms | client-direct WS avoids a proxy hop; 0 for local ASR |
| ASR first partial | ~150 ms US / 250–350 ms global | stream partials while speaking → perceived latency ≈ 0 |
| **Endpointing (silence → final)** | **300–800 ms** | **biggest hidden cost & biggest lever — make it tunable** |
| LLM cleanup | ~150–400 ms first token + gen | can't start until final transcript |
| Insert at cursor | <50 ms | AX API faster than clipboard |

The user never waits on the *partial* path (text appears as they speak). Real perceived latency is
the **tail after they stop**: endpoint + LLM + insert. Levers: aggressive/tunable endpointing,
small fast cleanup model, warm connections, co-locate ASR+LLM in one region, stream cleanup tokens.
Wispr's "priority processing" on Pro is essentially a paid latency SLA. [confirmed anchors + inferred]

---

## 5. Privacy & security (Wispr's most-attacked flank = Yap's opening)

**[confirmed]** Wispr is **cloud-only**; two independent controls:
- **Privacy Mode** (governs *training*) — **default OFF** for standard/trial users (i.e. your
  dictation may be used to improve their models).
- **Private Cloud Sync** (governs *server storage*) — default ON.
- **ZDR** = Privacy Mode ON + Cloud Sync OFF; enterprise-enforceable.
- In transit TLS 1.2+; at rest AES-256, FIPS 140-2 HSM keys. **US-only residency, no EU region.**
- SOC 2 Type I (Apr 2026, A-LIGN, clean); Type II observation still running as of Aug 2026; ISO
  27001 Stage 1; signable HIPAA BAA. (Note: an earlier SOC 2 via the Delve/ACCORP ecosystem was
  invalidated Mar 2026 — a trust wound.)
- **Reputational wounds:** 2025 backlash when Context Awareness silently captured window
  **screenshots** every few seconds → CTO apologized, made it opt-in. App can read keystrokes.
  11+ subprocessors (PostHog session replay, Sentry, Supabase, OpenAI/Anthropic/Meta, Cerebras).

> **Yap recommendation — differentiate hard on privacy:**
> - **Privacy-by-default**: no training on user data by default; Privacy Mode **ON** out of the box.
> - Ship a **genuine on-device/offline mode** (removes the entire subprocessor surface + the
>   latency tax) — Wispr structurally cannot match this today.
> - Context via **accessibility text only**; if you ever add screenshot/OCR, make it **explicit
>   opt-in from launch** (Wispr's biggest wounds came from doing it silently).
> - Always **exclude secure/password fields**.
> - For enterprise: put no-training + retention promises **in the DPA/contract**, ship
>   admin-locked org policy, SSO/SCIM, audit logs, and a **ZDR org lock**. An **EU data-residency
>   region** is a concrete gap you can beat Wispr on for UK/EU buyers.

Sources: docs.wisprflow.ai/articles/3467817258 (security FAQ) + 4709791908 (privacy mode),
wisprflow.ai/data-controls, getvoibe.com/resources/is-wispr-flow-safe, HN 47781148.

---

## 6. Cost model

**[inferred, ~150 wpm ≈ 200 transcript tokens/min]**
- ASR ≈ **$0.005/min** (dominant cost).
- LLM cleanup (4o-mini-class, ~1000 in / 250 out) ≈ **$0.0003/min** (nearly free).
- **Total variable COGS ≈ $0.005–0.006/min ≈ $0.30–0.36/hr.**

Mapping to Wispr's pricing (Pro $12–15/mo): a ~30 min/day user ≈ 660 min/mo ≈ ~$3.5 COGS (healthy
margin); a ~2 hr/day power user ≈ 2600 min/mo ≈ ~$13–16 COGS (roughly break-even at Pro price). This
is why "unlimited" works only with long-tailed usage, and why "priority processing" doubles as a
soft throttle.

> **Yap recommendation:** COGS floor is **ASR, not the LLM** — pick ASR deployment carefully
> (self-hosted whisper/Parakeet on your GPUs beats $0.005/min at scale). **On-device mode has
> near-zero marginal COGS**, which can fund a genuinely unlimited free tier or a lower price.
> Use a cheap fast cleanup model — spend the savings on latency, not a bigger model. Model the
> heavy-user cliff (~2 hr/day ≈ break-even at $12–15) and soft-throttle/meter above a fair-use line.

Sources: Deepgram pricing, GPT-4o-mini / Claude Haiku pricing, eesel.ai/blog/wispr-flow-pricing.

---

## 7. Competitive landscape & positioning

| Tool | Differentiator | Price | Key weakness |
|---|---|---|---|
| **Wispr Flow** | Best AI reformatting, category leader, enterprise compliance | $15/mo ($12 annual) | Cloud-only, no offline, priciest, opt-out privacy, reliability complaints post-trial |
| **Superwhisper** | **On-device** option; cheapest | $8.49/mo, **$249.99 lifetime** | Mac/iOS-focused; less reformatting polish |
| **Aqua Voice** | Tuned for **technical/coding vocab** | $8/mo | Cloud-only; tiny company |
| **Talon Voice** | Full voice **control** + coding (accessibility gold standard) | Free / ~$25 Patreon | Steep learning curve |
| **VoiceInk** | **Open-source (GPL v3)**, 100% local (whisper.cpp), per-app Power Mode | **$39.99 one-time** | Mac-only; setup effort |
| **Willow Voice** | Near Wispr clone; low latency (~200 ms claimed) | $15/mo | Cloud-only; me-too |
| **macOS/Windows built-in** | Free, on-device | Free | No AI reformatting/flow |
| **Otter.ai** | Meeting transcription | $16.99/mo | Different job |
| **Dragon** | Legacy pro (medical/legal), offline | from $699 one-time | Mac abandoned; dated |

**Wispr's exploitable gaps:** (1) cloud-only / no offline — the biggest structural gap; (2) opt-out
privacy defaults; (3) most expensive, no lifetime option; (4) post-trial reliability complaints +
high idle RAM/CPU + editor freezes; (5) uneven Windows; (6) weak on math/specialized input.

> **Yap positioning (pick the wedge Wispr can't take):**
> **"Wispr-quality dictation, but your voice never has to leave your machine."** Local-first +
> optional BYO-key cloud, developer-friendly, **~$8/mo + a lifetime option**, privacy by default,
> and reliability as a feature (low idle footprint, never freezes your editor).

Sources: superwhisper.com/vs/wispr-flow, digitalapplied.com, getvoibe.com reviews, Product Hunt reviews.

---

## 8. Recommended stack for Yap (starting point)

- **Shell:** native **Swift/AppKit menu-bar agent** (`LSUIElement`), shared **Rust** core for
  ASR+orchestration (portable to a Windows backend later). (Tauri if one codebase is mandatory; avoid Electron.)
- **Hotkey:** `CGEventTap` (mac) / `WH_KEYBOARD_LL` (win), hold-vs-tap state machine, event-swallow.
- **Targeting:** `NSWorkspace` + Accessibility API (mac) / `GetForegroundWindow` + UI Automation (win).
- **Insertion:** AX/UIA direct-set when supported, else clipboard→paste→restore; zero modifier flags; skip secure fields.
- **Audio:** `AVAudioEngine` streaming tap → PCM16 mono 16 kHz; non-activating `NSPanel` overlay; pre-warm.
- **ASR:** pluggable adapter. Default **whisper.cpp (Metal)** local; **BYO-key Deepgram/AssemblyAI** cloud option.
- **Cleanup LLM:** small fine-tuned ~7–8B (local via llama.cpp for offline, or hosted vLLM/TensorRT-LLM) + deterministic post-layer.
- **Transport (cloud path):** warm WebSocket, 16 kHz PCM16, 100–250 ms chunks, MessagePack, short-TTL client tokens.
- **Sync:** split always-sync config vs privacy-gated content; account + Stripe billing.
- **Packaging:** Developer-ID signed + notarized + Hardened Runtime, direct download; re-verify permissions after update.

---

## 9. Phased roadmap

**MVP (ruthless):**
1. System-wide push-to-talk hotkey → insert at cursor in any app.
2. Local transcription (whisper.cpp) — privacy default, offline works.
3. LLM cleanup pass (filler removal, punctuation, casing, lists, self-correction) — local small model or BYO-key cloud.
4. Personal custom dictionary (+ auto-learn from edits).
5. Dictation history. Sequential-permission onboarding + re-verify. One platform done well (macOS).

**V1 (parity where it matters):**
- Command Mode (highlight + speak to edit; inline generate).
- Per-app tone/format profiles. Hands-free mode (VAD/endpointing).
- Snippets/text-expansion. Cross-device sync of dictionary/snippets/settings.
- Multilingual + code-switching. Second platform (Windows or iOS).
- Reliability marketed as a feature: low idle RAM/CPU, no target-app freezing.

**Ambitious (differentiate, don't just clone):**
- Privacy-certified tier: on-device by default, provable ZDR, regulated-vertical story (HIPAA), EU residency.
- Deep developer/agent integration: MCP, IDE/terminal-aware modes, technical-vocab tuning (out-Aqua Aqua).
- Voice *control* (Talon-adjacent) but easy to set up — an accessibility angle Wispr ignores.
- Optional self-hosted / air-gapped enterprise deployment.

---

## 10. Decisions (resolved) — see `IMPLEMENTATION_PLAN.md`

1. **First platform:** macOS-native only (dev machine Apple M5 Pro, 48 GB, macOS 26.5). ✓
2. **ASR strategy:** 100% local, no cloud. Stage-1 engines all switchable for A/B: WhisperKit (large-v3-turbo, default), Parakeet (FluidAudio), Apple SpeechAnalyzer. ✓
3. **Primary wedge:** privacy / local-first — audio never leaves the machine. ✓
4. **Cleanup model:** local only, switchable: MLX small model (Qwen2.5-7B / Llama-3.1-8B) + Apple Foundation Models. No cloud path. ✓
5. **Business model:** none — personal, not for sale. Monetization/enterprise scope dropped (no billing, pricing, SSO, teams). ✓
6. **Stack:** pure Swift + AppKit (no Rust core; every engine has a Swift binding). ✓

---

## 11. Primary sources

- Wispr: wisprflow.ai (/, /features, /why-flow, /whats-new, /privacy, /data-controls, /post/technical-challenges, /research)
- Docs: docs.wisprflow.ai (what-is-flow, setup-guide, command-mode, hands-free, context-awareness, smart-formatting-and-backtrack, dictionary, multiple-languages, security-and-compliance-faq, privacy-mode-and-cloud-sync, supported-devices, MDM)
- API: api-docs.wisprflow.ai/websocket_api
- Infra: baseten.co/resources/customers/wispr-flow
- Background: en.wikipedia.org/wiki/Wispr_Flow, sacra.com/c/wispr, TechCrunch, Bloomberg, Tracxn
- Reviews/analysis: tldv.io/blog/wisprflow, spokenly.app, voisty.com, getvoibe.com, kintal.co, eesel.ai, digitalapplied.com
- OS integration: Apple dev forums (AXUIElement, secure input), github.com/missuo/koe, github.com/farzaa/clicky, github.com/10xChengTu/input0, learn.microsoft.com (UI Automation), macupdater.net
- ASR benchmarks: Coval, Northflank, FutureAGI; github.com/SYSTRAN/faster-whisper, whisper.cpp
- Competitors: superwhisper.com/vs/wispr-flow, VoiceInk (GitHub), Talon, Aqua, Willow
