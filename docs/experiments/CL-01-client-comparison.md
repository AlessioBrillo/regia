# CL-01 — Client comparison

Status: **plan, not executed.** Device matrix decided in [#14](https://github.com/AlessioBrillo/regia/issues/14).
Report §4.1. Feeds ADR-012. Owner: @AlessioBrillo ([#17](https://github.com/AlessioBrillo/regia/issues/17)).

```text
ID / version / owner:            CL-01 / v1 (plan) / @AlessioBrillo
Technical question:              For one realistic flow, how do native clients and a cross-platform client compare
                                 on integration effort, stability, accessibility, perceived latency, storage and
                                 debuggability, and how much shared Rust core pays for itself?
Linked vision requirement:       AR-02, AR-14; R-08
Initial hypothesis/alternatives: A  Native: SwiftUI + Jetpack Compose
                                 B  Cross-platform: Flutter or React Native
                                 C  Rust core + native shells (share only what is measurably worth sharing)
                                 Provisional report leaning (native clients, Rust only where measurable) is a
                                 hypothesis to test, not a result.
Relevant threat model:           T-01, T-04 (local store, notifications, caches)
Reproducible setup:              Devices and OS versions from #14; toolchains pinned; build scripts in the repo
Method and test inputs:          The single flow below, including error paths (no demo-only app)
Metrics collected and units:     See "Metrics"
Raw results and repetitions:     Not executed
Failed cases / limits:           Not executed
Effect on requirements:          Not executed
Decision: adopt/trial/defer/reject:  Not executed
Linked ADR:                      ADR-012
Conditions that would reopen:    OS API changes affecting share/storage/on-device AI; a device class added to the
                                 supported matrix; binding cost changes materially
```

## Device matrix (owner decision, 2026-11-08, #14)

The first platform is chosen **from the devices actually available**, not in the abstract (ADR-012). OS versions are
recorded at the first run; no claim of "on-device" support is made in general (report §8.3).

| Device | Role | OS version |
|---|---|---|
| Windows 11 desktop | Desktop client, dev machine | recorded at run |
| Recent Android phone | Mobile client | recorded at run |
| Recent iPhone + Mac with Xcode | Mobile client; the Mac is required to build iOS | recorded at run |
| Arch Linux box | Linux client and test backend host (also runs CI) | recorded at run |

## Flow under test

Receive text or a PDF from the system share sheet → persist locally → show a structured proposal → confirm or cancel →
sync → handle a network error. Error cases are part of the flow: network lost mid-sync, app killed mid-write, permission
denied, malformed PDF, duplicate share.

## Metrics

| Metric | Unit |
|---|---|
| Implementation time and number of platform-specific shims | hours; count |
| Crashes and hangs over a fixed scripted run | count per 100 runs |
| Accessibility audit: screen reader, dynamic type, contrast | issues found by severity |
| Perceived latency: share → proposal visible, and local actions | ms, p50/p95 (device baseline) |
| Local storage footprint and protected-storage support | bytes; supported / not |
| Binary size; binding cost and crash-debugging effort for the Rust core | bytes; hours |
| Debugging quality: time to diagnose an injected fault | min |
| Behaviour with an on-device AI task, notification and background dictation | works / needs shim / blocked |

## Gate and scoring

1. **Gate first:** the flow completes on each supported device with the error cases handled and no content leaking to
   logs, notifications or temp files (T-04); local actions meet p95 < 100 ms on the baseline device (§8.3).
2. Survivors scored with the frozen weights in [`TEMPLATE.md`](TEMPLATE.md).

## Out of scope

Sync engine choice (DAT-01), key management (CRY-01), full UI design.
