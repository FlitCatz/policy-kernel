# Documentation Lattice Design

## Goal

Make task-oriented documentation discoverable from one location while preserving
historical design, plan, and review records as immutable evidence. Add LAT as a
documentation-only knowledge graph; CodeGraph is explicitly out of scope.

## Current problem

The repository has current usage guidance, integration references, historical
specifications, implementation plans, and execution reviews under adjacent
directories. Several records intentionally describe the state at the time they
were written, while current README content needs the latest state. Readers cannot
reliably infer which document is authoritative for current status or which Task
owns a record.

## Information architecture

```text
docs/
  status.md
  tasks/
    README.md
    task-01/README.md
    …
    task-14/README.md
  integrations/
  trust-model.md
  promotion.md
  provenance.md
  archive/
    plans/
    specs/
    reviews/
    superpowers/
lat.md/
  overview.md
  status.md
  tasks.md
  trust-boundaries.md
```

`docs/status.md` is the sole current-status document. It states implemented
scope, verified evidence, conditions not demonstrated in production, and
canonical references. It must link to source records rather than copying their
detail.

`docs/tasks/README.md` indexes Tasks 1–14 and defines status terms.
Every `docs/tasks/task-NN/README.md` is that Task's canonical landing page. It
contains: purpose, implementation scope, current status, limits, relevant source
locations, and links to archived records. A Task page summarizes the current
state; it never changes historical facts in an execution record.

`docs/archive/` stores existing dated specifications, plans, reviews, and
legacy superpowers material. The content is retained verbatim except for relative
link rewrites caused by relocation. Multi-Task records remain single documents in
the appropriate archive category and are linked from each related Task page.

`docs/integrations/`, `docs/trust-model.md`, `docs/promotion.md`, and
`docs/provenance.md` remain live reference documents because they describe
interfaces, boundaries, and operations rather than a single Task's history.

## LAT scope

LAT is used as a local, tracked documentation graph rooted at `lat.md/`.
The graph contains concise navigation and semantic context only; canonical
technical content remains in `docs/`. LAT files use wiki links among graph
sections and ordinary Markdown links to canonical documentation and source.

Initial LAT coverage is limited to the repository overview, current status,
Task map, and trust boundaries. Do not add `@lat` backlinks to Rust source,
test-spec enforcement, hosted embeddings, agent hooks, or CI gates in this
change. Run `lat check` as a documentation consistency check after initialization.

## Migration rules

1. Add the status page and all Task landing pages before moving existing files.
2. Relocate dated historical documents to `docs/archive/` by category.
3. Update every inbound Markdown link to each relocated file.
4. Update README files to point at `docs/status.md` and `docs/tasks/README.md`
   instead of individual historical records when the reader needs current state.
5. Retain dates and historical-status wording inside archived records.
6. Do not change source code, CI workflow semantics, policy contracts, or
   promotion requirements.

## Verification

- Run `lat check` after generating the LAT graph.
- Check every relative Markdown link in tracked documentation.
- Run `git diff --check`.
- Run `cargo test --locked --workspace` after documentation changes to preserve
  the repository baseline; no Rust behavior is expected to change.

