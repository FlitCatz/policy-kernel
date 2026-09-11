# Documentation Lattice Implementation Plan

> **For agentic workers:** Execute this plan inline in the isolated
> `docs-lat-restructure` worktree. Do not use CodeGraph.

**Goal:** Make each implementation Task discoverable from one documentation
directory, preserve historical records in an archive, and add LAT as a
documentation-only knowledge graph.

**Architecture:** `docs/status.md` is the current-state authority.
`docs/tasks/task-01/` through `task-14/` contain current Task landing pages
that link to immutable historical records. Dated plans, specifications, reviews,
and legacy superpowers material move to `docs/archive/`; their contents change
only where relative links require a new path. `lat.md/` provides a concise
navigable graph over the canonical `docs/` material.

**Tech Stack:** Markdown, Git moves, LAT CLI, existing Rust workspace checks.

**Spec:** `docs/superpowers/specs/2026-09-10-documentation-lattice-design.md`

## Global Constraints

- Keep repository execution logic, CLIs, verification tools, report converters,
  bootstrap installer, and tests in Rust.
- Preserve historical execution records as contemporaneous facts.
- Do not claim production promotion, authoritative CI, or protected-merge success
  from local or fixture evidence.
- Do not add CodeGraph, `@lat` source backlinks, test-spec enforcement, hosted
  embeddings, agent hooks, or CI gates.
- Preserve all existing Markdown destinations by updating every relative inbound
  link after each move.
- Run `lat check`, a relative-link check, `git diff --check`, and
  `cargo test --locked --workspace` before completion.

---

### Task 1: Create the current documentation entry points

**Files:**
- Create: `docs/status.md`
- Create: `docs/tasks/README.md`
- Create: `docs/tasks/task-01/README.md` through
  `docs/tasks/task-14/README.md`

**Consumes:** Existing README files, integration references, trust/promotion
references, and dated Task records.

**Produces:** One current-status authority and fourteen canonical Task landing
pages before historical files move.

- [ ] **Step 1: Define the status vocabulary in `docs/tasks/README.md`**

  Define `implemented`, `locally verified`, `CI observed`, and
  `not demonstrated in production`. State that each Task page summarizes the
  latest known state and its linked records remain historical evidence.

- [ ] **Step 2: Create the Task index**

  Add a Task 1–14 table with columns for Task, capability, current status, and
  landing-page link. Use the implementation plan and current execution records as
  the source of capability/status text.

- [ ] **Step 3: Create each Task landing page**

  Use this exact section order in every page:

  ```markdown
  # Task NN — <capability>

  ## Purpose

  ## Current status

  ## Evidence and limits

  ## Related implementation and references
  ```

  Link Tasks 1–5 to their sections in the archived implementation plan. Link
  Task 6 to its design and implementation plan; Task 7 to its implementation
  plan; Task 8 to its CLI/schema plan; Task 9 to its collector design, plan, and
  review; Task 10 to validation and live-promotion records; Task 11 to release
  gates; Task 12 to profile plan and execution record; Task 13 to bootstrap plan
  and execution record; Task 14 to acceptance plan, execution record,
  remediation, and final verification.

- [ ] **Step 4: Create `docs/status.md`**

  Add exactly these sections: `Implemented scope`, `Verified evidence`,
  `Not demonstrated in production`, and `Canonical references`. Cite the
  final codebase verification and Task 14 execution records for local evidence;
  cite trust model and promotion protocol for production prerequisites.

- [ ] **Step 5: Review the new entry points**

  Confirm every Task number occurs once in the index, every landing page has the
  required four sections, and the status page does not duplicate detailed
  historical prose.

- [ ] **Step 6: Commit**

  ```bash
  git add docs/status.md docs/tasks
  git commit -m "docs: add task-oriented documentation index"
  ```

### Task 2: Archive dated documentation and preserve references

**Files:**
- Move: `docs/plans/*` → `docs/archive/plans/`
- Move: `docs/specs/*` → `docs/archive/specs/`
- Move: `docs/reviews/*` → `docs/archive/reviews/`
- Move: completed historical files in `docs/superpowers/plans/` →
  `docs/archive/superpowers/plans/`
- Move: completed historical files in `docs/superpowers/specs/` →
  `docs/archive/superpowers/specs/`
- Modify: every Markdown file with a link to a moved document

**Consumes:** Current entry points from Task 1.

**Produces:** One archive location for dated records, with working inbound and
outbound links.

- [ ] **Step 1: Create archive category directories**

  Create the five destination directories listed above. Keep the active
  documentation-lattice design and plan in `docs/superpowers/` until this work
  is complete; archive them in the finishing change after all inbound links use
  their final paths. Keep
  `docs/integrations/`, `docs/trust-model.md`, `docs/promotion.md`,
  `docs/provenance.md`, and `docs/obligation-matrix.yaml` in their current
  locations.

- [ ] **Step 2: Move the dated documents with `git mv`**

  Preserve every basename. Move every pre-existing dated superpowers plan and
  specification, but retain the active documentation-lattice design and plan
  until the finishing change. Do not rename documents or edit historical prose
  while moving them.

- [ ] **Step 3: Rewrite relative links**

  Search all tracked Markdown for `docs/plans/`, `docs/specs/`,
  `docs/reviews/`, and `docs/superpowers/`. Update links from README files,
  live reference documents, archived documents, and Task landing pages to their
  archive paths. Preserve fragment identifiers.

- [ ] **Step 4: Add archive ownership notes**

  Add `docs/archive/README.md` explaining that archived files are historical
  records, their dates/status statements are contemporaneous, and the current
  state belongs in `docs/status.md` and `docs/tasks/`.

- [ ] **Step 5: Verify migration completeness**

  Confirm no tracked Markdown link targets a removed pre-archive path and no
  moved file has an unadjusted relative path to a sibling reference.

- [ ] **Step 6: Commit**

  ```bash
  git add docs README.md README_KO.md action
  git commit -m "docs: archive dated task records"
  ```

### Task 3: Connect the root documentation to current entry points

**Files:**
- Modify: `README.md`
- Modify: `README_KO.md`
- Modify: `AGENTS.md`

**Consumes:** `docs/status.md`, `docs/tasks/README.md`, and archive paths.

**Produces:** Bilingual root documentation that directs readers to current
information and agent instructions that name the canonical locations.

- [ ] **Step 1: Replace task-history links in both READMEs**

  Keep quick-start, workspace map, CLI, schema, bootstrap, and CI guidance.
  Replace the single Task 14 execution-record link in each Further reading list
  with links to `docs/status.md` and `docs/tasks/README.md`. Ensure both
  READMEs contain equivalent destinations.

- [ ] **Step 2: Update links to moved architecture and bootstrap design records**

  Rewrite the two moved specification links in each README to
  `docs/archive/specs/`. Leave links to live trust, promotion, integration,
  action, schemas, workflow, and toolchain documents unchanged.

- [ ] **Step 3: Add canonical-document guidance to `AGENTS.md`**

  Add one documentation-maintenance bullet requiring current status updates in
  `docs/status.md`, Task-specific summaries in `docs/tasks/task-NN/`, and
  preservation of dated records in `docs/archive/`.

- [ ] **Step 4: Compare language parity**

  Verify that every root-document destination present in English has the same
  destination in Korean, except for language-switch links.

- [ ] **Step 5: Commit**

  ```bash
  git add README.md README_KO.md AGENTS.md
  git commit -m "docs: link root guidance to task index"
  ```

### Task 4: Initialize the LAT documentation graph

**Files:**
- Create: `lat.md/overview.md`
- Create: `lat.md/status.md`
- Create: `lat.md/tasks.md`
- Create: `lat.md/trust-boundaries.md`
- Modify: `.gitignore` only if LAT creates untracked cache/index output

**Consumes:** Canonical documents created in Tasks 1–3.

**Produces:** A tracked, documentation-only LAT graph that navigates canonical
repository knowledge without duplicating it.

- [ ] **Step 1: Install and inspect LAT**

  Install the official LAT CLI outside the repository with
  `npm install -g lat.md`. Run `lat init` in the worktree only after reviewing
  its proposed repository changes. Retain `AGENTS.md` rules; reject generated
  agent hooks or instruction changes outside the intended LAT marker section.

- [ ] **Step 2: Create the graph entry point**

  In `lat.md/overview.md`, state that `docs/` remains canonical and link
  with wiki references to `[[status]]`, `[[tasks]]`, and
  `[[trust-boundaries]]`. Add ordinary Markdown links to `../README.md`,
  `../docs/status.md`, and `../docs/tasks/README.md`.

- [ ] **Step 3: Create graph nodes**

  `lat.md/status.md` links to the current status and production prerequisites.
  `lat.md/tasks.md` lists Tasks 1–14 and links to every Task landing page.
  `lat.md/trust-boundaries.md` links to trust model, promotion protocol,
  authoritative CI action, and advisory/local boundaries.

- [ ] **Step 4: Remove unintended scaffolding**

  Do not commit a Node package manifest, lockfile, source-code annotation, API
  key, embedding index, or generated agent configuration. Add only an ignore
  rule if LAT creates a local cache/index that should not be tracked.

- [ ] **Step 5: Run graph validation**

  Run `lat check`. Resolve every broken LAT reference before continuing.

- [ ] **Step 6: Commit**

  ```bash
  git add lat.md .gitignore AGENTS.md
  git commit -m "docs: add LAT knowledge graph"
  ```

### Task 5: Run final documentation and workspace verification

**Files:**
- Verify: all tracked `*.md` files and `docs/obligation-matrix.yaml`
- Verify: `README.md`, `README_KO.md`, `docs/status.md`, `docs/tasks/`,
  `docs/archive/`, and `lat.md/`

**Consumes:** All prior tasks.

**Produces:** Evidence that the migration preserves links, LAT graph consistency,
formatting, and the Rust test baseline.

- [ ] **Step 1: Check every relative Markdown destination**

  Extract Markdown links from tracked documentation, ignore external URLs,
  `mailto:`, and fragment-only links, resolve each remaining target relative to
  the containing file, and fail on a missing path.

- [ ] **Step 2: Check stale locations**

  Search for `docs/plans/`, `docs/specs/`, `docs/reviews/`, and
  `docs/superpowers/` in tracked Markdown. The only allowed matches are
  historical prose that describes old paths; Markdown link destinations must use
  `docs/archive/`.

- [ ] **Step 3: Run LAT and diff validation**

  Run `lat check` and `git diff --check`.

- [ ] **Step 4: Run the Rust workspace baseline**

  Run `cargo test --locked --workspace`. Record its test count; documentation
  changes must not affect Rust behavior.

- [ ] **Step 5: Inspect the final change**

  Review `git status --short`, `git diff --stat`, and the task index. Confirm
  every Task 1–14 has one landing page, archived records remain present, and
  CodeGraph files/configuration are absent.

- [ ] **Step 6: Commit**

  ```bash
  git add AGENTS.md README.md README_KO.md .gitignore docs lat.md
  git commit -m "docs: organize task documentation and LAT graph"
  ```
