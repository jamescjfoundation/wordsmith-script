# SPDX-License-Identifier: MIT OR Apache-2.0
"""Transcribe three public-domain photoplay scenarios from Project Gutenberg into Fountain.

The scripts are the golden corpus for pagination and PDF tests (docs/golden.md). Each was
printed in the United States before 1931 and is in the public domain there; sources and dates
are in docs/provenance.md. Run it once with the three Gutenberg plain-text files; the results
are reviewed by eye and committed under tests/golden/.

    python3 tools/make-golden-scripts.py WRITING_THE_PHOTOPLAY.txt HOW_TO_WRITE_PHOTOPLAYS.txt THE_PHOTODRAMA.txt OUT_DIR

How 1910s scenario form maps to screenplay elements (the texts are kept word for word, only
re-wrapped):

- numbered scene line   -> Scene Heading. "Interior ..." -> INT., "Exterior ..." -> EXT.,
                           anything else is a forced heading (leading "."; quote marks around
                           its first word, or brackets around the whole line, are dropped, since Fountain needs a letter after the
                           "."); the scene's own number is kept as a Fountain scene number (#12#).
- description           -> Action.
- leader, caption, subtitle, insert (a title card)
                        -> Action, "TITLE: ...", "INSERT: ..." or "LETTER: ...".
- a spoken title card whose speaker the text right before it names
                        -> Character and Dialogue.
- "(CONTINUED.)" scene lines, running heads, footnotes and the books' teaching notes are left
  out.
"""

import re
import sys
from pathlib import Path

INTRO = re.compile(r"^(interior|exterior)\b[\s,.:;-]*(of\s+)?", re.I)


def heading(place: str, number: str) -> str:
    place = place.strip().rstrip(".").strip()
    m = INTRO.match(place)
    if m:
        kind = "INT." if m.group(1).lower() == "interior" else "EXT."
        rest = place[m.end():].strip(" -.,") or "UNKNOWN"
        text = f"{kind} {rest.upper()}"
    else:
        # Fountain forces a heading only when "." is followed by a letter or digit, so quote
        # marks around the first word are dropped ('"COPS" BEFORE ...' -> 'COPS BEFORE ...').
        place = re.sub(r'^"([^"]*)"', r"\1", place)
        if place.startswith("(") and place.endswith(")"):
            place = place[1:-1]
        text = "." + place.upper()
    return f"{text} #{number}#"


def paragraphs(lines):
    """Join wrapped lines into paragraphs (blank lines separate them)."""
    out, cur = [], []
    for line in lines:
        if line.strip():
            cur.append(line.strip())
        elif cur:
            out.append(" ".join(cur))
            cur = []
    if cur:
        out.append(" ".join(cur))
    return out


def clean(text: str) -> str:
    text = re.sub(r"\[\d+\]", "", text)  # footnote marks
    text = text.replace("_", "")  # italics
    text = text.replace("“", '"').replace("”", '"').replace("’", "'").replace("‘", "'")
    return re.sub(r"\s+", " ", text).strip()


SPEECH = re.compile(r"\b(say|says|saying|said|ask|asks|asking|asked|replies|replying|explains|exclaims|cries|calls|tells|answers|declares|whispers|retorts|adds)\b", re.I)


def speaker(before: str, cast):
    """The cast member the last sentence of `before` makes the speaker, if any."""
    sentences = re.split(r"(?<=[.!?])\s+", before.strip())
    last = sentences[-1] if sentences else ""
    if not SPEECH.search(last):
        return None
    verb = SPEECH.search(last)
    cut = verb.start() if verb else len(last)
    # The cast member named nearest before the speech verb ("... as Blinker says:").
    found = [(m.start(), n) for n in cast for m in re.finditer(r"\b" + re.escape(n) + r"\b", last[:cut])]
    return max(found)[1] if found else None


def title(kind: str, text: str) -> str:
    text = clean(text).strip()
    # Drop quote marks only when they enclose the whole card.
    if len(text) > 1 and text.startswith('"') and text.endswith('"') and text.count('"') == 2:
        text = text[1:-1].strip()
    return f"{kind}: {text}"


def emit(blocks):
    """Fountain text from (kind, text) blocks."""
    out = []
    for kind, text in blocks:
        if kind == "heading":
            out.append(text)
        elif kind == "action":
            # A line of capitals could read as a Character cue: force it to Action.
            out.append(("!" + text) if text.isupper() and len(text) < 40 else text)
        elif kind == "speech":
            name, line = text
            out.append(f"{name.upper()}\n{line}")
        out.append("")
    return "\n".join(out).rstrip() + "\n"


def title_page(title_text, credit, source):
    return f"Title: {title_text}\nCredit: {credit}\nSource: {source}\n\n"


def everybodys_girl(path: Path) -> str:
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    chapter = next(i for i, l in enumerate(lines) if l.strip().startswith("COMPLETE FIVE-REEL PHOTOPLAY SCRIPT"))
    start = next(i for i, l in enumerate(lines) if i > chapter and l.strip() == "SCENARIO, OR CONTINUITY OF SCENES")
    end = next(i for i, l in enumerate(lines) if i > start and l.strip() == "CHAPTER XXI")
    body = lines[start + 1 : end]
    cast = ["Florence", "Ella", "Blinker", "Bill", "Frank", "Oldport", "Watchdog"]
    blocks, last_action = [], ""
    i = 0
    while i < len(body):
        line = body[i]
        s = line.strip()
        if s.startswith("[Footnote"):
            while i < len(body) and body[i].strip():
                i += 1
            continue
        m = re.match(r"^(\d+)--(.*)$", s)
        if m:
            # A scene line can wrap onto the next line.
            place = m.group(2)
            i += 1
            while i < len(body) and body[i].strip() and not body[i].startswith(" "):
                place += " " + body[i].strip()
                i += 1
            blocks.append(("heading", heading(clean(place), m.group(1))))
            continue
        lm = re.match(r"^_(Cut-in[- ]leader|Leader)--_$", s, re.I)
        if lm:
            i += 1
            chunk = []
            while i < len(body) and not body[i].strip():
                i += 1
            while i < len(body) and body[i].strip():
                chunk.append(body[i])
                i += 1
            text = clean(" ".join(c.strip() for c in chunk))
            who = speaker(last_action, cast) if lm.group(1).lower().startswith("cut-in") else None
            if who and text.startswith('"'):
                blocks.append(("speech", (who, text.strip('"'))))
            else:
                blocks.append(("action", title("TITLE", text)))
            continue
        im = re.match(r'^_?Cut-in-? ?leader_?--_?\s*(\S.*)$', s, re.I)
        if im:
            # A cut-in leader written inline: 'Cut-in-leader--"TEXT ..."' (may wrap).
            chunk = [im.group(1)]
            i += 1
            while i < len(body) and body[i].strip():
                chunk.append(body[i].strip())
                i += 1
            text = clean(" ".join(chunk))
            who = speaker(last_action, cast)
            if who and text.startswith('"'):
                blocks.append(("speech", (who, text.strip('"'))))
            else:
                blocks.append(("action", title("TITLE", text)))
            continue
        if s:
            chunk = []
            while i < len(body) and body[i].strip() and not re.match(r"^(\d+)--", body[i].strip()):
                chunk.append(body[i])
                i += 1
            text = clean(" ".join(c.strip() for c in chunk))
            if text and not text.startswith("[Footnote"):
                blocks.append(("action", text))
                last_action = text
            continue
        i += 1
    return title_page("Everybody's Girl", "Adapted by A. Van Buren Powell from O. Henry's \"Brickdust Row\"", "Esenwein and Leeds, Writing the Photoplay (1919), Project Gutenberg #17903") + emit(blocks)


def timid_teddy(path: Path) -> str:
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    start = next(i for i, l in enumerate(lines) if re.match(r"^\s*SCENE 1:", l))
    end = next(i for i, l in enumerate(lines) if i > start and l.strip() == "FINIS.")
    cast = ["Teddy", "Jackson", "Edith", "Mrs. Harcourt", "Ransom", "Janice", "Stone", "Martha", "Butler"]
    blocks, last_action = [], ""
    for para in paragraphs(lines[start:end]):
        p = clean(para)
        if not p or p == "Timid Teddy":
            continue
        m = re.match(r"^SCENE (\d+): (.*)$", p, re.I)
        if m:
            place = re.sub(r"\(same as \d+\.?\)|same as \d+\.?", "", m.group(2), flags=re.I).strip(" .")
            blocks.append(("heading", heading(place or "SAME", m.group(1))))
            continue
        m = re.match(r"^SUBTITLE NO\. ?\d+[.:]?-*\s*(.*)$", p, re.I)
        if m:
            text = m.group(1)
            who = speaker(last_action, cast)
            if who and text.startswith('"'):
                blocks.append(("speech", (who.replace("Mrs. ", "Mrs "), text.strip('"'))))
            else:
                blocks.append(("action", title("TITLE", text)))
            continue
        blocks.append(("action", p))
        last_action = p
    return title_page("Timid Teddy", "By Embrie Zuver", "Zuver, How to Write Photoplays (1915), Project Gutenberg #76829") + emit(blocks)


def salt_of_vengeance(path: Path) -> str:
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    start = next(i for i, l in enumerate(lines) if l.strip() == "_Scenario_:")
    end = next(i for i, l in enumerate(lines) if i > start and l.strip() == "THE END.")
    cast = ["Dalton", "Donnelly", "Florence", "Arthur", "Mrs. Dalton", "Mrs. Donnelly", "Doctor", "doctor"]
    blocks, last_action = [], ""
    for para in paragraphs(lines[start + 1 : end]):
        p = clean(para)
        if not p:
            continue
        if re.match(r"^SCENE \d+\. \(CONTINUED\.?\)$", p, re.I):
            continue
        m = re.match(r"^SCENE (\d+)\. (.*)$", p, re.I)
        if m:
            place = re.sub(r"^SAME AS \d+\.\s*", "", m.group(2), flags=re.I)
            place = re.sub(r"\(.*?\)", "", place).strip(" .")
            blocks.append(("heading", heading(place, m.group(1))))
            continue
        m = re.match(r"^CAPTION \d+\.\s*(.*)$", p)
        if m:
            blocks.append(("action", title("TITLE", m.group(1))))
            continue
        m = re.match(r"^INSERT \([IVXL]+\)\s*(DIALOG\.|LETTER\.|News head,?)?\s*(.*)$", p, re.I)
        if m:
            kind = (m.group(1) or "").upper()
            text = m.group(2).lstrip(". ")
            if kind.startswith("DIALOG"):
                who = speaker(last_action, cast)
                if who and text.startswith('"'):
                    blocks.append(("speech", (who, text.strip('"'))))
                    continue
                blocks.append(("action", title("TITLE", text)))
            else:
                blocks.append(("action", title("LETTER" if kind.startswith("LETTER") else "INSERT", text)))
            continue
        blocks.append(("action", p))
        last_action = p
    return title_page("The Salt of Vengeance", "By Henry Albert Phillips", "Phillips, The Photodrama (1914), Project Gutenberg #70937") + emit(blocks)


def main():
    if len(sys.argv) != 5:
        sys.exit(__doc__)
    a, b, c, out = (Path(x) for x in sys.argv[1:])
    out.mkdir(parents=True, exist_ok=True)
    (out / "everybodys-girl.fountain").write_text(everybodys_girl(a), encoding="utf-8")
    (out / "timid-teddy.fountain").write_text(timid_teddy(b), encoding="utf-8")
    (out / "salt-of-vengeance.fountain").write_text(salt_of_vengeance(c), encoding="utf-8")


if __name__ == "__main__":
    main()
