# Character classes — reference

Everything decided (and still open) about `[...]` syntax, gathered in one place.
This is a spec to implement against, not implementation itself — no code here.

## Grammar

```
atom       := ... | '[' class_body ']'
class_body := '^'? class_item+          -- '+' vs '*' is still open, see below
class_item := CHAR '-' CHAR             -- a range
            | CHAR                      -- a single member
```

A class matches **exactly one input character**, tested against a set. That set
is the union of every `class_item`: some are single characters, some are
ranges, and they all just add their members to one pool. Order among items
never matters — `[abc]` and `[cba]` are the same set.

## The three position-sensitive characters

There is no lexer (Lesson 1), so `^`, `-`, and `]` don't have fixed meanings —
their meaning depends entirely on *where* they sit. This is the whole reason
character classes are their own parsing problem.

### `^` — negation

Only negates when it is the **very first character** right after `[`.
Anywhere else, it's a literal `^`.

- `[^abc]` — negated: any char that is *not* `a`, `b`, or `c`.
- `[a^bc]` — not negated: the literal set `{a, ^, b, c}`.

### `-` — range operator

Only forms a range when it sits **between two characters that are each still
"available"** — not yet consumed as part of another item, and not the very
edge of the class.

The general rule (not two separate rules for "first" and "last"): a `-`
becomes a range operator **only if** there is an unconsumed character
immediately before it *and* a character immediately after it (and that
following character isn't `]`). If either side is missing, `-` is read as an
ordinary literal character instead.

- `[-az]` — no character available before the `-` (it's the first thing in the
  class) → literal: `{-, a, z}`.
- `[az-]` — no character available after the `-` (`]` follows) → literal:
  `{a, z, -}`.
- `[ab-z]` — `b` and `z` are both available → range: `{a} ∪ {b..z}`.

**Once a character is consumed as the end of a range, it cannot be reused** as
the start of another. Parsing is one greedy left-to-right pass with no
backtracking — see the `[a-d-z]` trace below.

### `]` — closer

Closes the class. There's no PCRE-style quirk here (a leading `]` being
read as a literal instead of closing an empty class) — deliberately skipped.
So `]` always closes, even as the very first character, which raises the
"is an empty class legal" question below.

## Range validity

`CHAR '-' CHAR` requires `start <= end` (compared by Unicode scalar value,
same order `char` already uses everywhere else in this engine). `start > end`
is a **parse error**, not a silently-empty range and not an auto-swap.

- `[a-z]` — valid, 26 members.
- `[z-a]` — **error**. Nothing in the pattern says which direction was
  intended; refusing it surfaces the typo instead of guessing.
- `[za-az]` — valid. `z` and `z` (later) are standalone items; only the
  middle `a-a` is ever checked as a range, and `a <= a` holds. The check is
  local to each item, never global across the class.

## Worked edge cases

### Multiple ranges: `[a-dm-z]`

Perfectly fine — a class is a *sequence* of items. `a-d` closes when the peek
after `d` isn't `-`; a fresh item then starts at `m`, and `m-z` forms the
second range. Result: `{a..d} ∪ {m..z}`.

### A `-` right after a finished range: `[a-d-z]`

- `a-d` consumes and **spends** `a`, `-`, and `d`.
- The next `-` starts a **fresh** item. It's read as an ordinary value (there
  is no rule against `-` being the character a fresh item reads), and since
  the very next char (`z`) isn't itself a further `-`, this item is just the
  literal `-`.
- `z` starts its own fresh item, standalone.

Result: `{a, b, c, d} ∪ {-} ∪ {z}`. **Not** `d-z` — `d` was already spent by
the first range and can't be reused as a second range's start.

### Parens do nothing special: `[(ab)-(cd)]`

Groups don't exist inside classes in any regex flavor — `class_item` has no
grammar slot for a sub-expression the way `parse_atom`'s `(` case does. So
`(` and `)` are just ordinary characters here. Walking it: `(`, `a`, `b` are
three standalone items, then `)` is followed by `-` followed by `(`, so it
reads as the range `)-(``. Since `)` (0x29) > `(` (0x28), that's an
**inverted range — parse error**. Not because groups were used, but because
the flat class grammar happens to compose into an inverted range here.

### Ranges are Unicode-scalar-value order, not just ASCII

`char` already compares by scalar value everywhere in this engine (Lesson 1's
"alphabet = `char`" decision). Ranges inherit that for free — `['a'-'z']`-style
reasoning generalizes to any part of the Unicode range, not just ASCII, and
"inverted" is decided the same way regardless of which characters are
involved.

## Semantics of negation

`[^...]` matches any character **not** in the set — the complement, over the
*whole alphabet* (every `char`), not just some ASCII subset.

Consistency with `.`: this engine's `.` has **no hidden exclusions** — it
matches literally everything, newline included (see `dot_has_no_special_case_for_whitespace_or_newline`
in the test suite). A negated class should follow the same rule for the same
reason: no silent carve-outs. `[^a]` matches `\n`, `" "`, anything at all
except the literal `a`.

## Examples

| Pattern     | Input   | Matches? | Why |
|---|---|---|---|
| `[abc]`     | `"a"`   | yes | `a` is a member |
| `[abc]`     | `"d"`   | no  | not in `{a,b,c}` |
| `[a-z]`     | `"m"`   | yes | in range |
| `[a-z]`     | `"M"`   | no  | uppercase is a different scalar value, out of range |
| `[^a-z]`    | `"M"`   | yes | negated: `M` is not in `a-z` |
| `[^a-z]`    | `"m"`   | no  | `m` *is* in `a-z`, so negation excludes it |
| `[ab-df]`   | `"e"`   | no  | set is `{a, b, c, d, f}` — `e` is the gap |
| `[a^bc]`    | `"^"`   | yes | `^` not first, so it's a literal member |
| `[-az]`     | `"-"`   | yes | leading `-` is literal |
| `[a-d-z]`   | `"-"`   | yes | see trace above — `-` is its own member here |
| `[a-d-z]`   | `"e"`   | no  | `e` is in none of `{a..d}`, `{-}`, `{z}` |
| `[a-z]+`    | `"cab"` | full match | classes compose with quantifiers like any atom |
| `[abc]?d`   | `"d"`   | yes | the class is optional, same as any atom under `?` |

## Still open (your calls, not yet decided)

- **Is `[]` (empty class) legal?**
  - If `class_item+` (at least one required): `[]` is a parse error — the
    class body can't be empty.
  - If `class_item*` (zero allowed): `[]` is a legal, always-failing atom —
    it can never advance, for any input. This is the *local*, class-scoped
    version of Kleene's `∅`, which the top-level `Ast` deliberately can't
    express (Lesson 2) — a class *can* express it naturally, if you allow it.
  - Neat consequence if you allow it: `[^]` (negated empty class) would mean
    "not in the empty set" — i.e. *every* character. That's the exact same
    language as `.`, reached by a completely different route. Not a reason
    you have to allow empty classes, just a fact worth knowing before you
    decide.
- **Instruction representation** — covered in chat, not repeated here:
  dedicated `ConsumeClass`-style instruction, item list lives in a side table
  indexed by `usize` (not inline in the instruction) to keep `ValidInstruction`
  `Copy`.
