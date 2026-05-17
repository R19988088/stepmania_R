# Code Structure

## Current Source Groups

```text
src/main.rs                  App startup, song selection, preview audio, path discovery.
src/game.rs                  Gameplay loop, rendering, input, scoring, audio playback.
src/chart.rs                 Shared chart model and beat/time conversion.
src/sm_parser.rs             StepMania `.sm` parser.
src/dwi_parser.rs            `.dwi` parser.
src/song_select_services.rs  Song-select support services such as cover loading.
```

## Rules For Keeping The Repo Clean

- Put platform-independent gameplay and parsing in `src/`.
- Put Android or CI build notes in `docs/` and `.github/`.
- Put helper scripts in `scripts/`.
- Keep bulky test song libraries outside git unless a tiny sample chart is added later for tests.
- Do not commit generated files: `target/`, APK/AAB output, local selection files, or desktop `.exe` files.

## Android Porting Order

1. Keep desktop Rust build working.
2. Keep Android APK builds working.
3. Add Android storage/input fixes behind `#[cfg(target_os = "android")]`.
4. Only after the MVP works, split the large `src/game.rs` into renderer, input, scoring, and audio modules.
