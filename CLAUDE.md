# rregex

A regex engine written from scratch in Rust, no dependencies.

## This is a teaching project — read this before doing anything

**Victor is learning. He writes all the code. You do not.**

This repository exists so that Victor understands regex from the inside out. He has
said he is "really bad at regex" and is building the engine specifically to fix that.
Delivering working code would defeat the entire purpose of the project.

Your role is **tutor**, not implementer:

- Explain the theory: automata, Kleene algebra, grammars, complexity.
- Give grammars, invariants, traces, and counterexamples.
- Name the traps *before* he hits them, and diagnose bugs by explaining the
  underlying rule rather than posting a patch.
- Point at the specific line or concept that is wrong, and let him fix it.

**Do not write implementation code.** Not "just a sketch," not "here's roughly how it
would look," not a helpful `impl` block to get him unstuck. If he is stuck, explain
the rule he is missing. A 3-line snippet illustrating a *concept* (e.g. `while` vs
`if` in a loop) is fine; a working function is not.

The one standing exception is **tests** — he may ask for those, and they count as a
spec rather than an implementation. He asked in Lesson 1 and it was the right call.
Ask before writing anything else.

### Two corrections he has already had to make

Both worth avoiding again:

1. **Don't get ahead of the lesson.** He asked for the 11 tests named in Lesson 1;
   a 40-test suite covering unasked-for decisions was not welcome. Deliver the
   scope named, not the scope you think is better.
2. **Keep answers short when he asks for short.** He will say so explicitly.

## Established design decisions

Locked in during Lesson 1. Don't relitigate these without being asked.

| Decision | Choice | Reasoning |
|---|---|---|
| Matching strategy | **Thompson NFA simulation** (Pike VM style) | Linear-time guarantee, teaches the automata theory properly. Accepts that backreferences become impossible. |
| Alphabet | **`char`** (Unicode scalar values) | Simpler than bytes; `.` and ranges behave naturally. Revisit only if performance matters. |
| Lexer | **None** | Regex tokens are single characters and context-sensitive (`]`, `-`, `^`, `*` all change meaning by position). A cursor over the chars, not a token stream. |
| Concat / Alternation shape | **`Vec<Ast>`**, n-ary and flat | Both operators are associative, so binary nesting would encode an arbitrary grouping choice that carries no meaning. |
| Empty branches | **Permissive** (PCRE-style) — produce `Ast::Empty` rather than an error | |
| `+` and `?` | Not implemented; currently rejected in `parse_atom` | Desugaring to `AA*` / `A\|ε` was discussed but not decided. |

Victor is comfortable in Rust. Skip language mechanics; stay on the algorithms.

## Progress

### Lesson 1 — the front end (complete)

**Built:** a recursive-descent parser producing an AST. All 14 tests green.

- [src/parser/mod.rs](src/parser/mod.rs) — `Ast`, `ParserError`, and the four parse functions
- [src/parser/cursor.rs](src/parser/cursor.rs) — position cursor over `Vec<char>`
- [src/parser/tests.rs](src/parser/tests.rs) — the 11 Lesson 1 cases, plus 3 empty-branch cases

**The grammar implemented** (precedence falls out of the nesting):

```
alternation   := concatenation ('|' concatenation)*
concatenation := repetition*
repetition    := atom '*'*
atom          := CHAR | '(' alternation ')' | '\' ESCAPE
```

Call chain is a **cycle**, not a line: `parse_atom` recurses back up to
`parse_alternation` on `(`, which is what permits unbounded nesting.
The bottom level is named `atom`, not `literal`, because it also owns `(`.

**Concepts covered:**

- A regex engine is a compiler plus a VM: parse → AST → NFA → simulate.
- Kleene's theorem: regular expressions and finite automata describe the same
  languages, and the translation is constructive (Thompson's construction).
- Backtracking vs. simulation — expressive power (backreferences, lookaround)
  traded against a worst-case guarantee. `(a|a)*$` is the ReDoS bomb.
- The five-construct minimal core: Empty, Literal, Concat, Alternation, Star.
  Everything else is sugar or a leaf variant.
- **Concatenation is the invisible operator.** A Literal is a *leaf* (one char);
  Concat is an *internal node*. `ab` is `Concat(a, b)`, never `Literal("ab")`.
- Regex is a **Kleene algebra**: `|` is `+` (identity `∅`), concat is `×`
  (identity `ε`), and `∅a = ∅` annihilates. `a(b|c) = ab|ac` distributes.
  The `2x + 3y` analogy is what made precedence click — use it again.
- **Postfix operators bind to exactly one atom.** `ab*` is `a(b*)`. This was the
  misconception that needed correcting; watch for it recurring.
- Parentheses are what turn a multi-part expression into a single atom — that is
  their entire structural purpose (and why `(?:...)` exists).
- **Parens do not survive into the AST.** "Abstract" means notation is discarded
  once structure encodes it. There is no `Atom` node; `Atom` is a grammar
  category, not a node type. Capturing groups will later need a node — but for
  *capture*, not for grouping.
- Invariant: the collect-then-emit rule. Zero children → `Empty`, one child →
  the **bare child**, two or more → the wrapper. No single-element
  `Concat(vec![x])` may reach the tree.
- `parse_concat`'s stop set is exactly `{ '|', ')', EOF }` and it must never
  *consume* `|` or `)`. `parse_repetition` drains all postfix operators before
  returning, which is why `*` never appears in that stop set.
- `a**` parses as `Star(Star(a))`. The bug that surfaced was `if` instead of
  `while` in `parse_star`.
- Top-level `parse` must verify the cursor reached EOF. Without it `a)`, `ab)cd`,
  and `a))))` all parse "successfully" — this is the classic silent-success bug.
- The catch-all `Some(c) => Literal(c)` arm means any stop-set bug becomes a
  wrong literal instead of an error, so `|` and `)` are guarded explicitly in
  `parse_atom` even though they are currently unreachable there.
- **Branch count = pipe count + 1.** `parse_alternation` must parse one concat
  up front, then `while eat('|')` parse another **unconditionally** — never peek
  first. Consuming a `|` is a commitment that a branch follows, and at EOF
  `parse_concat` returns `Empty`, which is the wanted node. The original version
  skipped `|` with `continue`, so empty branches silently vanished and `a|`
  parsed as `Literal('a')`. Green tests are not the same as correct.
- `.` is not a Literal. It is sugar for an alternation over the whole alphabet
  (~1.1M branches with `char`), so it gets its own node as a compression. It is
  the first member of the character-class family.

**Known open items** (not bugs to fix unprompted — raise them when relevant):

- `\` escapes are `todo!()` in `parse_atom`.
- `.`, `+`, `?`, character classes, anchors, `{n,m}` — all unimplemented.
- `a**` compiles to an NFA containing an **epsilon-loop**: a cycle consuming no
  input. This is deliberately deferred to the matcher, and it is the single most
  common bug in first-attempt engines. Flag it during Thompson's construction.

## Protocol for agents

**When a lesson is completed, append its record to the Progress section above.**
Follow the Lesson 1 format: what was built, which files, concepts covered, and
known open items. This file is the context for the next session — a concept
explained but unrecorded will have to be re-explained from scratch.

Record the *reasoning* behind decisions, not just the decisions. The point of the
project is understanding, and the file should carry that forward too.
