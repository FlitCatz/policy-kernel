# Project-owned tool versions local follow-up — 2026-09-30

This record covers the uncommitted Task 17 candidate based on
`dc8a0ffc43736d2cc17ec3f4600ca73df0c3b351`. The harness reported a dirty
source tree. It is local advisory evidence, not a hosted required check or a
production promotion.

The fixed native matrix completed in
`/private/tmp/policy-project-versions-20260930-13` with Node 26.10.0,
npm 10.9.3 and 12.1.0, Vitest/Playwright and Jest. The same run exercised unit
failure, browser retry, an actual lodash advisory, controlled audit service
failure, malformed/missing reports, stale head and incomplete audit arguments.
Each phase retained its original report bytes in a separate `*-reports`
directory, checked against the normalized report digest. The harness executable
digest was `ffe309274cf3ce7f8c3a3b64e753792ec84e30005dab09a5378f154a1e256b26`.

The candidate input mode completed in
`/private/tmp/policy-project-candidate-20260930-02` using a copy of the
frontend example and its generated schema 4 profile. The profile digest was
`701af73d993843fd0e8e9bf67fc6a458236d0855db5895a1ff8f02441f4c2842`,
the candidate source digest was
`c720e2e5fa12e487a80108cb1bce66897fb119d015f9b5e11f01413428d756b8`,
and Node/npm observations were 26.10.0/12.1.0. It ran the selected sensors and
empty-report rejection with the same harness executable digest. The candidate
mode accepts a supplied frontend fixture, so a consumer version-change PR can
be tested without editing the Rust runner.

The fixed matrix and candidate mode both passed their ignored Rust test.
This record does not establish compatibility for arbitrary future producer
versions. No hosted Task 17 result, authenticated exact-head required check,
protected merge, or production promotion was observed here.
