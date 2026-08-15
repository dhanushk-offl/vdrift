# vdrift

Detect, propose, synchronize, and verify versions across your codebase.
Version drift shouldn't happen.

A global Git integration stops pushes whose changes should have bumped the
project version — and a JSON agent API makes the whole cycle deterministic
for CI and automation.

## Why

Every project has one source of truth for its version (`package.json`,
`Cargo.toml`, a Helm `Chart.yaml`, ...). Everything else — lockfiles, docs,
generated files — is a reference that must agree. vdrift keeps them in sync
and blocks the mistake at the moment it's cheapest to fix: before the push.

## Install

```sh
cargo install --path .
vdrift init        # once per machine: installs the global pre-push hook
```

`vdrift init` writes a tiny dispatcher into the OS config directory and sets
`git config --global core.hooksPath` to it. It never touches per-repository
hook files and records any pre-existing `core.hooksPath` so `vdrift uninstall`
can restore it.

## Usage

| Command | Purpose |
| --- | --- |
| `vdrift` | Interactive flow: analyze, propose, confirm, apply, commit |
| `vdrift scan` | Discover project, sources, references, candidates |
| `vdrift check` | Report drift; exit 1 when anything is out of sync |
| `vdrift plan` | Read-only proposal (`patch` / `minor` / `major`) |
| `vdrift bump <patch\|minor\|major\|X.Y.Z>` | Bump to a version and sync |
| `vdrift sync` | Synchronize references to the canonical version |
| `vdrift apply --version X.Y.Z [--commit]` | Non-interactive update |
| `vdrift verify [--ci]` | Assert consistency after changes |
| `vdrift status` / `doctor` / `disable` / `uninstall` | Manage the installation |
| `vdrift hook pre-push` | Internal entry point used by the dispatcher |

All commands support `--json`, `--dry-run`, and `-C <dir>`.

## How it works

1. **Detect** — the canonical version is resolved with an explicit priority
   order (npm > Cargo/Tauri > Python/Maven > generic configured source).
   Lockfiles and generated files become *derived* references; configured files
   become *reference* references; a gitignore-aware scan finds *candidates*
   (never written without configuration).
2. **Propose** — commit subjects since the last version change are classified
   with deterministic Conventional Commits rules: `feat` → minor, `fix`/
   `perf`/`refactor` → patch, `!`/`BREAKING CHANGE` → major.
3. **Synchronize** — writes the canonical version to every writable reference.
   Dirty files are refused unless `--force`; all dirty checks happen before
   any write so a failed run never leaves a half-applied tree.
4. **Verify** — re-detects and compares; `vdrift verify --ci` is the
   deterministic gate for pipelines.

## Supported ecosystems

| Ecosystem | Canonical source | Derived / synced references |
| --- | --- | --- |
| Node.js (npm / pnpm / bun / yarn) | `package.json` | `package-lock.json`, `pnpm-lock.yaml`, `bun.lock`, `yarn.lock` |
| Rust / Tauri | `Cargo.toml`, `tauri.conf.json` | `Cargo.lock`, `src-tauri/Cargo.toml` |
| Go | `version.go` (`var/const Version`) | — |
| Python | `pyproject.toml`, `setup.py`, `setup.cfg` | `_version.py` / `__init__.py` (`__version__`) |
| Java / JVM | `pom.xml`, `build.gradle(.kts)`, `gradle.properties` | — |
| Dart / Flutter | `pubspec.yaml` (`version: 1.2.0+5`) | — |
| Elixir | `mix.exs` (`version: "…"`) | — |
| PHP | `composer.json` | — |
| Ruby | `*.gemspec` (`spec.version =`) | — |
| Haskell | `package.yaml` / `*.cabal` (`version:`) | — |
| Anything else | configured `[version] source` | configured `[references] files` |

Multiple canonical sources may coexist; they must agree on the version or
`vdrift` refuses to guess (`MULTIPLE_VERSION_SOURCES`).

## Git integration

On every `git push`, the global pre-push dispatcher:

- skips when `CI`, `VDRIFT_SKIP`, or `VDRIFT_RUNNING` is set (recursion guard);
- skips when the repository opted out (`.vdrift.toml` `enabled = false`);
- analyzes the commits being pushed;
- if a version change is needed (or drift exists), runs the interactive flow
  and **stops the push** with exit 1 until the version commit is included.

## Configuration

`.vdrift.toml` (per repository):

```toml
[version]
source = "helm/Chart.yaml"        # explicit canonical source

[references]
files = ["README.md", "docs/version.json"]

[behavior]
enabled = true                    # set false to opt out of the global hook
auto_bump = false                 # skip the "update version?" prompt
auto_commit = false               # always create the version commit
```

Text files are updated by replacing their current version string. JSON, YAML,
and TOML references are updated structurally (top-level `version` key).

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | success / no drift |
| 1 | drift detected, or push stopped |
| 2 | configuration error |
| 3 | unsupported project |
| 4 | git error |
| 5 | cancelled |
| 6 | unsafe working tree |
| 7 | invalid version |
| 8 | adapter failure |
| 10 | internal error |

## Agents & CI

```sh
vdrift check --json            # exit 1 on drift, JSON report
vdrift plan --json             # read-only proposal
vdrift apply --version 1.2.4 --commit --force
vdrift verify --ci             # deterministic pass/fail
```

## Development

```sh
cargo build
cargo test     # unit + end-to-end CLI tests (all ecosystem fixtures)
cargo clippy --all-targets
```