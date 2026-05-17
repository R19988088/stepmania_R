# Android Build

## Local APK Build

Local builds produce an Android APK:

```powershell
.\scripts\build-android-apk.ps1
```

Output:

```text
target/android-artifacts/release/apk/
```

Required environment:

```text
ANDROID_NDK_HOME
```

`NDK_HOME` is also accepted and copied to `ANDROID_NDK_HOME` by the script.

The build script adds `-C link-arg=-lc++abi` for Android so Oboe/Rodio links against the NDK C++ ABI runtime correctly.

On Windows, if the repository, SDK, NDK, or Rust toolchain path contains spaces, the script uses no-space build paths or junctions because Android linker tools can split unquoted paths.

## GitHub Actions APK Build

APK packaging runs in GitHub Actions through `.github/workflows/android.yml`.

Expected artifacts:

```text
android-apk
```

## Target Direction

- GPU-first rendering.
- Android first target: `arm64-v8a`.
- Android compile SDK: 30. Target SDK: 22, so sideload builds get legacy external-storage permission behavior for visible debug logs.
- Desktop-class minimum GPU direction: GTX 1080.
