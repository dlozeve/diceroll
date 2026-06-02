# diceroll Android

A minimal Android wrapper that bundles `diceroll_wasm/www/` into a WebView. Fully
offline: no `INTERNET` permission, no network at runtime - the HTML/JS/WASM are
loaded from the APK assets via `WebViewAssetLoader`.

The Android build does **not** copy or duplicate the web assets. It points the
Gradle `assets` source set at `../../diceroll_wasm/www/` directly, so the same
files that ship to the web are bundled into the APK.

## Prerequisites

- JDK 17
- Android SDK (command-line tools or Android Studio) with platform 36 and build-tools 36.0.0
- Gradle 9.4.1+ (only if generating the wrapper; not needed once `gradlew` exists)
- `wasm-pack` (to build the Rust -> WASM artifacts)

Set `ANDROID_HOME` or create `android/local.properties` with `sdk.dir=...`.

## Build

```sh
# 1. Build the WASM bundle that the app will ship
(cd diceroll_wasm && ./build.sh)

# 2. First time only - generate the Gradle wrapper
(cd android && gradle wrapper)

# 3. Build the APK
(cd android && ./gradlew assembleRelease)
```

The signed-with-debug-key APK lands in
`android/app/build/outputs/apk/release/`. For Play Store distribution, replace
the debug signing config with a real release keystore in
`app/build.gradle.kts`.

## Install on a device

```sh
adb install -r android/app/build/outputs/apk/release/app-release.apk
```

## What's inside

- `app/src/main/java/run/diceroll/MainActivity.kt` — single `ComponentActivity`
  with a `WebView` and a custom `PathHandler` that returns `application/wasm`
  for `.wasm` requests (the default handler serves them as
  `application/octet-stream`, which breaks `WebAssembly.instantiateStreaming`).
- `app/src/main/AndroidManifest.xml` — no permissions, dark theme, single
  launcher Activity.
- `app/build.gradle.kts` — `assets.srcDirs("../../diceroll_wasm/www")` is the
  whole code-sharing mechanism.
