# Profile assets

The profiles in this directory are retained version 1.0.0 compatibility assets.
Their versions and bytes remain stable so historical digests and verification
records keep their original meaning. They are not an implicit latest release.

In particular, `fullstack/profile.json` has 14 required sensors. The current
supported bootstrap template release is 1.0.2, whose fullstack profile has 15
required sensors and adds `shared:dependencies` to audit the shared npm graph.
For a new direct validation, select the release explicitly:

```console
rtk cargo run --locked -p project-verification -- validate-profile bootstrap/templates/1.0.2/profiles/fullstack/profile.json
```

For a consumer installation, select `--release 1.0.2`, review and apply the
bootstrap plan, then validate the installed `.policy/project-profile.json`.
Releases 1.0.0 and 1.0.1 remain embedded for explicit compatibility selection,
receipt validation, recovery, upgrade/downgrade testing, and uninstall.

See [project verification](../docs/integrations/project-verification.md) for
profile semantics and [bootstrap](../docs/integrations/bootstrap.md) for release
selection, pins, receipts, and removal.
