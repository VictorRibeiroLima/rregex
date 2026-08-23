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
| State set | **Bitset** — `Vec<bool>` indexed by state, plus a cached `matched` flag. *Added in Lesson 3.* | States are dense (`< program.len()`), so membership is one indexed load; no hashing. Set and visited-marker are the same object, so they cannot drift apart. Cost: `step` scans all `n` slots rather than only the live ones — same O(n·m) bound. |

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
- **The ε-loop is built and visible.** `a**` compiles to slots `4: Split(2,5)`,
  `2: Split(0,3)`, `3: Jump(4)` — a cycle in which nothing advances the input, so
  a finger can walk `4 → 2 → 3 → 4` forever. The compiler is right to emit it;
  Thompson's construction is local and must not reject a well-formed tree.
  `stacked_stars_build_an_epsilon_loop` documents the exact slots. *Resolved in
  Lesson 3* by the already-seen check in `follow`.
- `Machine` has no matcher yet. *Resolved in Lesson 3* — `Regex` runs strings
  through the program via `Machine::start()` and `Machine::program()`.
- Parser stack depth is O(input length) after the switch to binary nodes; a long
  literal run will overflow. Production engines cap nesting depth (`nest_limit`).
- ε-only slots (`Jump`) are kept deliberately: they make the program match the
  hand-drawn diagrams and are removable later by a peephole pass that does not
  touch `compile`. Cox's dangling-out-pointer scheme avoids them and was
  rejected for now on debuggability grounds.

### Lesson 3 — the simulator (complete)

**Built:** a Thompson NFA simulator (Pike VM shape, minus priority). All 37 tests
green (16 parser, 8 machine, 13 matcher).

- [src/regex/mod.rs](src/regex/mod.rs) — `Regex`, `SeenSet`, and the three
  functions `step`, `closure`, `follow`, plus the matcher tests
- [src/regex/error.rs](src/regex/error.rs) — `RegexError`, with
  `From<ParserError>`
- [src/machine/program.rs](src/machine/program.rs) — `Instruction`,
  `ValidInstruction`, `Program`, `ValidProgram` (moved out of `machine/mod.rs`)

**`ValidProgram` — the hole assertion became a type.** Lesson 2 ended with a
runtime `assert!(!program.contains(&Hole))`. `ValidProgram::new` now consumes a
`Program` and returns a `Vec<ValidInstruction>` — the same enum minus `Hole`. The
matcher therefore `match`es four arms, not five, and there is no unreachable case
to write. Illegal states made unrepresentable, at exactly the boundary where the
program stops being under construction. Victor's own move; keep it.

**The recurrence, which is the whole lesson:**

```
S_0     = closure({ start })
S_{i+1} = closure( step( S_i, input[i] ) )
accept iff Match ∈ S_final
```

`closure` appears in both lines, `step` only in the second — that asymmetry is
why they are two functions. `closure` takes no character; a character is only
needed to decide which `Consume`s survive, and at position 0 nothing has been
consumed.

**The representation:**

```
SeenSet = { seen: Vec<bool> indexed by state, matched: bool }
```

- The set and the visited-marker are **the same object**. `seen[s]` answers both
  "is s in the set?" and "have I already added s during this closure?" — they can
  never disagree.
- `matched` is cached when `follow` visits a `Match`, so the final verdict is O(1)
  instead of a scan. It is `false` at every new position because `step` builds a
  fresh `SeenSet`; that must stay true if the allocation is ever reused.

**Division of labour:**

| fn | reads input? | job |
|---|---|---|
| `step` | yes | for each live `Consume(c, t)` with `c` == the char, contribute **`t`**. Everything else dies. Returns a fresh, *unclosed* set. |
| `closure` | no | seeding loop: call `follow` on every state already in the set |
| `follow` | no | the walk: `Jump`/`Split` → check-mark-recurse per target; `Consume`/`Match` → wall |

**Concepts covered:**

- **The set is the whole trick.** A `Split` does not create two sets; it puts two
  states into the one set. Two sets would be two independent machines — that is
  backtracking, and it is exponential. There is exactly one set alive at a time
  (plus the `next` being filled).
- **Simulation is subset construction done lazily.** A set of NFA states *is* a
  DFA state; the simulator builds one at a time and throws it away. Same insight,
  no exponential table.
- **The invariant:** `S_i` = exactly the states reachable from `start` by some
  path spelling the first *i* characters, where "reachable" always includes free
  ε-travel. Position 0 is not a special case — it is that rule with *i* = 0,
  which is why `S_0` needs a closure even though no input has been read.
  Skipping it breaks every regex whose start state is a `Split` (`a|b` starts at
  4, `a*` at 2, `(a|b)*c` at 6) and makes `a*` reject `""`.
- **The set is a position, not a log.** It records where fingers stand, never
  where they have been. `S_1 == S_2` for `a*` on `"aa"` is not a bug — it is a
  loop at steady state.
- **Lockstep.** One input pointer for the whole machine; every finger is always
  at the same input position. The set has no way to express otherwise and never
  needs to. Nothing ever rewinds — that is the linear-time guarantee.
- **`|` splits the pattern, not the input.** Both branches are rivals for the
  *same* character. Concatenation is the operator that divides the input between
  two sub-patterns; alternation never lengthens the language, only widens it.
- **The dedup check is termination, not validation.** Arriving at an
  already-marked state is normal, expected, and load-bearing: it is the base case
  of the walk. Every `Star` compiles to a cycle by construction, so a matcher
  that treats a revisit as an error rejects every star. Chalk-marks in a maze:
  you turn around, you do not declare the maze invalid.
- The same check does two jobs at once, and they are the same fact: it terminates
  ε-cycles, and it merges duplicate paths so the set is bounded by the number of
  **states** rather than the number of **paths**. Paths vs. states is the whole
  distance between ReDoS and O(n·m).
- Which is also exactly why **backreferences are impossible**: the merge is sound
  only because a finger's future depends on `(state, input position)` and nothing
  else. `\1` would make history matter, and two fingers on one slot would stop
  being interchangeable. The merge buys the speed and forbids the feature.
- **Failure is the absence of an arrow**, in code: a non-matching `Consume` is
  simply not copied into `next`. No dead state.
- **The empty set is absorbing.** `step({}, c) = {}` for every `c`, so an early
  `return false` is sound — an optimisation, not a correctness requirement.
- **`Match` has no outgoing edges of any kind.** Closure walls on it; `step` has
  no arm for it. A finger that lands on `Match` dies at the next character. So
  the `Match ∈ S` test is made **once**, after the loop — never inside it.
- **The machine answers membership, not search.** `ab` does not match `"abc"`;
  `a|b` does not match `"ab"`. Search is a loop *around* this machine: try each
  start offset and accept as soon as `Match` appears without requiring the input
  to be exhausted. Keep the boundary sharp.
- Order is free *today*: `step`'s result is a union of contributions, and union
  is commutative, so scan order cannot change the answer. It stops being free in
  Lesson 4.

**Bugs that surfaced:**

- `step` contributing `i` instead of `j` — parking the finger back on the
  `Consume` it just executed. Rule: *what survives is where the arrow pointed,
  not the instruction that pointed*. The unused-variable warning on `j` said so.
- `let seen_set = self.step(...)` **inside** the `for` body — a new binding
  scoped to the loop, dropped at the closing brace, so every character was
  matched against `S_0` forever. Shadowing in a loop body always does this.
- Writing `closure` as a **scan over the program** rather than a walk from a
  given set: it marked every state with an incoming edge and never mentioned
  `machine.start()`. A closure that does not read the start state cannot be
  computing reachability from it. Diagnostic: a walk needs "reached but not yet
  examined"; `for inst in program` has no such thing.
- The first `closure` also built a fresh empty `next`, dropping its own input.
  Rule: *closure only ever adds*; `closure(X) ⊇ X`. The walls it was discarding
  were the entire answer.
- `return Err(RegexError::StateLoop)` as the body of the already-seen check —
  detecting the cycle correctly and then drawing the wrong conclusion from it.
  Surfaced on `a|b|c` against `"b"`, where `follow(7)` was reached twice by two
  different routes and there was no cycle at all.
- `if seen[j1] || seen[j2]` on a `Split` — abandoning **both** arms because one
  was seen. The arms are independent; a stale `j1` says nothing about a fresh
  `j2`. (An intermediate `&&` version was correct but re-explored stale arms.)

**Teaching notes for next time:**

- The hand-traces did the work again. He wrote `S_0 … S_n` out slot by slot for
  `ab`, `abc`, `a|b`, `a*` before touching Rust, and every
  trace was mechanically correct. Keep this workflow; it is the third lesson in a
  row where it was the thing that landed.
- **The recurring misconception is anchoring, not automata.** He predicted a
  match for `ab` vs `"abc"` and for `a|b` vs `"ab"`, twice, *after* correctly
  tracing both to `false`. The mechanics were never the problem — the reading of
  the pattern was. The fix that worked: enumerate the language as a literal list
  of strings (`a|b → { "a", "b" }`) and count characters before tracing.
- He needed "why does `S_0` exist at all?" answered from the invariant, not from
  the code. Answer that landed: position 0 is the base case of the same rule, and
  `a*` on `""` is the counterexample that makes it concrete.
- He asked "how did `S_2` know the jump in `S_1` was taken?" — reading `step` as
  consulting history. Answer that landed: **the fact lives in the contents of the
  set, not in a flag.** State 2 being present *is* "the jump was followed."
  Related: he assumed the visited marker persists across positions; the
  counterexample is `a*` on `"aa"`, where state 0 must be added at every
  position.
- Direction of comparison confused him once: `step` iterates the *set* and asks
  each `Consume` about the character, rather than pushing the character through
  the program looking for a home.
- Keep the two vocabularies apart — he mixed them repeatedly. **Wall** is a
  closure word (no ε-edge to follow). **Dies** is a `step` word (no matching
  arrow, not copied forward). Mixing them is a reliable early sign that the two
  phases have blurred.

**Known open items** (not bugs to fix unprompted — raise them when relevant):


- `step` allocates a fresh `SeenSet` per character. The two lists should be
  reused across positions (`clist`/`nlist` plus a swap). The stamp/generation
  trick — store *when* a state was last added and compare against the current
  position, instead of storing a `bool` and clearing `n` of them — was explained
  and deliberately deferred until there is a benchmark that complains.
- `follow` recurses per ε-edge, so stack depth is bounded by `program.len()`.
  Same class of problem as the parser's O(input) depth.
- `closure` scans `0..len` to find its seeds; `step` scans `0..len` to find live
  states. Both are O(n) per character regardless of how few states are live. A
  sparse set (RE2, Rust's `regex`) gives iteration proportional to the live set.
- `Match` is found by `follow` visiting it, which relies on the caller having
  marked the state first. The last trace of "marking lives in two places."
- Still unimplemented: `\` escapes (`todo!()` in `parse_atom`), `.`, `+`, `?`,
  character classes, anchors, `{n,m}`.
- **No priority.** The set is unordered, so greedy vs. lazy is invisible and the
  answer is a bare `bool`. This is Lesson 4's subject.

## Protocol for agents

**When a lesson is completed, append its record to the Progress section above.**
Follow the Lesson 1 format: what was built, which files, concepts covered, and
known open items. This file is the context for the next session — a concept
explained but unrecorded will have to be re-explained from scratch.

Record the *reasoning* behind decisions, not just the decisions. The point of the
project is understanding, and the file should carry that forward too.
