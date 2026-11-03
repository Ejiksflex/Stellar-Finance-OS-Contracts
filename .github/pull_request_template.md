## What changed

<!-- One or two sentences. -->

## Checklist

- [ ] `./scripts/check.sh` passes (fmt, clippy -D warnings, tests)
- [ ] Every new/changed entrypoint has happy-path **and** failure-path tests
- [ ] No `mock_all_auths()` unless the test name says so
- [ ] README §3/§4 tables updated if the public surface changed
- [ ] `docs/threat-model.md` updated if a new threat or mitigation applies
- [ ] No secrets, seeds or identity files

## Breaking change?

- [ ] No
- [ ] Yes: function signature / event shape / error code changed (needs a major version bump and a `CHANGELOG.md` entry)
