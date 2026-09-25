# Android voice entry

The microphone control under the quick-entry composer uses Android's on-device speech service. It is separate from the LLM and does not bundle Whisper. Windows voice transcription is unavailable. Successful speech recognition only replaces draft text; reading, reviewing and confirming are still required to save entries.

The app checks availability before requesting microphone permission. If the system supports a missing Thai language model, a download action delegates installation to Android without opening the microphone. Wait for the model to install, then retry. Cancelled or failed recognition preserves existing text.

## Platform support

- Android 12 / API 31 or later is required for this feature; older Android versions can still use manual entry.
- API 33+ checks installed on-device language support before listening.
- API 31–32 attempts the on-device recognizer directly and reports unsupported-language errors.
- Thai uses `th-TH`. Availability depends on the device's speech provider and downloaded language pack; an emulator is not evidence of Thai microphone support.

The host uses `createOnDeviceSpeechRecognizer`, not a general recognizer with an advisory offline flag. There is no cloud fallback. Permission is requested only when the user starts voice input. The app does not save audio files or log transcripts.

The recognizer has bounded setup, listening and final-result times (45, 30 and 10 seconds). Results over 1,000 characters are rejected rather than truncated. Session IDs reject late callbacks; navigation, cancellation, activity stop and destruction clean up capture. Review digits, decimal points and names before parsing; the deterministic command parser does not infer spoken-number corrections.

## Verification

Test permission denial/revocation, unsupported languages, retry, silence, cancellation, repeated taps, navigation, backgrounding and lock-screen interruption using synthetic input. On supported physical hardware, test Thai recognition in airplane mode and verify that successful recognition alone creates no journal entry. Emulator UI testing does not establish physical microphone or offline language accuracy.

UI state is in `crates/ui/src/voice.rs`; native adapters are in `apps/android/src/voice.rs` and `apps/android/native/MainActivity.kt`.
