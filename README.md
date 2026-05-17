# StepMania R

Rust mini StepMania project prepared for cross-platform work, with Android as the first port target.

## Project Layout

```text
src/          Game code and chart parsers.
assets/       Small runtime assets used by the Rust game.
NoteSkins/    StepMania note-skin graphics needed by gameplay.
scripts/      Build helper scripts.
docs/         Porting notes and project organization docs.
.github/      GitHub Actions for Android builds.
```

Keep generated files out of the repository. Songs, local save files, APKs, and `target/` builds should stay local.

## Local Desktop Run

```powershell
cargo run -- "Songs/Your Song/Your Chart.sm"
```

The current code is GPU-first through Macroquad. The minimum target GPU class is GTX 1080 on desktop-class hardware; Android work should keep the same GPU-first rendering direction.

## Android Direction

Android is handled in two steps:

1. Build the APK locally when the Android SDK/NDK is installed.
2. Let GitHub Actions package APK artifacts after push.

See `docs/ANDROID_BUILD.md`.
