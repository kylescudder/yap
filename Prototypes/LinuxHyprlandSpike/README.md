# Yap Linux / Hyprland feasibility spike

> **PROTOTYPE ONLY.** This directory is disposable evidence-gathering code, not the Linux
> implementation.

This spike answers four questions before Yap commits to a durable Rust/GTK architecture:

1. Can Hyprland deliver distinct press and release edges for bare Right Super?
2. Can PipeWire capture a useful 16 kHz mono utterance?
3. Can a warm `whisper.cpp` CUDA context meet the three-second p95 acceptance limit on the target
   RTX 3080?
4. Can native virtual-keyboard insertion, with a clipboard fallback, preserve Unicode and multiline
   text across native Wayland and XWayland applications?

## Run

From the repository root:

```sh
./Prototypes/LinuxHyprlandSpike/run.sh
```

The guided run checks dependencies, generates a Hyprland snippet, records three Right-Super
press/release pairs, explicitly offers to download the 547 MiB quantized Whisper model, captures an
utterance, runs five warm inference measurements, and walks through the application insertion
matrix.

The script never invokes `sudo`, edits `hyprland.conf`, installs packages, persists raw audio outside
the runtime scratch directory, or makes background network requests. The only network operation is
the confirmed model download.

If dependencies are missing, install them yourself and rerun:

```sh
sudo pacman -S --needed pipewire-audio whisper-cpp ggml-cpu ggml-cuda wtype wl-clipboard curl python
```

The model is stored separately under `${XDG_DATA_HOME:-$HOME/.local/share}/yap-prototype/models` and
verified against the Hugging Face LFS SHA-256. Runtime audio, logs, and measurements live under
`${XDG_RUNTIME_DIR:-/tmp}/yap-linux-spike-$UID`.

## Individual probes

```sh
./Prototypes/LinuxHyprlandSpike/run.sh doctor
./Prototypes/LinuxHyprlandSpike/run.sh setup
./Prototypes/LinuxHyprlandSpike/run.sh hotkey
./Prototypes/LinuxHyprlandSpike/run.sh capture
./Prototypes/LinuxHyprlandSpike/run.sh benchmark
./Prototypes/LinuxHyprlandSpike/run.sh insertion
./Prototypes/LinuxHyprlandSpike/run.sh report
```

The generator detects Hyprland's configuration style. Hyprland 0.55+ receives
`${XDG_CONFIG_HOME:-$HOME/.config}/yap/prototype-hyprland.lua` and an exact `require(...)` line;
older installations receive `prototype-hyprland.conf` and an exact `source = ...` line. Yap never
rewrites compositor-owned configuration.

The insertion text begins each line with `#` to make an accidental terminal target harmless. Do not
run the insertion probe against a password field.

## Verdict

The spike succeeds only when:

- three ordered Right-Super press/release pairs are observed;
- PipeWire produces a non-empty WAV;
- `whisper-server` confirms CUDA/NVIDIA initialization;
- five warm release-to-insertion trials have p95 latency at or below three seconds; and
- all mandatory reference applications pass direct or clipboard insertion.

Print the evidence with:

```sh
./Prototypes/LinuxHyprlandSpike/run.sh report
```

Once the verdict is captured, run `cleanup` and remove the printed source line from
`hyprland.conf`. The downloaded model is deliberately retained unless removed manually.
