# Contributing to vdrift

Thanks for wanting to contribute! This guide covers the development setup, the
codebase layout, how version detection works, and how to add support for a new
ecosystem. Reading it will make reviews faster and your first PR smoother.

## Table of contents

- [Development setup](#development-setup)
- [Common commands](#common-commands)
- [Codebase overview](#codebase-overview)
- [How detection works](#how-detection-works)
- [Adding a new ecosystem adapter](#adding-a-new-ecosystem-adapter)
- [Testing conventions](#testing-conventions)
- [Commit & PR guidelines](#commit--pr-guidelines)
- [Before you submit](#before-you-submit)

## Development setup

**Prerequisites**

- [Rust](https://rustup.rs/) (stable toolchain; pin via `rust-toolchain.toml` if
  the project adds one). rustfmt and clippy components are used in CI.
- `git` (required — the integration test suite creates real repositories and
  exercises the pre-push hook).
- A Linux or macOS host (Windows works for most commands but is not exercised
  in CI today).

**Clone and build**

```sh
git clone https://github.com/dhanushk-offl/vdrift.git
cd vdrift
cargo build
cargo run -- --help
```

That's it — no other services, no database, no environment variables. The
binary reads/writes the local repository only.

## Common commands

| Task | Command |
| --- | --- |
| Build (debug) | `cargo build` |
| Build (release) | `cargo build --release` |
| Run the CLI | `cargo run -- <args>` (e.g. `cargo run -- scan --json`) |
| Format check | `cargo fmt --all -- --check` |
| Lint (CI-clean) | `cargo clippy --all-targets -- -D warnings` |
| All tests | `cargo test` |
| Unit tests only | `cargo test --lib` |
| One integration test | `cargo test --test cli <name>` |

CI runs the exact commands above; if they pass locally, the pipeline will too.

## Codebase overview

```
src/
  main.rs                 CLI entry point, arg parsing, exit-code mapping
  errors.rs               error types -> exit codes + JSON error payloads
  config/
    project.rs            .vdrift.toml parsing, defaults, helpers
  git/
    repository.rs         repo discovery, dirty-tree checks, commit helper
  core/
    project.rs            ProjectType / PackageManager detection
    detection.rs          reference discovery, canonical resolution, scan
    proposal.rs           Conventional Commits -> bump level
    synchronization.rs    plan_changes / sync_references (writes)
    verification.rs       verify() consistency check
    version.rs            semver wrapper + bump logic
  adapters/
    mod.rs                VersionAdapter trait, registry, routing
    util.rs               shared JSON/YAML/TOML/XML/line text helpers
    <ecosystem>.rs        one file per supported ecosystem
tests/
  cli.rs                  end-to-end tests: real git repos + the real binary
```

The rule of thumb: **`core/` has no format-specific knowledge and `adapters/`
has no business logic.** A new file format means a new adapter, never a change
to `core/`.

## How detection works

Every version-bearing location is a **reference** with one of four kinds:

| Kind | Meaning | Writable |
| --- | --- | --- |
| `canonical` | source of truth (e.g. `package.json`) | yes |
| `derived` | generated from the canonical (e.g. lockfiles) | yes |
| `reference` | configured / known file that must agree | yes |
| `candidate` | found by heuristic scanning | **no** |

`detect()` (in `core/detection.rs`) runs every adapter in registry order, then
a config-driven generic adapter, dedupes by path, and resolves the canonical:

1. An explicit `[version] source` in `.vdrift.toml` wins and demotes all
   auto-detected canonicals to references.
2. Otherwise the highest-priority canonical with a known version is chosen
   (`canonical_priority`); agreeing extras are demoted to references.
3. If two canonicals disagree → `MULTIPLE_VERSION_SOURCES` error.

Writable references whose current version is unknown are skipped during sync
(auto-detected) — configured `reference`-kind files without a resolvable version
are an error so the user fixes the config.

## Adding a new ecosystem adapter

Say you want to support a new package manager. The steps:

1. **Study an existing adapter** — `src/adapters/npm.rs` (JSON manifest +
   lockfile) and `src/adapters/go.rs` (line-keyed text) are the simplest.
   Reuse helpers from `src/adapters/util.rs` instead of writing your own
   parsing when possible.

2. **Create `src/adapters/<name>.rs`** implementing `VersionAdapter`:

   ```rust
   pub struct FooAdapter;

   impl super::VersionAdapter for FooAdapter {
       fn detect(&self, repo: &Repository) -> Result<Vec<VersionReference>> {
           // Return every version-bearing file this ecosystem has.
           // Use ReferenceKind::Canonical for the source of truth,
           // ReferenceKind::Derived for lockfiles.
       }
       fn update(&self, reference: &VersionReference, version: &Version) -> Result<()> {
           // Write the version into `reference.file`. Never rewrite history,
           // never push, never touch other files.
       }
   }
   ```

   Keep `update` surgical: preserve the surrounding file byte-for-byte where
   practical (see the quick-xml POM helpers and the TOML section writers).

3. **Register it** in `src/adapters/mod.rs`:
   - `pub mod <name>;`
   - add it to `all()` (registry order matters if files can overlap);
   - add its filenames/extensions to `adapter_for()` and to
     `is_known_manifest()` so the generic adapter never claims them.

4. **Extend project detection** (`src/core/project.rs`) so `scan`/`check`
   report the right `ProjectType` and `PackageManager` for the new ecosystem,
   and extend `canonical_priority` if the new manifest should rank above or
   below existing ones.

5. **Test it** — see [Testing conventions](#testing-conventions).

6. **Update the README** ecosystem matrix table.

## Testing conventions

- **Unit tests** live in `#[cfg(test)]` modules inside the source files
  (pure logic only — e.g. version bumping, Conventional Commits parsing).
- **Integration tests** live in `tests/cli.rs` and spawn the real binary
  against a real (temporary) git repository via the `fixture()` helper:

  ```rust
  #[test]
  fn my_ecosystem_is_synced() {
      let d = fixture("myeco");
      write(&d.path().join("manifest.json"), "{\"version\": \"1.0.0\"}\n");
      commit_all(d.path(), "chore: initial");
      let out = run(vdrift().args(["check", "--json"]).current_dir(d.path()));
      assert_eq!(out.status.code(), Some(0), "{}", stdout(&out));
  }
  ```

- Helpers available in `tests/cli.rs`: `fixture`, `write`, `commit_all`,
  `init_npm_fixture`, `vdrift()`, `run`, `stdout`, `stderr`, and the hook
  helpers `feed_hook_stdin` / `hook_stdin_line`.
- Every new adapter needs at least one integration test proving: detection,
  a sync/apply round-trip that writes the new version, and a clean `check`.
- Keep tests deterministic: no networking, no fixed clock, no prompts.

## Commit & PR guidelines

- Branch from `main`; open a PR with a focused scope.
- Use [Conventional Commits](https://www.conventionalcommits.org/) subjects —
  the project itself classifies them (`feat`, `fix`, `perf`, `refactor`,
  `docs`, `chore`, `test`, `ci`, `style`, `!` for breaking changes). Example:
  `feat: add foo ecosystem adapter`.
- Describe the change and why; reference any issue numbers.
- Keep the diff reviewable — split unrelated changes into separate PRs.

## Before you submit

Run, from the repo root, and confirm all pass:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

CI enforces all three (`.github/workflows/ci.yml`). A release build
(`cargo build --release`) is also part of CI.

Questions? Open a discussion or ask in your PR. Thank you for contributing!