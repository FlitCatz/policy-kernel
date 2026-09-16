# Task index

This directory is the canonical starting point for Tasks 1–15. Each landing page
summarizes the latest known state and points to the implementation, reference
documentation, and archived evidence for that Task.

## Status vocabulary

- **Implemented**: the repository contains the intended implementation.
- **Planned**: a work plan exists; its implementation and validation are not complete.
- **Locally verified**: a recorded local or fixture-backed execution supports the
  stated behavior, within the record's limits.
- **CI observed**: a dated record contains a specific CI observation; it is not a
  standing claim about later commits.
- **Not demonstrated in production**: required live, authenticated, exact-head,
  protected-merge, or independent-review evidence is absent.

Dated plans, designs, and reviews are historical evidence. Their original status
statements are not rewritten when later work changes the current state.

| Task | Capability | Current status |
| --- | --- | --- |
| [Task 01](task-01/README.md) | Foundation, provenance, and obligation baseline | Implemented |
| [Task 02](task-02/README.md) | Versioned contracts and strict deserialization | Implemented |
| [Task 03](task-03/README.md) | Canonical bundle integrity and trust validation | Core implemented; mandatory knowledge enforcement tracked in Task 15 |
| [Task 04](task-04/README.md) | Sensor applicability and test adequacy | Implemented |
| [Task 05](task-05/README.md) | Review consensus and human waivers | Implemented |
| [Task 06](task-06/README.md) | Pre-merge evaluation | Implemented |
| [Task 07](task-07/README.md) | Post-merge evaluation | Implemented |
| [Task 08](task-08/README.md) | Advisory CLI and generated schemas | Implemented |
| [Task 09](task-09/README.md) | Bounded evidence collection | Implemented |
| [Task 10](task-10/README.md) | GitHub Actions authoritative boundary | Implemented; production evidence remains incomplete |
| [Task 11](task-11/README.md) | Release gates and rollback drill | Implemented; production promotion not demonstrated |
| [Task 12](task-12/README.md) | Project profiles and report normalization | Implemented; historical records retain their original verification state |
| [Task 13](task-13/README.md) | Transactional bootstrap installer | Implemented; historical records retain their original verification state |
| [Task 14](task-14/README.md) | Consuming-project acceptance | Locally verified; authoritative production evidence not demonstrated |
| [Task 15](task-15/README.md) | Compatibility, runtime independence, and verification completion | Planned; implementation has not started |

For the repository-wide latest status, read [Current status](../status.md).
