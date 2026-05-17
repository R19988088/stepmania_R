param(
    [switch]$Release = $true
)

$ErrorActionPreference = "Stop"

if (-not $env:ANDROID_NDK_HOME -and $env:NDK_HOME) {
    $env:ANDROID_NDK_HOME = $env:NDK_HOME
}

if (-not $env:ANDROID_NDK_HOME) {
    Write-Error "Please set ANDROID_NDK_HOME or NDK_HOME."
    exit 1
}

function Use-NoSpaceJunction {
    param(
        [string]$Name,
        [string]$Source
    )

    if (-not $Source -or $Source -notmatch "\s") {
        return $Source
    }

    $root = Join-Path $env:SystemDrive "codex_build\junctions"
    New-Item -ItemType Directory -Force -Path $root | Out-Null
    $target = Join-Path $root $Name
    if (-not (Test-Path -LiteralPath $target)) {
        New-Item -ItemType Junction -Path $target -Target $Source | Out-Null
    }
    return $target
}

$env:ANDROID_NDK_HOME = Use-NoSpaceJunction "android-ndk" $env:ANDROID_NDK_HOME
$env:NDK_HOME = $env:ANDROID_NDK_HOME

if (-not $env:ANDROID_HOME -and $env:ANDROID_SDK_ROOT) {
    $env:ANDROID_HOME = $env:ANDROID_SDK_ROOT
}

if (-not $env:ANDROID_HOME) {
    $sdkGuess = Split-Path -Parent $env:ANDROID_NDK_HOME
    if (Test-Path -LiteralPath (Join-Path $sdkGuess "platforms")) {
        $env:ANDROID_HOME = $sdkGuess
        $env:ANDROID_SDK_ROOT = $sdkGuess
    }
}

if ($env:ANDROID_HOME) {
    $env:ANDROID_HOME = Use-NoSpaceJunction "android-sdk" $env:ANDROID_HOME
    $env:ANDROID_SDK_ROOT = $env:ANDROID_HOME
}

$quadSdk = Join-Path $env:SystemDrive "codex_build\android-sdk-quad"
$quadBuildTools = Join-Path $quadSdk "build-tools"
$quadPlatforms = Join-Path $quadSdk "platforms"
New-Item -ItemType Directory -Force -Path $quadBuildTools, $quadPlatforms | Out-Null
$buildTools30 = Join-Path $env:ANDROID_HOME "build-tools\30.0.3"
$platform30 = Join-Path $env:ANDROID_HOME "platforms\android-30"
if ((Test-Path -LiteralPath $buildTools30) -and -not (Test-Path -LiteralPath (Join-Path $quadBuildTools "30.0.3"))) {
    New-Item -ItemType Junction -Path (Join-Path $quadBuildTools "30.0.3") -Target $buildTools30 | Out-Null
}
if ((Test-Path -LiteralPath $platform30) -and -not (Test-Path -LiteralPath (Join-Path $quadPlatforms "android-30"))) {
    New-Item -ItemType Junction -Path (Join-Path $quadPlatforms "android-30") -Target $platform30 | Out-Null
}
$env:ANDROID_HOME = $quadSdk
$env:ANDROID_SDK_ROOT = $quadSdk

$quadBuildTools30 = Join-Path $env:ANDROID_HOME "build-tools\30.0.3"
$dxExe = Join-Path $quadBuildTools30 "dx.exe"
if (-not (Test-Path -LiteralPath $dxExe)) {
    $wrapperRs = Join-Path $env:TEMP "dx-wrapper.rs"
    if (Test-Path -LiteralPath $dxExe) {
        Remove-Item -LiteralPath $dxExe -Force
    }
    @'
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};

fn main() {
    let exe = env::current_exe().expect("current exe");
    let dir = exe.parent().expect("dx parent");
    let script = dir.join("d8.bat");
    let mut output = PathBuf::from("classes.dex");
    let mut inputs = Vec::new();
    let mut min_api = None::<String>;
    let mut args = env::args_os().skip(1).peekable();
    while let Some(arg) = args.next() {
        let s = arg.to_string_lossy().to_string();
        if s == "--dex" {
            continue;
        } else if let Some(rest) = s.strip_prefix("--output=") {
            output = PathBuf::from(rest);
        } else if s == "--output" {
            if let Some(v) = args.next() {
                output = PathBuf::from(v);
            }
        } else if s == "--min-sdk-version" {
            if let Some(v) = args.next() {
                min_api = Some(v.to_string_lossy().to_string());
            }
        } else {
            let path = PathBuf::from(&arg);
            if path.is_dir() {
                collect_class_files(&path, &mut inputs);
            } else {
                inputs.push(arg);
            }
        }
    }

    let parent = output.parent().unwrap_or_else(|| std::path::Path::new("."));
    let out_dir = if parent.as_os_str().is_empty() {
        std::path::Path::new(".")
    } else {
        parent
    };
    let status = Command::new("cmd.exe")
        .arg("/c")
        .arg("call")
        .arg(script)
        .arg("--output")
        .arg(out_dir)
        .args(min_api.iter().flat_map(|v| ["--min-api".to_string(), v.clone()]))
        .args(inputs)
        .status()
        .expect("run d8.bat");
    exit(status.code().unwrap_or(1));
}

fn collect_class_files(dir: &Path, out: &mut Vec<std::ffi::OsString>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_class_files(&path, out);
            } else if path.extension().and_then(|s| s.to_str()) == Some("class") {
                out.push(path.into_os_string());
            }
        }
    }
}
'@ | Set-Content -LiteralPath $wrapperRs -Encoding ASCII
    rustc $wrapperRs -o $dxExe
}

$ndkBin = Join-Path $env:ANDROID_NDK_HOME "toolchains\llvm\prebuilt\windows-x86_64\bin"
$toolAliases = @{
    "aarch64-linux-android-ar.exe" = "llvm-ar.exe"
    "aarch64-linux-android-ld.exe" = "ld.lld.exe"
    "aarch64-linux-android-readelf.exe" = "llvm-readelf.exe"
    "aarch64-linux-android-objcopy.exe" = "llvm-objcopy.exe"
    "aarch64-linux-android-strip.exe" = "llvm-strip.exe"
    "aarch64-linux-android-ranlib.exe" = "llvm-ranlib.exe"
    "aarch64-linux-android-nm.exe" = "llvm-nm.exe"
    "aarch64-linux-android-objdump.exe" = "llvm-objdump.exe"
}
foreach ($alias in $toolAliases.Keys) {
    $aliasPath = Join-Path $ndkBin $alias
    $sourcePath = Join-Path $ndkBin $toolAliases[$alias]
    if ((Test-Path -LiteralPath $sourcePath) -and -not (Test-Path -LiteralPath $aliasPath)) {
        Copy-Item -LiteralPath $sourcePath -Destination $aliasPath
    }
}

if ($env:CARGO_HOME) {
    $env:CARGO_HOME = Use-NoSpaceJunction "cargo-home" $env:CARGO_HOME
}

$activeToolchain = & rustup show active-toolchain
$toolchainName = ($activeToolchain -split "\s+")[0]
if ($env:RUSTUP_HOME -and $toolchainName) {
    $toolchainRoot = Join-Path $env:RUSTUP_HOME "toolchains\$toolchainName"
    if ($toolchainRoot -match "\s") {
        $safeToolchainRoot = Join-Path $env:SystemDrive "codex_build\rust-toolchains\$toolchainName"
        if (-not (Test-Path -LiteralPath (Join-Path $safeToolchainRoot "bin\rustc.exe"))) {
            New-Item -ItemType Directory -Force -Path (Split-Path $safeToolchainRoot) | Out-Null
            robocopy $toolchainRoot $safeToolchainRoot /E /NFL /NDL /NJH /NJS /NP
            if ($LASTEXITCODE -gt 7) {
                exit $LASTEXITCODE
            }
        }
        $toolchainRoot = $safeToolchainRoot
    }
    $rustcPath = Join-Path $toolchainRoot "bin\rustc.exe"
    if (Test-Path -LiteralPath $rustcPath) {
        $env:RUSTC = $rustcPath
    }
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error "cargo not found in PATH."
    exit 1
}

if (-not (Get-Command cargo-quad-apk -ErrorAction SilentlyContinue)) {
    Write-Host "Installing cargo-quad-apk..."
    cargo install cargo-quad-apk --locked
}

if (-not $env:CARGO_TARGET_DIR -and ((Get-Location).Path -match "\s")) {
    $safeTarget = Join-Path $env:SystemDrive "codex_build\stepmania_R_target"
    New-Item -ItemType Directory -Force -Path $safeTarget | Out-Null
    $env:CARGO_TARGET_DIR = $safeTarget
    Write-Host "Workspace path contains spaces; using CARGO_TARGET_DIR=$safeTarget"
}

$abiFlag = "-C link-arg=-lc++abi"
$clangUnwindDir = Join-Path $env:ANDROID_NDK_HOME "toolchains\llvm\prebuilt\windows-x86_64\lib64\clang\14.0.7\lib\linux\aarch64"
$unwindFlag = "-Lnative=$clangUnwindDir"
if ($env:RUSTFLAGS) {
    if ($env:RUSTFLAGS -notlike "*$abiFlag*") {
        $env:RUSTFLAGS = "$env:RUSTFLAGS $abiFlag"
    }
    if ((Test-Path -LiteralPath $clangUnwindDir) -and $env:RUSTFLAGS -notlike "*$clangUnwindDir*") {
        $env:RUSTFLAGS = "$env:RUSTFLAGS $unwindFlag"
    }
} else {
    $env:RUSTFLAGS = $abiFlag
    if (Test-Path -LiteralPath $clangUnwindDir) {
        $env:RUSTFLAGS = "$env:RUSTFLAGS $unwindFlag"
    }
}

$mode = if ($Release) { "--release" } else { "" }

Write-Host "Building Android APK..."
cargo quad-apk build $mode --target aarch64-linux-android

Write-Host "Done."
Write-Host "APK output: $env:CARGO_TARGET_DIR\android-artifacts"
