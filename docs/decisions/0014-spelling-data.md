# 0014: Spelling data

Date: 2026-11-19

The project owner chose (M3 start) to ship spelling without craft-i18n's
SCOWL dictionaries. English spelling uses craft-proof's built-in
public-domain Moby word list (about 160,000 words) with craft-proof's
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
