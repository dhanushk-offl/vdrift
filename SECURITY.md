# Security Policy

vdrift is a developer tool that inspects your repository, reads manifests and
lockfiles, and (during sync/apply) writes version strings back to them. It also
installs a global Git pre-push hook. Please read the notes below before
reporting.

## Supported Versions

Only the latest release on the `main` branch is supported. If you are on an
older release, upgrade before reporting — the fix will land in the newest
version.

| Version | Supported |
| --- | --- |
| latest (`main` / latest tag) | ✅ |
| older releases | ❌ |

## What we take seriously

- **Code execution or privilege escalation** triggered by a repository, a
  `.vdrift.toml`, a manifest, or a lockfile that `vdrift` processes.
- **Path traversal / arbitrary file writes** via crafted references, config
  `files`, or `source` values.
- **Command injection** in any shell-out path (e.g. `git`, hooks).
- **Exposure of secrets** — vdrift must never read, log, or commit API keys or
  credentials, and must never write them to a file.
- **Hook takeover or bypass** — a way for untrusted content to make the global
  pre-push hook run arbitrary commands, or to silently disable drift checks
  when the user expects them.

## Reporting a vulnerability

Do **not** open a public issue for security problems. Instead, report privately:

- **GitHub Security Advisory (preferred):** use the "Report a vulnerability"
  button on the repository's *Security* tab.
- **Email:** reach the maintainer privately via the contact listed on the
  repository's GitHub profile.

Please include:

1. A short description of the issue and its impact.
2. Steps to reproduce, ideally with a minimal test repository.
3. The affected version and platform.
4. Any suggested fix, if you have one.

We aim to acknowledge reports within **48 hours** and to triage within **5
business days**. You will receive updates as we work toward a fix and release.
Once a fix ships, we are happy to credit you (if you want) in the release notes.

## Disclosure policy

We follow **coordinated disclosure**: we work with the reporter on a fix and a
release before publicizing the issue. We ask that reporters wait for the fix to
ship before publishing details.

## Safety guarantees you can rely on

- vdrift never pushes, force-pushes, or rewrites history.
- vdrift refuses to modify files with uncommitted changes unless `--force` is
  passed, and checks every target before writing anything.
- The pre-push hook skips when `CI`, `VDRIFT_SKIP`, or `VDRIFT_RUNNING` is set.
- Reading a repository is best-effort and non-destructive; detection never
  writes.