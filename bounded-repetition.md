# Bounded repetition — reference

Everything about `{n,m}` notation, gathered in one place, starting from what
it even means. This is a spec to implement against, not implementation itself
— no code here.

## What it means

`{n,m}` after an atom means "repeat this atom at least `n` times and at most
`m` times." Three shapes, same idea:

- `a{3}` — exactly 3. Matches `"aaa"`. Does **not** match `"aa"` (too few) or
  `"aaaa"` (too many, at least not as a *full* match — `find` would still
  report a match ending after the third `a`, same as any other pattern that
  doesn't have to consume the whole input).
- `a{2,4}` — between 2 and 4, inclusive. Matches `"aa"`, `"aaa"`, `"aaaa"`.
  Does not match `"a"` (too few) as a full match.
- `a{2,}` — at least 2, no upper limit. Matches `"aa"`, `"aaa"`, `"aaaaaaaa"`,
  anything with 2 or more.

Enumerating the language is the fastest way to build intuition here (same
trick that untangled `a|b` vs `ab` back in Lesson 3): `a{2,3}` describes
exactly the set `{"aa", "aaa"}` — two strings, nothing else.

## It's not new — it's the general case of what you already built

`*`, `+`, and `?` are three specific points on the same number line `{n,m}`
covers:

| Shorthand | Equivalent `{n,m}` | Meaning |
| --- | --- | --- |
| `a*` | `a{0,}` | zero or more |
| `a+` | `a{1,}` | one or more |
| `a?` | `a{0,1}` | zero or one |

So this isn't a fourth, unrelated repetition construct — it's the same idea
as `*`/`+`/`?`, parameterized. Same postfix-binds-to-one-atom rule applies
unchanged: `ab{2,3}` is `a(b{2,3})`, not `(ab){2,3}`.

## The forms, and which ones are actually standard

- `{n}` — exact count.
- `{n,m}` — bounded range, `n <= m`.
- `{n,}` — at least `n`, unbounded above.
- `{,m}` — "at most `m`" (i.e. `{0,m}`). **Not universally supported** —
  some engines accept it, others (PCRE among them) treat an unrecognized
  brace form as a literal string instead of a quantifier. Worth deciding
  deliberately whether to support it at all, same category of choice as
  `class_item+` vs `class_item*` for empty classes.

## Validity: what happens when `n > m`?

`a{5,3}` asks for "at least 5, at most 3" — no count satisfies both, so the
set of valid repetition counts is empty. This is exactly the ∅ conversation
from the character-class lesson, resurfacing in a new spot: is `a{5,3}` a
parse error (consistent with how `[z-a]`, a backward range, is currently
rejected rather than silently treated as contributing nothing), or is it
legal syntax that compiles to something that can never match — the same
move `[]` already makes? Worth deciding the same way, not by accident.

One case worth tracing on its own: `a{0}`. Zero required, zero allowed —
every count in `[0, 0]` is satisfied by *not matching `a` at all*. That's ε
(the empty string), not ∅ — same distinction the very first ∅ discussion
drew out: "repeat zero times" is always satisfiable (by doing nothing), so
it can never be the empty language, no matter how it's spelled.

## The genuinely new parsing problem

Every atom and operator so far is decided by looking at one character, or at
most one character of lookahead (the lazy `'?'` suffix). `{n,m}` is
different in kind: deciding whether `{` even **is** a quantifier requires
reading and validating an entire sub-grammar — digits, an optional comma,
optional more digits, a closing `}` — before you know whether to treat it as
one.

```
count      := DIGIT+
bound      := '{' count ('}' | ',' count? '}')
```

And the trap that comes with it: **not everything that looks like `{...}`
is a quantifier.** PCRE/Perl (the convention this project has leaned on
throughout — see the empty-branch and class-negation decisions) treat a
malformed brace expression as an ordinary literal `{`, not a parse error.
`a{2,1x}` — invalid count — falls back to matching the literal characters
`{`, `2`, `,`, `1`, `x`, `}`. `a{` with nothing sensible following does the
same. This means the parser has to be able to **attempt** the bounded-count
grammar and **back out** to "just a literal `{`" if it doesn't parse clean —
the first place in this grammar that needs a tentative parse-and-rollback
instead of a decision made from a fixed lookahead window. Worth deciding
whether to adopt that convention or simply make a malformed `{...}` a hard
parse error — a real fork, not a foregone conclusion, and different from
every disambiguation so far (`^`'s two meanings, the lazy-`?` collision) in
that those were resolved by *position* or *one-character lookahead*, not by
running a whole grammar and possibly discarding the attempt.

## The genuinely new compile-time problem

Every gadget you've built so far — `*`, `+`, `?`, and their lazy twins —
calls `compile_fragment` on the child **exactly once**, then wires a
`Split`/`Jump` around that one copy. Repetition comes from the NFA's own
ε-loop, not from duplicated instructions — that's *why* Thompson's
construction stays linear in AST size regardless of how large a `Star`'s
match count ends up being at run time.

`{n,m}` breaks that invariant. An NFA has no counter register — a state is
just an index, nothing travels with it that remembers "how many times have I
looped" (this is the same "state = index, nothing more" fact from Lesson 2).
So there's no way to compile "loop between `n` and `m` times" as a single
small cycle the way `*` does. The standard move (what RE2 and similar
engines actually do) is to **unroll**: compile the child fragment `n` times
in a row for the mandatory part, then up to `m - n` more times wrapped in
`?` for the optional part (or a trailing `*` when there's no upper bound —
`a{2,}` is exactly `aa` followed by `a*`). That means calling
`compile_fragment` on the same AST node multiple times — a real, new fact
about this gadget, not a style choice.

That has a real cost, and it compounds exactly the way the original
quantifier-desugaring decision worried about: nested bounded repetition,
`(a{2,3}){4,5}`, unrolls the inner 3 copies once for *each* of up to 5 outer
copies — up to 15 copies of the base fragment from six characters of
pattern. This is why real engines cap the maximum allowed count (PCRE's
default limit is in the tens of thousands, and it separately limits how
large a nested product is allowed to get) — not a nicety, a necessary guard
against a small pattern compiling to an enormous program. Worth deciding
whether this project wants a cap now or is comfortable letting a pathological
pattern produce a pathological program, consistent with "the parser refuses
nothing, illegal *languages* aren't the parser's problem" — except program
size, unlike language legality, is a resource the compiler actually has to
account for.

## Composing with laziness

The existing grammar already has a slot for this: `QUANT := ('*' | '+' |
'?') '?'?` just needs `{n,m}` added as a fourth alternative under the same
rule — `a{2,4}?` prefers as few repetitions as possible (down to the
mandatory 2), the same "prefer skipping" flip every other lazy variant
already makes on its `Split`.

## Examples

| Pattern    | Input     | Matches full string? | Why                                                        |
| ---------- | --------- | --------------------- | ----------------------------------------------------------- |
| `a{3}`     | `"aaa"`   | yes                   | exactly 3                                                    |
| `a{3}`     | `"aa"`    | no                    | one short                                                    |
| `a{3}`     | `"aaaa"`  | no                    | one over, as a *full* match — `find` still reports 3 consumed |
| `a{2,4}`   | `"aaa"`   | yes                   | 3 is within `[2,4]`                                          |
| `a{2,4}`   | `"a"`     | no                    | below the minimum                                            |
| `a{2,}`    | `"aaaaaa"`| yes                   | no upper bound                                               |
| `a{0}`     | `""`      | yes                   | zero repetitions is ε, always satisfiable                    |
| `a{0}`     | `"a"`     | no                    | zero means zero, not "any number including zero and more"    |

One case deliberately left out of the table: `a{2,3}?` against `"aaa"`. If the
mandatory-then-lazy-optional unrolling is what you build, the optional third
`a` is reached through a `Split` whose higher-priority arm points straight at
`Match` — structurally the same shape as `a|ab` against `"ab"`, the pattern
behind this project's one open, deliberately-unfixed bug (`full_match` derived
from `find`'s leftmost-first cut can hide a longer match that a lower-priority
thread would have reached). Don't assume this case is fine without tracing it
against that same bug.
