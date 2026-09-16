# Profile assets

The profiles in this directory are retained version 1.0.0 compatibility assets.
Their versions and bytes remain stable so historical digests and verification
records keep their original meaning. They are not an implicit latest release.

In particular, `fullstack/profile.json` has 14 required sensors. The current
bootstrap template release is 1.1.0: its schema 2 fullstack profile has 17
required sensors, including shared npm and separate app/Unit/Integration NuGet
audits. Frozen 1.0.2 has 15 required sensors, including the shared npm audit.
For a new direct validation, select the release explicitly:

```console
rtk cargo run --locked -p project-verification -- validate-profile bootstrap/templates/1.1.0/profiles/fullstack/profile.json
```

For a consumer installation, select `--release 1.1.0`, review and apply the
bootstrap plan, then validate the installed `.policy/project-profile.json`.
Releases 1.0.0–1.0.2 remain embedded for explicit compatibility selection,
receipt validation, recovery, upgrade/downgrade testing, and uninstall.

See [project verification](../docs/integrations/project-verification.md) for
profile semantics and [bootstrap](../docs/integrations/bootstrap.md) for release
selection, pins, receipts, and removal.
