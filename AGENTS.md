# AGENTS.md

Fleet-wide conventions, judgment boundaries, and the build gate are defined once
in the ecosystem repository:

**https://github.com/gaganjainse/shesh-ecosystem/blob/main/AGENTS.md**

Read that first, then
[HANDOFF.md](https://github.com/gaganjainse/shesh-ecosystem/blob/main/HANDOFF.md)
for where to continue work.

This file records only what differs in `SheshAOS`.

## This repository

`Product. The Rust governance kernel.`

See [README.md](README.md) for what this component does, how to build it, and
how to run its tests.

## Local notes

- Run this repository's own suite before committing. The ecosystem gate covers
  integration, not these units.
- The README is canonical here. Do not copy its content into `shesh-docs`;
  link to it.
