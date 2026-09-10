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
- Check documented commands, arguments, and exit codes against the implementation.
  After documentation changes, verify Markdown relative links and `git diff --check`.
