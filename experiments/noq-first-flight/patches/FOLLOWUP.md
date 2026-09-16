# Follow-up patches for noq #806

These are experimental patches accompanying
[the issue discussion](https://github.com/n0-computer/noq/issues/806#issuecomment-5703285920).
They do not propose changing global pacing defaults.

- [noq repeated-Initial flight hint](noq-main-flight-hint.patch): apply to noq
  `1a26a8b064d21e316fe6769f068617975bd8a27b`. Repeats the first Initial's CRYPTO
  bytes under normal pacing and congestion control, selected by a default-false
  crypto-backend hint. Includes four focused tests. The rate-cap test checks a
  conservative pacing-delay bound, not an exact rate guarantee.
- [Iroh integration and final benchmark harness](iroh-identity-final-cap-study.patch):
  apply to iroh `e230454595b0ca64b61a17d356f2918bf0efe731`. Includes the optional
  policy-derived hint, rate-limit builder forwarding method, and harness settings
  for RTT, segmentation offload, independent bulk-flow rate caps, and PQ-only
  fixed arrivals. Repetition is gated by `unstable-identity-flight-hint`, which is
  disabled by default. The selector assumes raw public keys, disabled resumption,
  and an exclusively ML-DSA-65 remote policy; mixed and unknown policies stay off.
- [Minimal iroh rate-limit API](iroh-outgoing-rate-limit-api.patch): an independent
  alternative on the same iroh base, exposing noq's existing per-connection cap.
  It leaves the default at `None` and has no hint or benchmark changes.

Apply each full patch to its clean pinned base, not on top of earlier experiment
patches. The minimal iroh patch is already included in the full iroh patch;
do not apply both. The harness manifests on the iroh base expect patched noq
sources under `experiments/noq-first-flight/noq`.

The earlier published experiment artifacts use older mechanisms and topology.
Do not attribute those older measurements to these patches. This small update
publishes the reviewed patch files; it is not the complete later measurement
archive.

Validation before publication: patch application against clean pinned bases and
reverse checks against experiment worktrees. The noq implementation's saved run
has 430 passing library tests, including the four focused tests. The iroh harness
was built and exercised in the follow-up measurements. The rate-cap test comment
was corrected after fact-checking without changing its assertion or runtime code.
This does not claim a fresh full-workspace CI run.

Prepared with AI assistance at the user's direction.
