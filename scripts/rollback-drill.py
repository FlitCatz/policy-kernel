#!/usr/bin/env python3
"""Check supplied normalized rollback snapshots; never call GitHub or write files."""
import argparse
import json
from pathlib import Path
import re
import sys

KERNEL_CHECK = "policy-kernel/pre-merge"
PROTECTION_FIELDS = {
    "schema_version", "repository", "branch", "required_checks",
    "required_approving_review_count", "dismiss_stale_reviews",
    "require_code_owner_reviews", "enforce_admins", "allow_force_pushes", "allow_deletions",
}


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON field: {key}")
        result[key] = value
    return result


def read_json(path):
    return json.loads(Path(path).read_text(), object_pairs_hook=unique_object)


def protection(value):
    if not isinstance(value, dict) or set(value) != PROTECTION_FIELDS:
        raise ValueError("protection snapshot has missing or unknown fields")
    if type(value["schema_version"]) is not int or value["schema_version"] != 1:
        raise ValueError("unsupported protection schema version")
    if not isinstance(value["repository"], str) or not re.fullmatch(r"[^/]+/[^/]+", value["repository"]):
        raise ValueError("invalid repository binding")
    if not isinstance(value["branch"], str) or not value["branch"].strip():
        raise ValueError("missing branch binding")
    count = value["required_approving_review_count"]
    if type(count) is not int or count < 1:
        raise ValueError("native human review requirement must remain enabled")
    for field in ("dismiss_stale_reviews", "require_code_owner_reviews", "enforce_admins", "allow_force_pushes", "allow_deletions"):
        if type(value[field]) is not bool:
            raise ValueError(f"{field} must be boolean")
    if value["allow_force_pushes"] or value["allow_deletions"]:
        raise ValueError("native baseline must prevent force push and branch deletion")
    checks = value["required_checks"]
    if not isinstance(checks, list) or not checks:
        raise ValueError("native required checks must be nonempty")
    identities = []
    for check in checks:
        if not isinstance(check, dict) or set(check) != {"context", "app_id"}:
            raise ValueError("invalid required-check identity")
        if not isinstance(check["context"], str) or not check["context"].strip():
            raise ValueError("required-check context must be nonempty")
        if type(check["app_id"]) is not int or check["app_id"] < 1:
            raise ValueError("required checks must bind a positive application identity")
        identities.append((check["context"], check["app_id"]))
    if len(set(identities)) != len(identities):
        raise ValueError("duplicate required checks")
    return dict(value, required_checks=sorted(checks, key=lambda check: (check["context"], check["app_id"])))


def evidence(value):
    if not isinstance(value, dict) or set(value) != {"schema_version", "artifacts"} or type(value["schema_version"]) is not int or value["schema_version"] != 1:
        raise ValueError("invalid evidence manifest")
    artifacts = value["artifacts"]
    if not isinstance(artifacts, list) or not artifacts:
        raise ValueError("evidence inventory must be nonempty")
    paths = set()
    for artifact in artifacts:
        if not isinstance(artifact, dict) or set(artifact) != {"path", "sha256"}:
            raise ValueError("invalid artifact entry")
        path, digest = artifact["path"], artifact["sha256"]
        if not isinstance(path, str) or not path or path in paths:
            raise ValueError("artifact paths must be nonempty and unique")
        if not isinstance(digest, str) or not re.fullmatch(r"[0-9a-f]{64}", digest):
            raise ValueError("artifact digest must be SHA-256")
        paths.add(path)
    return sorted(artifacts, key=lambda artifact: artifact["path"])


def verify(args):
    baseline = protection(read_json(args.baseline))
    promoted = protection(read_json(args.promoted))
    restored = protection(read_json(args.restored))
    if any(check["context"] == KERNEL_CHECK for check in baseline["required_checks"]):
        raise ValueError("native baseline must precede kernel promotion")
    kernel_checks = [check for check in promoted["required_checks"] if check["context"] == KERNEL_CHECK]
    if len(kernel_checks) != 1:
        raise ValueError("promoted snapshot must contain exactly one kernel check")
    without_kernel = dict(promoted, required_checks=[check for check in promoted["required_checks"] if check["context"] != KERNEL_CHECK])
    if without_kernel != baseline:
        raise ValueError("promotion changed the recorded native baseline")
    if restored != baseline:
        raise ValueError("rollback did not restore the exact native protection baseline")
    if evidence(read_json(args.evidence_before)) != evidence(read_json(args.evidence_after)):
        raise ValueError("rollback changed the supplied evidence inventory")
    return {"schema_version": 1, "status": "passed", "scope": "supplied-snapshots-only", "live_protection_verified": False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("baseline", "promoted", "restored", "evidence-before", "evidence-after"):
        parser.add_argument(f"--{name}", required=True)
    args = parser.parse_args()
    try:
        result = verify(args)
    except (ValueError, OSError, TypeError, KeyError) as error:
        print(f"rollback-drill: {error}", file=sys.stderr)
        return 64
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
