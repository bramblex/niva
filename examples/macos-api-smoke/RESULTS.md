# macOS API and NodeCompat test record — 2026-09-24

Run on an awake, unlocked Apple Silicon Mac against this checkout's
`target/debug/niva` using:

```sh
python3 -B examples/macos-api-smoke/run_all.py target/debug/niva
```

The sequential run passed all ten disposable suites. Its exact logs were
written to `/tmp/niva-macos-final-5/` on the test Mac.

| Surface | Real macOS result |
|---|---:|
| Public `Niva.api` methods | 167/167 invoked; 161 behavior-verified, 6 invocation-only |
| Top-level `NivaObj` bridge methods | 9/9 behavior-verified |
| NodeCompat documented methods and aliases | 185/185 checks in 16 cases across 15 modules |

The six invocation-only methods are `window.requestRedraw`,
`window.requestUserAttention`, `window.setCursorGrab`,
`window.setIgnoreCursorEvents`, `window.setImePosition`, and
`window.setVisibleOnAllWorkspaces`. Each call resolved against the native Mac
app and its test state was cleaned up. The stated visual, pointer, IME, or
Space effect was not independently observed; they are not included in the
161 behavior-verified count.

The dialog suite opened all six native panel types and checked their dismiss
or cancellation paths. Selecting multiple files/folders is a separate branch
that this automatic run did not exercise. The NodeCompat HTTPS cases check
wrong-scheme rejection without an outbound TLS request. The run uses isolated
debug resources, not a downloaded or signed release package.

## Thread-affinity follow-up

A macOS crash report from 2026-09-24 02:35 shows AppKit rejecting
`window.setVisibleOnAllWorkspaces` on the `smol-1` worker thread with
“Must only be used from the main thread”. The main-thread dispatch for that
specific method entered the test branch later at 04:25. The subsequent
thread-affinity audit also moved other AppKit-backed `window.*` reads and
mutators to the main event loop.

On the isolated follow-up branch, `run_thread_affinity.py` invoked 22 window
calls from a real macOS WebView, including enabling and restoring
`setVisibleOnAllWorkspaces`; all resolved and the process stayed alive until
the runner terminated it. This checks the crash mechanism, not the visual
Space effect. A fresh full supervised run timed out after its first two
cases while exercising the always-on-bottom case. The extended automatic
suite stopped at its strict monitor precondition because Tao returned no
connected displays in that session. Those runs do not count as full-suite
passes; repeat them with an awake, unlocked desktop before release acceptance.

Local gates on the same checkout passed: `cargo fmt --all -- --check`,
`cargo check --workspace`, `cargo clippy --workspace --all-targets`,
`cargo test --workspace` (90 Niva and 10 win_packager tests), NodeCompat
unit tests (68), Devtools API/bridge scripts (21), and
`npm run build --workspace=packages/devtools`. The macOS arm64 release binary
at `target/release/niva` was **2,288,336 bytes**, below the 3 MB target.
