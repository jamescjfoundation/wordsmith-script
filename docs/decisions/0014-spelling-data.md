# 0014: Spelling data

Date: 2026-11-19 · Status: general part moved to the core
([core decision 0007](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0007-engine-agents-output.md))

Since decision 0044 (2026-11-24) spelling data and the personal dictionary come
from the core's `jcj-proof`; the core's record says which word lists it uses.
Script embeds no language data of its own and names no craft-libs crate.

Spelling ships without craft-i18n's SCOWL dictionaries. English spelling uses
craft-proof's built-in public-domain Moby word list (about 160,000 words) with craft-proof's
modern-word supplement, plus the system's Hunspell English dictionary where
one is installed. No language data is embedded at build time
(`CRAFT_I18N_DIR` stays unset).

Why: the SCOWL word lists and the Kuenning affix file carry their own
permissive notices, neither on the allowlist (`docs/decisions/0001`). The
Moby list needs no new licence. The cost is weaker coverage of newer words and
of British and Canadian spellings.

Consequences: the three WordCraft layout tests that need craft-i18n data stay
ignored (`docs/gaps.md`). Hyphenation patterns are not embedded either;
screenplays are not hyphenated, so nothing is lost there.

Revisit when: writers report missing words, or a release needs other
languages. Adding SCOWL then is its own licence decision.
