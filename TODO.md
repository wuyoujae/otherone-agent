# Rust Agent Streaming Stabilization TODO

## Goal

Make the Rust `otherone-agent` framework usable for desktop chat streaming without requiring the UI to switch sessions before replies appear.

## Checklist

- [x] Fix OpenAI-compatible SSE parsing so JSON events split across TCP chunks are preserved.
- [x] Add tests for split SSE frames, multiple frames per chunk, and `[DONE]` handling.
- [x] Preserve OpenAI-compatible extra request parameters such as `reasoning_effort`.
- [x] Fix streamed tool-call accumulation to merge by `index` like the TS framework.
- [x] Store assistant `tool_calls` in localfile/database entries.
- [x] Expose configurable localfile storage root without requiring global `current_dir`.
- [x] Keep old current-dir behavior as the default for compatibility.
- [x] Add a live ignored stream test using environment variables.
- [x] Run workspace checks and targeted tests.
- [x] Publish all `0.1.2` crates.
- [x] Switch desktop dependency from local path back to published `otherone = "0.1.2"`.

## Current Publish State

- Published or confirmed already uploaded on crates.io: `otherone-ai 0.1.2`, `otherone-storage 0.1.2`, `otherone-context 0.1.2`, `otherone-tools 0.1.2`, `otherone-mcp 0.1.2`, `otherone-agent 0.1.2`, `otherone 0.1.2`.

## Rollback

- Desktop can temporarily use the local path dependency to validate the fixed framework.
- Once crates.io is reachable, publish remaining crates and set desktop back to `otherone = "0.1.2"`.
