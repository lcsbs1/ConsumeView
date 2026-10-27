# Lockfiles & icons

These tracked assets were not uploaded through the Github MCP text path in the MVP push:

- `pnpm-lock.yaml` — run `pnpm install`
- `src-tauri/Cargo.lock` — run `cd src-tauri && cargo generate-lockfile` (or `cargo build`)
- `src-tauri/icons/*` — binary PNG/ICO/ICNS; copy from a local clone or `git push` with a user PAT

All application source (Rust domain/proxy/db/secrets, React UI, configs) is on this branch.
