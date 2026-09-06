# Anchors — reference

Everything decided (and still open) about `^` and `$`, gathered in one place.
This is a spec to implement against, not implementation itself — no code here.

## Grammar

```
atom := CHAR | '(' alternation ')' | '\' ESCAPE | '[' class ']' | '.' | '^' | '$'
```

`^` and `$` are just two more members of `atom`. That's the whole grammar
change — no new nesting, no sibling grammar the way `[...]` needed one. They
slot in exactly where `.` or a `Literal` would, which means they compose with
everything `atom` already composes with: concatenation, alternation, groups,
and (see below) quantifiers.

## What they test

Every other atom tests a **character** — is the next input character `a`, is
it in this set, is it anything at all. `^` and `$` test a **position** instead:

- `^` — "the current position is 0" (the very start of the input).
- `$` — "the current position is the end of the input" (every character has
  already been consumed — precisely, position equals the total character
  count, counted the same way `.` and classes already count: Unicode scalar
  values, not bytes).

Neither one consumes a character. A match built entirely from `^abc$` still
only advances through the input three times — once each for `a`, `b`, `c`.
`^` and `$` contribute zero width; they only ever vote yes-or-no on whether
the surrounding match is allowed to stand.

## Not positionally restricted in the pattern text

This is the one place the natural guess is wrong. `^` inside `[...]` really
is positionally restricted — it only negates as the literal first character
of a class (see `character-classes.md`). Anchors as atoms have **no**
equivalent restriction: `^` and `$` are legal wherever `atom` is legal,
anywhere in the pattern, not just at the very front or back of the whole
regex string.

- `^abc` — the usual placement, but not a special case; `^` is just the atom
  that happens to be written first here.
- `foo$|bar` — `$` sits in the *middle* of the whole pattern, attached to only
  the left branch of the alternation:

  | Input     | Matches? | Why                                                              |
  | --------- | -------- | ----------------------------------------------------------------- |
  | `"foo"`   | yes      | left branch: `foo` consumed, position is now 3, string length is 3 — `$` holds |
  | `"foobar"`| yes      | left branch fails (`$` needs position 6==6? no — after "foo", position is 3, length is 6, so `$` fails); but the right branch `bar` doesn't require `foo` at all — wait, `bar` only matches the substring "bar", and this engine's `find` is anchored at position 0, so it needs "bar" starting at index 0, which `"foobar"` doesn't have — **no match** |
  | `"bar"`   | yes      | right branch: `bar` matches at position 0, needs nothing from `$` |
  | `"foox"`  | no       | left branch: `foo` consumed, position 3, length 4, `$` fails; right branch: `"foox"` doesn't start with `bar` |

  The key point either way: `$` only constrains the branch it's actually
  written in. It cannot reach across a `|` and constrain the other side —
  same locality as any other atom inside an `Alternation`.

- `(ab$)?c` — `$` inside a group, itself under `?`. Perfectly legal; whether
  it's *satisfiable* is a separate question from whether it *parses*.

## The one real overload — and it's simpler than the lazy-`?` case

`^` already has one other job: negation, as the first character of `[...]`.
Outside a class, there's no ambiguity to resolve by lookahead the way lazy
quantifiers needed it (`a*` vs `a*?` — same characters, decided by peeking
one further char). This one is decided purely by **which grammar you're
currently parsing**: inside `class_body`, `^` is the negation flag; anywhere
`atom` is being parsed at the top level, `^` is the start anchor. The two
parsing functions never overlap, so there is no runtime lookahead here at
all — just making sure the top-level atom dispatch doesn't fall through to
"literal character" for `^` the way it currently does.

`$` has no competing meaning anywhere in the grammar; it is new syntax, full
stop.

## Composing with structure: some placements are syntactically legal but always ∅

Input position only ever increases — one character consumed moves it forward
by exactly one, and nothing ever moves it back (Lesson 3's "lockstep": one
input pointer, never rewound). Two consequences fall out of that fact alone,
with no special-casing needed:

- **A `^` written anywhere after something that consumes at least one
  character is dead.** By the time control reaches it, position is at least
  1, and it can never become 0 again. `a^b` — consume `a` (position 1), then
  assert position 0: never true. The pattern parses fine; the language it
  describes is ∅, same empty-language idea `[]` gave a notation to, just
  reached here by composition instead of by an explicit empty class.
- **A `$` written anywhere before something that still needs to consume a
  character is dead**, for the same reason in reverse: once `$` holds,
  position equals the total length, and there is nothing left for a
  subsequent literal, class, or `.` to consume. `a$b` — assert end-of-input,
  then try to consume `b`: impossible. Also ∅.

Contrast with **repeating** an anchor, which is redundant but not dead:
`^^a` asserts position 0, then (having consumed nothing) asserts position 0
again — still true, so `^^a` and `^a` describe the same language. Zero-width
means "test again right here," never "test somewhere else."

No engine rejects `a^b` or `a$b` as a parse error — they're well-formed
syntax that happens to compile into a machine that can never reach `Match`.
That's consistent with this engine's existing stance: illegal *languages*
are not the parser's problem to detect (see also: nothing rejects `(a|a)*$`
being pathological for a backtracker — this engine just doesn't have that
problem).

## Composing with quantifiers

Still open, worth deciding deliberately rather than by accident (same
category of decision as `class_item+` vs `class_item*`): should `^`/`$` be
subject to `parse_repetition`'s postfix operators at all, or refused there?

If they are treated as ordinary atoms (simplest, most consistent with the
grammar above), quantifying one is legal but mostly redundant, for the same
"position only increases" reasoning as above — repeating a zero-width test
without consuming anything in between never changes the answer. `^*`, `^?`,
`^+` are all equivalent to plain `^`. The one case actually worth tracing:
`(a$)+` on `"a"` vs `"aa"` — one iteration consumes `a` (position 1) and
`$` must hold, so length must be exactly 1. A second iteration would need to
consume another `a` and *still* end exactly at the new end of input, which
only works if that second `a` is also the last character. In effect,
`(a$)+` can only ever match a single trailing `a` — the `+` doesn't buy
anything a plain `a$` didn't already give you. Not a bug to fix, just a
consequence of what `$` means, worth predicting before writing a test for it.

## No multi-line mode

`^` and `$` test the start/end of the **whole input string**, not line
boundaries. This engine has no notion of "start/end of line" and isn't
adding one here — consistent with Lesson 3's framing that the machine
answers exactly one question, "does this whole string belong to the
language." Some regex flavors offer a multi-line flag that redefines `^`/`$`
per line; that would be a distinct, later feature, not something these two
atoms do by default.

## Honest note on what `^` currently buys you

`find` is already anchored at position 0 — there is no unanchored search yet
(open item: "Search, spans, captures"). That means `^`, as notation, is
currently a no-op: whatever it would assert is already guaranteed by how
`find` works today. It's still worth implementing now, for two reasons: the
grammar shouldn't hard-code today's engine limitation into what's legal to
write, and the moment unanchored search exists, `^` stops being vacuous and
starts being the only way to force a match back to the true start. `$` has
no such caveat — it constrains something `find` does not already guarantee,
today, regardless of search mode.

## Examples

| Pattern     | Input      | Matches? | Why                                                              |
| ----------- | ---------- | -------- | ----------------------------------------------------------------- |
| `ab$`       | `"ab"`     | yes      | position after `ab` is 2, length is 2                             |
| `ab$`       | `"abc"`    | no       | position after `ab` is 2, length is 3 — `$` fails                  |
| `^ab`       | `"ab"`     | yes      | `find` already starts at position 0, so `^` holds trivially today  |
| `^ab`       | `"xab"`    | no       | fails regardless — `find` never even tries starting at position 1 |
| `^ab$`      | `"ab"`     | yes      | both hold: start is 0, end after `ab` is the full length          |
| `^ab$`      | `"abx"`    | no       | `$` fails — one character left over                                |
| `a^b`       | anything   | no       | ∅ by composition — `^` can never hold after consuming `a`          |
| `a$b`       | anything   | no       | ∅ by composition — nothing is left for `b` once `$` holds          |
| `foo$\|bar` | `"foo"`    | yes      | left branch, `$` holds at the true end                             |
| `foo$\|bar` | `"bar"`    | yes      | right branch, no `$` constraint there                              |
| `foo$\|bar` | `"foobar"` | no       | left branch's `$` fails; right branch needs `bar` at position 0    |
| `^^a`       | `"a"`      | yes      | repeating `^` re-tests the same true position — redundant, not dead |
