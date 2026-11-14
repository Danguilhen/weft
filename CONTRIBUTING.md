# Contributing to Weft

Read [`AGENTS.md`](AGENTS.md) first. It is the operational guide for both humans
and agents.

## Design-first process

Weft is documented before it is implemented. Before changing behaviour:

1. Read `docs/STATUS.md`, `docs/PLAN.md` and the owning ADR.
2. If the change conflicts with an ADR, open a new ADR or update the existing one
   before writing code.
3. If a choice is listed in `docs/OPEN_QUESTIONS.md`, do not decide it silently
   in a pull request. Propose an answer and update the question.
4. Update the API, protocol or schema document together with the code.

## What to work on

`docs/PLAN.md` is the open-work queue. Pick the first unblocked task, or a task
explicitly assigned to you. Do not start a later phase while an earlier phase's
exit criteria are unmet.

## Licensing

- Server/federation/CLI contributions: AGPL-3.0-or-later.
- `weft-core` and `weft-client` contributions: MIT.
- Keep SPDX headers on every source file.
- There is no copyright assignment and no CLA.

## Commits

- One scoped change per commit.
- Use an imperative subject, optionally with a conventional prefix:
  `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`.
- Explain protocol or security decisions in the body.
- Do not include generated build output or local runtime state.

## Before opening a pull request

- Run `python3 tools/check_docs.py`.
- Run the Rust checks once the workspace exists:
  `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets`,
  `cargo test --workspace`.
- Add or update tests for behaviour changes.
- Update `docs/STATUS.md` and remove the finished bullet from `docs/PLAN.md`.
- Add an ADR for a new decision.

## Code of conduct

Be civil, assume good faith and keep discussions technical. Harassment,
discrimination and disclosure of private information are not acceptable.
Moderation of the project itself is described in
[`docs/GOVERNANCE.md`](docs/GOVERNANCE.md).

## Security

Do not report vulnerabilities in public issues. See [`SECURITY.md`](SECURITY.md).
