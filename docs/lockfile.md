# Lockfile

`deco-lockfile` provides a provisional schema v1 for frozen resolution data.

Current model:
- `schema_version`
- `source.workspace_folder`
- `source.config_file`
- `targets[]` with `name`, `kind`, `reference`, `resolved_reference`, `digest`
- optional `metadata`

The crate currently owns modeling, validation, JSON parsing, and pretty serialization.
It is intentionally not wired into CLI behavior yet.
