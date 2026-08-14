# diceroll Android

A [Capacitor](https://capacitorjs.com/) wrapper that bundles `diceroll_wasm/www/`
into the app. Fully offline: everything (HTML/JS/WASM) is served by the Capacitor
bridge from the APK assets, and the app makes no network request at runtime.

This directory is a generated Capacitor Android project (`npx cap add android`).
The Capacitor configuration lives in [`capacitor.config.json`](../capacitor.config.json)
at the repository root, where `webDir` points at `diceroll_wasm/www` — the exact
same files that ship to the web. `npx cap sync android` copies them into
`app/src/main/assets/public/`, which is generated and git-ignored.

## Prerequisites

- Node.js 22+ (for the Capacitor CLI)
- JDK 21
- Android SDK (command-line tools or Android Studio) with platform 36 and build-tools 36.0.0
- `wasm-pack` (to build the Rust -> WASM artifacts)

Set `ANDROID_HOME` or create `android/local.properties` with `sdk.dir=...`.

## Build

```sh
# 1. Build the WASM bundle that the app will ship
(cd diceroll_wasm && ./build.sh)

# 2. Install the Capacitor CLI and copy the web assets into the Android project
npm ci
npx cap sync android

# 3. Build the APK
(cd android && ./gradlew assembleRelease)
```

The signed-with-debug-key APK lands in
`android/app/build/outputs/apk/release/`. For Play Store distribution, replace
the debug signing config with a real release keystore in `app/build.gradle`.

To open the project in Android Studio instead, run `npx cap open android`.

## Install on a device

```sh
adb install -r android/app/build/outputs/apk/release/app-release.apk
```

## What's inside

- `app/src/main/java/run/diceroll/MainActivity.java` — an empty
  `BridgeActivity`; Capacitor does the WebView setup, serves the assets (with
  the correct `application/wasm` MIME type), opens external links in the system
  browser, and maps the system bar insets onto the web layout.
- `app/src/main/res/values/styles.xml` — dark-only theme matching the web UI,
  with a plain coloured launch window instead of a splash image.
- `app/src/main/res/values/colors.xml`, `drawable/ic_launcher_*`,
  `mipmap-*/ic_launcher*` — the app palette and launcher icon.
- `app/build.gradle` — app id, version, and the debug signing config used for
  release builds.

Regenerating the project from scratch (`rm -rf android && npx cap add android`)
resets all of the above to the Capacitor defaults.
