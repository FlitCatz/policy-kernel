# Implementation language policy

Implement this repository's execution logic, CLIs, verification tools, report
converters, bootstrap installer, and tests in Rust. Do not add Python scripts.
Limit shell and YAML to the minimum needed to invoke Rust tools and configure CI.

# Documentation maintenance

- Update `README.md` and `README_KO.md` together when CLI behavior, profiles,
  bootstrap, verification procedures, or current status changes.
- Do not claim that production promotion, an authoritative CI decision, or a
  protected-branch merge is complete based only on local tests or fixture
  verification. Ground current-status claims in the relevant execution record and
  trust/promotion documentation.
- Preserve historical execution records as statements of their contemporaneous
  facts. Describe later status separately in a README or new status document;
  do not rewrite historical records to match the current state.
- Treat `docs/status.md` as the current-status authority and
  `docs/tasks/task-NN/` as the Task-specific landing pages. Preserve dated plans,
  specifications, and reviews in `docs/archive/`.
- Check documented commands, arguments, and exit codes against the implementation.
  After documentation changes, verify Markdown relative links and `git diff --check`.

# Knowledge graph and code index maintenance

`lat.md/` is a [lat.md](https://github.com/vercel-labs/lat.md) navigation and code
anchoring layer; `.codegraph/` is a local [CodeGraph](https://github.com/colbymchenry/codegraph)
index. Neither is policy evidence nor a current-status authority.

- In the same change that alters behavior, architecture, contracts, tests, Task
  status, or current status, update the affected `lat.md/` sections. Keep
  `lat.md/status.md` a summary of `docs/status.md` that claims nothing more, and
  list new Task landing pages in `lat.md/tasks.md`.
- `lat.md/kernel.md` anchors authorization rules to Rust symbols with
  `[[path/to/file.rs#symbol]]` (use `[[file.rs#Type#method]]` for impl methods).
  When adding, renaming, moving, or removing an anchored symbol or rule, update
  its section. Prefer these forward references to `// @lat:` comments in source
  files covered by `verification/mutation-contract.json`, whose hashes and spans
  are pinned.
- Every `lat.md/` section needs a leading paragraph of at most 250 characters.
  Add each new `lat.md/` file to `lat.md/lat.md`.
- Run `lat check` after every change and before committing; it must report
  `All checks passed`. The "No init version recorded" warning is expected until a
  maintainer runs `lat init` interactively.
- After changing source files, run `codegraph sync`. The index auto-syncs only
  while its MCP daemon runs, and `codegraph status` can report "up to date" for a
  stale index, so confirm a new or renamed symbol with `codegraph query <symbol>`
  before relying on the index. Never commit `.codegraph/`.
