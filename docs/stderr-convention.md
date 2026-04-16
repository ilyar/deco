# Stderr Convention

`deco` uses a simple split between machine output and human diagnostics:

- stdout is reserved for command payloads and structured envelopes;
- stderr carries informational messages, progress updates, warnings, and errors;
- stderr lines use a stable prefix so future commands can emit readable status without breaking parsers.

## Line Format

- info: `[deco:info] message`
- progress: `[deco:progress] stage: message`
- warning: `[deco:warning] message`

The helper lives in `deco_core_model::diagnostics` and is intentionally tiny. Command code can adopt it later without changing the contract.

## Intended Usage

- use `info` for coarse command lifecycle notes;
- use `progress` for stage-local status updates;
- use `warning` for recoverable fallbacks or degraded paths;
- keep stdout free of incidental text.

## Notes

This convention is documentation-first. No existing command behavior is changed by this helper until a command opts in.
