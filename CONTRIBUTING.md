# Contributing

Fleet-wide conventions, the build gate, and judgment boundaries are defined once
in [AGENTS.md](https://github.com/gaganjainse/shesh-ecosystem/blob/main/AGENTS.md).
Read that first.

## Before you start

```bash
make check          # or: pytest -q && ruff check .
```

A red gate on arrival is not yours to build on. Fix it or report it.

## Making a change

1. Branch as `feat/<slug>` or `fix/<slug>`. Never work on `main`.
2. Read the files you intend to change.
3. Keep the change small; one logical change per commit.
4. Add a test with a fix.
5. Run the gate before committing.
6. Use a Conventional Commit message: `feat:`, `fix:`, `docs:`, `refactor:`,
   `chore(ci):`.

## What blocks a merge

- A failing gate.
- A credential in the diff.
- A new dependency without justification and a licence check.
- A documented claim that the code does not support.
- A test weakened to make a change pass.

## Where things live

Product code ships to users and passes the release gate. Build tooling lives in
`shesh-workspace` and does not. Documentation lives in `shesh-docs`; this
repository's README stays canonical for how to build and run it.

See [HANDOFF.md](https://github.com/gaganjainse/shesh-ecosystem/blob/main/HANDOFF.md)
for the full work loop.
