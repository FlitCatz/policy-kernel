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
  `All checks passed`. The generic lat checklist between the `lat:begin` and
  `lat:end` markers below is maintained by `lat init`; do not edit it by hand.
  `lat init` also installs local agent hooks under the git-ignored `.claude/` and
  `.codex/` directories and keeps its search index in the git-ignored
  `lat.md/.cache/`; re-run it in an interactive terminal when lat reports a newer
  init version.
- After changing source files, run `codegraph sync`. The index auto-syncs only
  while its MCP daemon runs, and `codegraph status` can report "up to date" for a
  stale index, so confirm a new or renamed symbol with `codegraph query <symbol>`
  before relying on the index. Never commit `.codegraph/`.

%% lat:begin %%
# Before starting work

- Run `lat search` to find sections relevant to your task. Read them to understand the design intent before writing code.
- Run `lat expand` on user prompts to expand any `[[refs]]` — this resolves section names to file locations and provides context.

# Post-task checklist (REQUIRED — do not skip)

After EVERY task, before responding to the user:

- [ ] Update `lat.md/` if you added or changed any functionality, architecture, tests, or behavior
- [ ] Run `lat check` — all wiki links and code refs must pass
- [ ] Do not skip these steps. Do not consider your task done until both are complete.

---

# What is lat.md?

This project uses [lat.md](https://www.npmjs.com/package/lat.md) to maintain a structured knowledge graph of its architecture, design decisions, and test specs in the `lat.md/` directory. It is a set of cross-linked markdown files that describe **what** this project does and **why** — the domain concepts, key design decisions, business logic, and test specifications. Use it to ground your work in the actual architecture rather than guessing.

# Commands

```bash
lat locate "Section Name"      # find a section by name (exact, fuzzy)
lat refs "file#Section"        # find what references a section
lat search "natural language"  # semantic search across all sections
lat expand "user prompt text"  # expand [[refs]] to resolved locations
lat check                      # validate all links and code refs
```

Run `lat --help` when in doubt about available commands or options.

If `lat search` fails because no API key is configured, explain to the user that semantic search requires a key provided via `LAT_LLM_KEY` (direct value), `LAT_LLM_KEY_FILE` (path to key file), or `LAT_LLM_KEY_HELPER` (command that prints the key). Supported key prefixes: `sk-...` (OpenAI) or `vck_...` (Vercel). If the user doesn't want to set it up, use `lat locate` for direct lookups instead.

# Syntax primer

- **Section ids**: `lat.md/path/to/file#Heading#SubHeading` — full form uses project-root-relative path (e.g. `lat.md/tests/search#RAG Replay Tests`). Short form uses bare file name when unique (e.g. `search#RAG Replay Tests`, `cli#search#Indexing`).
- **Wiki links**: `[[target]]` or `[[target|alias]]` — cross-references between sections. Can also reference source code: `[[src/foo.ts#myFunction]]`.
- **Source code links**: Wiki links in `lat.md/` files can reference functions, classes, constants, and methods in TypeScript/JavaScript/Python/Rust/Go/C files. Use the full path: `[[src/config.ts#getConfigDir]]`, `[[src/server.ts#App#listen]]` (class method), `[[lib/utils.py#parse_args]]`, `[[src/lib.rs#Greeter#greet]]` (Rust impl method), `[[src/app.go#Greeter#Greet]]` (Go method), `[[src/app.h#Greeter]]` (C struct). `lat check` validates these exist.
- **Code refs**: `// @lat: [[section-id]]` (JS/TS/Rust/Go/C) or `# @lat: [[section-id]]` (Python) — ties source code to concepts

# Test specs

Key tests can be described as sections in `lat.md/` files (e.g. `tests.md`). Add frontmatter to require that every leaf section is referenced by a `// @lat:` or `# @lat:` comment in test code:

```markdown
---
lat:
  require-code-mention: true
---
# Tests

Authentication and authorization test specifications.

## User login

Verify credential validation and error handling for the login endpoint.

### Rejects expired tokens
Tokens past their expiry timestamp are rejected with 401, even if otherwise valid.

### Handles missing password
Login request without a password field returns 400 with a descriptive error.
```

Every section MUST have a description — at least one sentence explaining what the test verifies and why. Empty sections with just a heading are not acceptable. (This is a specific case of the general leading paragraph rule below.)

Each test in code should reference its spec with exactly one comment placed next to the relevant test — not at the top of the file:

```python
# @lat: [[tests#User login#Rejects expired tokens]]
def test_rejects_expired_tokens():
    ...

# @lat: [[tests#User login#Handles missing password]]
def test_handles_missing_password():
    ...
```

Do not duplicate refs. One `@lat:` comment per spec section, placed at the test that covers it. `lat check` will flag any spec section not covered by a code reference, and any code reference pointing to a nonexistent section.

# Section structure

Every section in `lat.md/` **must** have a leading paragraph — at least one sentence immediately after the heading, before any child headings or other block content. The first paragraph must be ≤250 characters (excluding `[[wiki link]]` content). This paragraph serves as the section's overview and is used in search results, command output, and RAG context — keeping it concise guarantees the section's essence is always captured.

```markdown
# Good Section

Brief overview of what this section documents and why it matters.

More detail can go in subsequent paragraphs, code blocks, or lists.

## Child heading

Details about this child topic.
```

```markdown
# Bad Section

## Child heading

Details about this child topic.
```

The second example is invalid because `Bad Section` has no leading paragraph. `lat check` validates this rule and reports errors for missing or overly long leading paragraphs.
%% lat:end %%
