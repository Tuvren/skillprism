# ADR-006: YAML Deserialization via yaml_serde

**Status:** Accepted

## Context

The stack requires YAML deserialization for `skillprism.yaml`, `skill.yaml`, and harness definition files. The PRD's vision.md appendix (non-binding) names `serde_yaml`, but that crate is deprecated (latest version 0.9.34+deprecated) and no longer maintained. Three alternatives exist in the ecosystem:

| Alternative | Status | Notes |
| :--- | :--- | :--- |
| `yaml_serde` | Maintained (v0.10.x) | Fork of `serde_yaml` by The YAML Organization; MSRV 1.82; mirrors serde_yaml API closely |
| `serde_yml` | Maintained | Independent implementation; different API surface |
| `serde-yaml` (manual reimplementation) | N/A | Would require writing a YAML parser from scratch |

## Decision

Use `yaml_serde` 0.10.x. It is the official maintained fork of the deprecated `serde_yaml`, published by The YAML Organization under the same MIT/Apache-2.0 license. The API is near-identical to `serde_yaml` (`from_reader`, `from_str`, `to_string`), making migration trivial for anyone familiar with the original.

## Consequences

- **Positive:** Drop-in replacement for `serde_yaml`. Active maintenance by the YAML standards body. Same Serde `Deserialize`/`Serialize` derive pattern.
- **Negative:** Different crate name (`yaml_serde` vs `serde_yaml`) may cause confusion. MSRV 1.82 is higher than the rest of the dependency tree (our MSRV is 1.85, so no impact).
- **Mitigation:** Document the deprecation rationale in this ADR and in comments at the import site.
