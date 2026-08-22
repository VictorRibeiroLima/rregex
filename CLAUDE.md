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

Locked in during Lessons 1-2. Don't relitigate these without being asked.

| Decision | Choice | Reasoning |
|---|---|---|
| Matching strategy | **Thompson NFA simulation** (Pike VM style) | Linear-time guarantee, teaches the automata theory properly. Accepts that backreferences become impossible. |
| Alphabet | **`char`** (Unicode scalar values) | Simpler than bytes; `.` and ranges behave naturally. Revisit only if performance matters. |
| Lexer | **None** | Regex tokens are single characters and context-sensitive (`]`, `-`, `^`, `*` all change meaning by position). A cursor over the chars, not a token stream. |
| Concat / Alternation shape | **Binary** — `Concat(Box<Ast>, Box<Ast>)`, `Alternation(Box<Ast>, Box<Ast>)`. *Changed in Lesson 2; was n-ary `Vec<Ast>`.* | The n-ary argument (associativity makes grouping meaningless) is still true of the *language*, but the NFA needs a bounded fan-out: `Split` holds exactly two targets. Binary nodes put the fold in the parser, so `compile` maps one node to one instruction. Cost: right-recursion makes parser stack depth O(input length). |
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

### Lesson 2 — Thompson's construction (complete)

**Built:** a compiler from `Ast` to a flat NFA program. All 24 tests green
(16 parser, 8 machine).

- [src/machine/mod.rs](src/machine/mod.rs) — `Instruction`, `Program`,
  `Fragment`, `Machine::new`, and the five `compile_*` functions plus their tests

**The AST changed.** `Concat` and `Alternation` became binary (see the decisions
table). Victor made the change himself once he saw that an n-ary `Alternation`
would compile to an n-way branch, which `Split` cannot hold. Two tests were added
to pin the now-observable associativity (`abc`, `a|b|c` — both right-nested,
following the right-recursive parser). Nothing in the *language* distinguishes
the nestings; the tests exist because an untested arbitrary choice silently
changes.

**The representation, and why:**

```
State       = usize          an index; identifies a slot
Instruction = the contents of a slot: what a finger standing here may do
Program     = Vec<Instruction>
Fragment    = { start: State, exit: State }
```

- **States are indices, not pointers.** Every `Star` creates a cycle, so a real
  node graph means `Rc<RefCell<_>>`: reference cycles that never drop, runtime
  borrows, and a fight with the borrow checker to look at both branches of a
  split. Indices sidestep all three, and the whole program is one allocation.
  This is what RE2, Go's `regexp`, and Rust's `regex` do.
- **Four instructions, because fan-out is ≤ 2.** Only `Alternation` and `Star`
  branch, and each creates exactly a 2-way choice, so an edge can be a fixed
  field rather than a list. `Consume(char, State)`, `Jump(State)`,
  `Split(State, State)`, `Match` — plus `Hole`, see below.
- **State and instruction are the same object from two directions.** State is the
  automata view (a dot); instruction is the VM view (a line of a program). Same
  slot. This is why the matcher will be a loop with a program counter.
- **The one line that partitions the enum:** `Consume` advances the input
  position, `Jump`/`Split`/`Match` do not. Lesson 3 branches on exactly this.

**Compile as push-and-patch:**

Every fragment's `exit` is a **hole** — a real slot, pushed so it occupies an
index, whose contents the *parent* overwrites later. Compilation is nothing but
"push states, fill holes." `Instruction::Hole` is an explicit variant so an
unfilled one is loud rather than silently pointing at slot 0.

| Node | pushes | fills |
|---|---|---|
| `Empty` | 1 (a lone `Hole`; `start == exit`) | — |
| `Literal(c)` | 2 (`Consume` + `Hole`) | — |
| `Concat(A,B)` | **0** | A's hole ← `Jump(B.start)` |
| `Alternation(A,B)` | 2 (`Split` + `Hole`) | both children's holes ← `Jump(exit)` |
| `Star(A)` | 2 (`Split` + `Hole`) | A's hole ← `Jump(split)` |

`Machine::new` fills the root's hole with `Match` — the one hole nobody else owns.

**Concepts covered:**

- An NFA is a directed graph plus a walking rule. Transitions are a *relation*,
  not a function: multiple targets per char, plus ε-edges taken free.
  Acceptance is existential — *some* path consumes the whole string.
- **ε-transitions buy composability, not power.** The invariant *exactly one
  start, exactly one accept; nothing enters the start, nothing leaves the accept*
  makes a fragment a black box with one plug and one socket, so gluing is a
  single ε-edge with no case analysis on what's inside. Without it, a fragment
  carries a *set* of exits and every gadget grows loop-and-union bookkeeping.
  One sentence: **ε-edges convert a set of exits into a single exit.**
- The price is state count and ε-chasing; the payoff is a construction that is
  linear in AST nodes and correct by a two-line induction. Linearity is what
  makes the Lesson 3 guarantee (O(states × input)) worth anything.
- **Failure is the absence of an arrow.** The transition relation is partial —
  no dead state, no trap. In the matcher, a finger with no matching arrow is
  dropped from the set.
- **The machine answers exactly one question:** does *this whole string* belong
  to the language? "`aaa` matches `a|b` three times" is a *search* concern — a
  loop wrapped around the machine — as are unanchored matching and submatches.
  Keep that boundary sharp.
- Compile-time sees no input. `compile` never touches a string.
- Recursion is **post-order**: children finish before parents, siblings left to
  right. Sibling order is genuinely free (it only changes numbering), but going
  left-to-right is what the `match` arms already read as.
- **The start state is usually not index 0.** Indices follow creation order, and
  branch nodes push their `Split` only after their children exist. `(a|b)*c`
  starts at 6.
- **Greedy vs lazy is the order of a `Split`'s two targets.** `Split(body, exit)`
  prefers the body, which is what makes `*` greedy once Lesson 3 makes the first
  branch preferred. `*?` will be that swap.
- ε is **not a symbol**. `Consume('\0', _)` is not `Empty`: it would make `""`
  fail to match and `"\0"` start matching, because in a `char` alphabet every
  `char` occurs. `Empty` is the language `{""}`; `∅` is `{}` and the AST
  deliberately cannot express it. (Kleene: ε is concat's identity, `∅` is
  alternation's identity and concat's annihilator.)

**Bugs that surfaced (all self-inflicted index arithmetic, all instructive):**

- Returning an exit index for a slot that was never pushed. The next fragment
  lands in that slot and the parent's patch **overwrites a real instruction**,
  producing a silent ε self-loop instead of a crash. Rule: *a hole must occupy a
  slot*; a fragment's `exit` must exist the moment the fragment returns.
- `program.len()` after a push is the index of the *next* slot, not the one just
  pushed. Same bug, one slot over. Rule: *the index of a slot is `len()` at the
  moment you push it* — capture it before the push.
- Reaching inside a child fragment when wiring. His first `a|b` merged the two
  literals onto shared states, and his first `Star` added an edge from the
  child's start straight to the child's accept. The second one creates a **pure
  ε-cycle in a plain `(a|b)*`** — correct language, broken machine. Rule: a
  gadget may use a child's `start` and `exit` as *two integers* and nothing else.
- Wiring the outside of a `Concat` and leaving the seam disconnected, giving a
  two-component graph that matches nothing. `Concat` pushes zero states; its
  entire product is the seam edge.

**Teaching notes for next time:**

- He inverted sibling order twice ("compile the last child first"). Worth
  re-checking if it recurs.
- He reads a wiring line like `(5) ⇢ (6)` as a *description* of what the child
  already does, rather than as a new edge being added. The fix that landed:
  "start and exit are two integers you carry, not a promise of free travel."
- Working the gadgets as hand-drawn diagrams **before** any Rust was what made it
  stick. He then predicted each program slot-by-slot in a comment before writing
  the arm, and every prediction was right. Keep this workflow.
- Asserting the *whole* program slot by slot is the right test shape for a
  compiler — it pins the numbering, which is what every index bug corrupts.

**Known open items** (not bugs to fix unprompted — raise them when relevant):

- `\` escapes are `todo!()` in `parse_atom`.
- `.`, `+`, `?`, character classes, anchors, `{n,m}` — all unimplemented.
- **The ε-loop is now built and visible, and it is Lesson 3's problem.** `a**`
  compiles to slots `4: Split(2,5)`, `2: Split(0,3)`, `3: Jump(4)` — a cycle in
  which nothing advances the input, so a finger can walk `4 → 2 → 3 → 4` forever.
  The compiler is right to emit it; Thompson's construction is local and must not
  reject a well-formed tree. The matcher fixes it by tracking which states it has
  already added at the current input position. `stacked_stars_build_an_epsilon_loop`
  documents the exact slots.
- `Machine` has no matcher yet and its fields are private; nothing runs a string
  through a program.
- Parser stack depth is O(input length) after the switch to binary nodes; a long
  literal run will overflow. Production engines cap nesting depth (`nest_limit`).
- ε-only slots (`Jump`) are kept deliberately: they make the program match the
  hand-drawn diagrams and are removable later by a peephole pass that does not
  touch `compile`. Cox's dangling-out-pointer scheme avoids them and was
  rejected for now on debuggability grounds.

## Protocol for agents

**When a lesson is completed, append its record to the Progress section above.**
Follow the Lesson 1 format: what was built, which files, concepts covered, and
known open items. This file is the context for the next session — a concept
explained but unrecorded will have to be re-explained from scratch.

Record the *reasoning* behind decisions, not just the decisions. The point of the
project is understanding, and the file should carry that forward too.
