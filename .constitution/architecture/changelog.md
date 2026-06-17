# Architecture Changelog

## v0.1.0 — Initial Architecture

- Established local-first compilation pipeline (pipe-and-filter) pattern
- Defined 7 logical containers: CLI Entrypoint, Project Loader, Harness Registry, Validator, Template Engine, Output Router, Scaffolder
- Documented collect-all-errors strategy and atomic write safety
- Created 12 flow files mapping to all P0 capabilities
- PRD corrected (v0.1.1): BD-1/BD-2 default scope changed to project-level agent paths (deploy-first model)
- Identified 6 logical risks with mitigations
