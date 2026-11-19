# 0001: License allowlist as SPDX ids

Date: 2026-11-19

## Context

`docs/REUSE.md` rule 5 allows MIT, Apache-2.0, BSD, ISC, OFL-1.1, CC0-1.0,
CC-BY-4.0, CC-BY-SA-4.0 and public domain. Cargo manifests state licenses as
SPDX expressions, so the check needs exact ids.

## Decision

`cargo xtask attribution` accepts these SPDX ids:

| Allowlist entry | SPDX ids |
| --- | --- |
| MIT | `MIT` |
| Apache-2.0 | `Apache-2.0`, also `Apache-2.0 WITH LLVM-exception` (the exception only adds permissions) |
| BSD | `BSD-2-Clause`, `BSD-3-Clause`, `0BSD` |
| ISC | `ISC` |
| OFL-1.1 | `OFL-1.1` |
| CC0-1.0, CC-BY-4.0, CC-BY-SA-4.0 | same ids |
| Public domain | `Unlicense` (a public-domain dedication) |

Expressions are evaluated properly: `A OR B` passes if either side is
allowed, `A AND B` only if both are. The older `A/B` form means OR. Anything
else, including a missing license field, fails the check.

Not on the list, so they stop the build and need a person's decision:
`Zlib`, `BSL-1.0`, `Unicode-3.0`, `MPL-2.0`, any GPL or LGPL, and any
custom license file. When the build needs one of these only as an OR
alternative, it passes without asking.

## Revisit when

A needed dependency carries a license outside this list. That is a stop
rule: ask the person before adding an exception, and record the answer here.
