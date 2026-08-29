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
| Thread ordering | **Ordered list beside the bitset** — `traversed: Vec<State>`, appended in `follow`'s DFS order; rank *is* the index. *Added in Lesson 4; replaced the `matched` flag.* | Leftmost-first (Perl) semantics need the backtracker's search order, and the bitset has none. The bitset keeps O(1) dedup; a `contains()` scan would make closure quadratic. Bonus: `step` now iterates the live list instead of scanning `0..len`. |
| Match semantics | **Leftmost-first**, not POSIX leftmost-longest | It is what every mainstream engine does and what intuition expects. `Split(body, exit)` order is greediness; the swap is `*?`. |

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
  answer is a bare `bool`. This is Lesson 4's subject. *Resolved in Lesson 4.*

### Lesson 4 — priority (complete)

**Built:** a priority-ordered simulation — the Pike VM shape, minus captures.
`find` returns an end offset instead of a `bool`. All 45 tests green (16 parser,
8 machine, 8 find, 13 full_match).

- [src/regex/mod.rs](src/regex/mod.rs) — `SeenSet` gains `traversed`; `find`,
  `full_match`, `is_match`
- [src/regex/tests/find.rs](src/regex/tests/find.rs),
  [src/regex/tests/full_match.rs](src/regex/tests/full_match.rs) — the matcher
  tests, split by which question they ask

**Why the answer had to stop being a `bool`.** Acceptance is existential, so
`bool` throws away *which* accepting path was found. The moment the answer
carries data — an offset, later a span or a capture — the paths disagree and a
tie-breaking rule is required. `a*` on `"aaa"` has four accepting walks; `a|ab`
and `ab|a` describe the same language and must give different answers.

**The two schools.** POSIX is leftmost-**longest**, defined declaratively.
Perl/PCRE/Python/JS/Rust are leftmost-**first**, defined *operationally* by a
backtracker's search order: leftmost start, first alternative before second, loop
before exit. This engine implements leftmost-first, which is what "greedy" means
— not "longest", but "the loop branch is attempted first".

**The trick, which is the whole lesson.** Perl semantics are defined by a DFS
this engine refuses to perform. You recover the backtracker's *answer* without
the backtracker by making the set an **ordered list**, appended in the order
`follow`'s recursion visits states. Then the Lesson 3 dedup check becomes the
tie-breaker for free: first arrival wins, and first arrival is the
highest-priority path. The check that bought termination and linear time now also
buys Perl semantics.

> A backtracker never **creates** the losing thread. The Pike VM creates it, runs
> it, and **cuts** it. That is why the work per character stays bounded.

**The representation:**

```
SeenSet = { seen: Vec<bool>, traversed: Vec<State> }
```

- `seen` — unchanged job: O(1) membership, cycle termination, path merging.
- `traversed` — the ordered list. **Rank is the index**; nothing stores it.
- They are not one set with two hats. The bitmap marks *every* state visited,
  ε-states included; the list holds only what can still be there when the next
  character arrives.

**`traversed` carries two different things at two moments** — this was the single
biggest source of confusion, so name the phases:

| moment | contents | written by |
|---|---|---|
| after `find`'s start / after `step` | **seeds** — raw targets, `Jump`/`Split` allowed | `find`, `step` |
| after `closure` | **walls** — `Consume`/`Match` only, ranked | `follow` |

`closure` therefore *rebuilds* the list rather than appending to it (it must drop
the ε-seeds), while the bitmap only ever grows. Each phase must have exactly one
writer — two writers is what produced every duplicate-entry bug below.

**Where the answer is recorded:** while `step` reads the list.

```
walking clist at position i:
   Consume(c,t) matching → contribute t
   Match                 → result = Some(i), stop reading the list
```

And the asymmetry that makes greedy work: a `Match` cuts every thread **below it
in the current list** (they lose the tie now), but not threads already carried
forward from higher-ranked ancestors (they outrank it and may overwrite the
recorded offset later). `a*` on `"ab"` records `Some(0)` then `Some(1)`; the
climb *is* greediness.

**API shape:**

```
find       -> Option<usize>    end offset, exclusive; match is input[0..n]
full_match -> bool             find(input) == Some(input.chars().count())
```

`Some(0)` is a real match (the empty string) and is not `None`. Leftover input is
no longer failure — `find("ab", "abc") == Some(2)`.

**Concepts covered:**

- **Three different bools.** `fullmatch` (whole string), Python's `re.match`
  (prefix — what `find(..).is_some()` gives), and `is_match`/`.test()`
  (substring, needs the search loop). Libraries ship several; they are different
  questions, not different engines.
- **`is_match` never needs priority.** `is_some()` does not care which path won,
  so the bool API is exactly the Lesson 3 machine. That is why real engines keep
  a fast lane (RE2 and Rust's `regex` run a DFA for `is_match` and only fall back
  to the Pike VM when offsets or groups are wanted). The bool did not disappear;
  it stopped being the primitive and became the thing you throw information away
  to get.
- **Greedy vs lazy is one field swap.** `Split(body, exit)` vs `Split(exit,
  body)`. Same states, same language, different rank order. `*?` costs one line
  in `compile` and nothing in the matcher — once the parser can say it.
- **The list is not a stack.** It is built *by* a stack (`follow`'s recursion)
  and read front-to-back. LIFO inverts every rank, i.e. turns every quantifier
  lazy. If `follow`'s recursion is ever flattened into an explicit stack, push
  `Split`'s targets **y then x** so `x` pops first.
- **Priority is not history.** A thread's rank is its current position in the
  list, not a record of its route. If rank required remembering the path the
  merge would be unsound and the engine exponential — the same argument that
  forbids backreferences.
- **`Match` has no outgoing edges of any kind**, so it is never copied into
  `next`. Carrying it forward keeps the list non-empty and lets the result be
  overwritten at every later position.
- **The empty list records nothing.** Emptiness means every thread died — the
  opposite of a match. It is a valid early break (the empty set is absorbing),
  never a signal.
- **The final list must be read after the loop.** A match ending at end-of-input
  leaves `Match` in the last list with no character to trigger a read.
- Search is still a loop *around* this machine. `(start, end)` pairs are a
  restart loop over offsets — correct, but O(n²·m), and it needs the empty-match
  guard (`if end == start`, advance one) and a non-overlapping convention.
  Recovering linear time means the implicit low-priority `.*?` prefix, which
  requires threads to carry their start offset — i.e. the same machinery as
  captures. That is the next lesson.

**Bugs that surfaced:**

- **`closure` dropped its own seeds from the list.** `rebuild` empties the list,
  `follow` only pushes *targets*, and a seed is already marked so `insert`
  refuses it. Pattern `a`: `S_0 = ({0}, [])` — the only live thread vanished
  before the first character. Rule: `closure(X) ⊇ X` applies to the **list**, not
  just the bitmap. The confusing part: seeds that are ε-states are *supposed* to
  disappear, so the bug is invisible in every trace whose seeds are all `Jump`s.
- **Pushing unconditionally in `insert`** (before the seen check) to fix the
  above. Doesn't fix it — the seed is never re-inserted — and introduces
  duplicates: `a|a` on `"a"` gives `[5, 5]`, two threads on one state, which is
  exactly the merge the dedup check exists to perform.
- **`traverse(i)` in the `Jump`/`Split` arms of `follow`.** Puts ε-states in the
  list, and the `Split` arm pushes `i` twice when both arms are new.
- **`result = Some(i)` when the list went empty.** `"a"` on `"b"` returned
  `Some(0)` — a zero-length match for a pattern that cannot match the empty
  string — and `"a"` on `"a"` returned `None`, because the loop was the only
  reader.
- **`step` copying `Match` into `next`.** `a|b` on `"abc"` returned `Some(2)`:
  the list never emptied, `Match` was re-read at every later position, and each
  read overwrote the correct `Some(1)`.
- **`'\0'` as an end-of-input sentinel** for the final read. Worked by accident
  (the `Match` arm returns before any comparison) but `'\0'` is a real `char` in
  this alphabet — the same trap as `Consume('\0', _)` vs `Empty` in Lesson 2.
  Replaced by `is_match`, which takes no character.

**Teaching notes for next time:**

- **`a|b` on `"ab"` is a bad worked example for this lesson** and cost a lot of
  time: the `Match` read and the list going empty happen at the same moment, so
  the trace cannot show which one produces the answer. Victor said, correctly,
  that it contradicted the explanation. The discriminating pair is `"a"` on `"b"`
  (list empties, no match) and `"a"` on `"a"` (match, list never empties).
  `a*` on `"ab"` is the best single trace — it shows the record, the overwrite,
  and an empty list that records nothing.
- The recurring question was **"which structure holds what, and who writes it"**,
  not the automata. What landed was the two-phase table (seeds vs walls) plus
  "each phase has exactly one writer". Expect to draw it again when threads start
  carrying capture slots.
- "Why keep the bitmap at all?" — answer that landed: two jobs, and the dedup
  check is the innermost operation in the engine, so it must be O(1). A
  `list.contains()` scan makes closure quadratic and pays for the linear-time
  guarantee twice.
- "Ordered set" almost went to a sorted container. Sorted-by-state-index is not
  priority order; in `a|ab` it puts `Match` *below* the `b` thread and flips the
  answer. What is needed is **insertion order following DFS**.
- Hand-traces did the work for the fourth lesson running. He now writes them as
  `({bitmap}, [list])` pairs; keep that notation.
- Vocabulary still leaks: he wrote "`Match` → wall, stop" inside `step`. **Wall**
  is a closure word, **dies** is a `step` word, and in `step` a `Match` is
  neither — it is a **record**.

**Known open items** (not bugs to fix unprompted — raise them when relevant):

- **Lazy quantifiers are unobservable.** The matcher is ready; the parser cannot
  say `*?`. Grammar shape discussed: `repetition := atom QUANT*` with
  `QUANT := ('*' | '+' | '?') '?'?` — the trailing `?` is the lazy flag and may
  only follow a quantifier, never a bare atom. Note `a?` (a quantifier) and `a*?`
  (a modifier on a quantifier) are different grammar objects sharing a character.
- **`full_match` is wrong, and there is a red test for it**
  (`a_higher_priority_short_branch_must_not_hide_a_full_match`). It is derived as
  `find(input) == Some(len)`, but `find` applies the cut: on `a|ab` vs `"ab"` the
  rank-0 `Match` records `Some(1)` and kills the thread that would have reached
  `Some(2)`, so a string that *is* in the language is rejected. Deriving
  membership from a leftmost-**first** search is only sound if the search were
  leftmost-**longest**. `full_match` needs the Lesson 3 question instead —
  consume the whole input with no early record and no cut, then ask whether
  `Match` is in the final closed set. Victor left this deliberately unfixed to
  think about; do not fix it unprompted.
- Unanchored search, `(start, end)` spans, and capture groups — one subject
  rather than three, deferred to after Lesson 5.
- `find` is O(n·m); a restart loop for search is O(n²·m).
- `step` still allocates a fresh `SeenSet` per character; the stamp/generation
  trick and clist/nlist reuse are still deferred until a benchmark complains.
  `std::mem::take` in `rebuild` is the first step toward it.
- `follow` still recurses per ε-edge; stack depth bounded by `program.len()`.
- ~~Still unimplemented: `\` escapes, `.`, `+`, `?`, character classes~~ — done,
  see Lesson 5.a. Anchors and `{n,m}` are still open, see Lesson 5.b.

### Lesson 5.a — `+`, `?`, lazy quantifiers, character classes, and escapes (complete)

**Built:** the rest of the notation the matcher was already able to express.
All tests green across the parser, machine, and matcher layers (`cargo test`
for current counts — every feature below got tests at every layer it touches).

- [src/parser/ast.rs](src/parser/ast.rs) — `Ast` moved out of `parser/mod.rs`
  into its own file; gained `Plus`, `LazyStar`, `LazyPlus`, `Question`,
  `LazyQuestion`, `Class(Vec<ClassType>, bool)`, `Any`. `ClassType`
  (`Range(char,char)` / `Single(char)`) lives here too.
- [src/parser/mod.rs](src/parser/mod.rs) — `parse_star` renamed
  `parse_repetition`, generalized to a `loop`/`match` over `*`/`+`/`?`, each
  with an optional trailing `?` for laziness; `parse_class`; `\`-escape
  handling in `parse_atom`. `ParserError` gained `InvalidRange(char, char)`.
- [src/machine/class.rs](src/machine/class.rs) — new file: `Class` (the
  compiled form — `instructions: Vec<ClassInstruction>`, `negated: bool`,
  `exit: State`) and `ClassInstruction` (`Range`/`Single`, the machine's own
  type, translated from `ClassType` via `From<&ClassType>`).
- [src/machine/program.rs](src/machine/program.rs) — `Instruction`/
  `ValidInstruction` gained a `Class(Class)` variant. `ValidInstruction`
  **lost `Copy`** (kept `Clone`) — see below.
- [src/machine/mod.rs](src/machine/mod.rs) — `compile_plus`,
  `compile_lazy_star/plus/question`, `compile_question`, `compile_class`.
- [src/regex/mod.rs](src/regex/mod.rs) — `step`/`follow` changed from copying
  `program[*i]` by value to borrowing `&program[*i]`, to accommodate the
  non-`Copy` `Class` variant.

**`+` and `?` compile as gadgets, not desugared.** The Lesson 1 open question
(`A+ → AA*` vs. new AST variants) was settled in favor of new variants, and
it's cheaper than it looks: `compile_plus`/`compile_question` each compile the
child fragment **once** and reuse one of its two holes as the decision point,
mirroring `compile_star`'s existing `Split`-based shape instead of literally
duplicating the child subtree.

- `compile_star`: entry is a **new** `Split` (so zero iterations can skip the
  child); `frag.exit` is overwritten with `Jump(start)`, looping back.
- `compile_plus`: entry is `frag.start` itself (at least one iteration is
  mandatory — no way to skip the child); `frag.exit`'s hole is overwritten
  **directly** with `Split(start, exit)` — no separate `Jump`, the hole just
  becomes the decision point.
- `compile_question`: entry is a **new** `Split(frag.start, exit)`;
  `frag.exit` is **reused as-is**, untouched, as the fragment's own exit —
  both the "took it" and "skipped it" paths already converge there.

Net: `+` costs one extra state over its child (same as `*`), `?` costs one
extra state too, and neither ever clones an AST subtree or duplicates program
states. The desugaring alternative (`AA*`, `A|ε`) was rejected specifically
because `+`'s two copies of `A` would need `Ast: Clone`, and that cost
compounds under stacking (`a+++...`).

**Lazy quantifiers are the Lesson 4 principle applied a second time, and cost
one line each.** `LazyStar`/`LazyPlus`/`LazyQuestion` compile identically to
their greedy siblings, with the `Split`'s two arguments swapped —
`Split(frag.start, exit)` becomes `Split(exit, frag.start)`. Nothing in
`regex/mod.rs` changes at all; laziness is entirely a compile-time decision
baked into instruction layout, since `step`/`closure`/`follow` just walk
whatever `Split` they're handed, in ranked order.

**`a??` is genuinely ambiguous, and the grammar picks one reading.**
`QUANT := ('*'|'+'|'?') '?'?` means the lazy-flag `?` and a second, stacked `?`
quantifier share the exact same two characters. The parser always tries to
consume the lazy suffix first, so `a??` parses as `LazyQuestion(a)`, never
`Question(Question(a))`. This retired the `stacked_question` test (`a??` used
to assert the stacked reading) — but nothing was lost: `Question(Question(a))`
was always the same *language* as `Question(a)`, stacking any quantifier onto
itself never changes what strings match, only the tree shape. The stacked
reading is still reachable via explicit grouping (`(a?)?`), just not via the
bare `a??` spelling anymore.

**Character classes are a second, independent grammar living inside `[...]`,
with no lexer to lean on.** `class_item := CHAR '-' CHAR | CHAR`, and `^`/`-`/
`]` all mean different things depending on position:

- `^` negates only as the very first character of the class.
- `-` forms a range only when there's an **unconsumed** character immediately
  before it and one immediately after — general enough that `[--z]` is one
  range (`-` through `z`, since the first `-` is still the pending value when
  the second is read) while `[a-d-z]`'s second `-` is literal (`d` was already
  spent as the first range's end and can't be reused).
- `]` always closes; the PCRE quirk where a leading `]` is read as a literal
  member was deliberately **not** implemented.
- Range validity (`start <= end`, by Unicode scalar value) is checked **per
  item**, never across the whole class — `[za-az]` only ever compares the
  middle `a-a`, the two `z`s are never candidates.
- `(`/`)` have no meaning inside a class — they're just characters, the
  grammar never gives them a sub-expression slot the way `parse_atom`'s `(`
  does. This is also why `[(ab)-(cd)]` is a parse error: not because groups
  are special-cased, but because the flat class grammar composes `)-(` into an
  *inverted* range (`)` is 0x29, `(` is 0x28).

**Negation is one bool on the whole class, not a per-item flag — and this was
a real, caught bug, not a style call.** The first version had
`ClassType::NegatedSingle`/`NegatedRange` variants, negation attached
per-item. That's mathematically wrong the moment a negated class has 2+ items:
De Morgan's says the complement of a *union* is the *intersection* of
complements, not a union of complements. Concretely, `[^abc]` built from three
`NegatedSingle`s, tested by the natural "does `c` match any item" union, would
say `'a'` **is** a member — `'a' != 'b'` is true, so `NegatedSingle('b')`
"matches." Fixed by hoisting `negated: bool` onto `Ast::Class`/`Class` as a
whole, computing membership as a plain union scan, and inverting **once**, at
the very end (`if self.negated { !matched } else { matched }`) — the actual
De Morgan's-correct operation. Same "illegal states unrepresentable" move as
`ValidProgram`'s hole-elimination in Lesson 3: the type no longer permits a
`Vec` where items disagree about whether the class is negated.

**A second, independent bug in the same function:** `start` (tracking "is this
the first character of the class, so can `^` still negate") was only ever
reset to `false` on the one code path that falls through to the bottom of the
`loop` — every other branch (`continue`s for pushing a single char, or for the
negation trigger itself) skipped it. Result: `^` could wrongly trigger
negation anywhere in the class as long as no range had formed yet — `[a^]`
came out as "not a" instead of the literal set `{a, ^}`. Fixed by setting
`start = false` explicitly on every branch, not relying on fall-through.

**The `Copy` question resolved simpler than the plan going in.** The concern:
`ValidInstruction` was `Copy` (letting `regex/mod.rs` write
`let inst = program[i];` everywhere), and a `Class` holding a `Vec` can't be
`Copy`. The original plan was a side-table + `usize` index (mirroring "State
is an index, not a pointer" one level up) — but it turned out unnecessary.
`step`/`follow` only ever *read* an instruction, never need to own one, so
switching to `let inst = &program[*i];` and dereferencing the small `Copy`
fields at their use sites (`*t`, `*j`) works directly, with `ValidInstruction`
just dropping `Copy` and keeping `Clone`. Simpler than adding a pool +
indirection layer.

**`HashSet` was considered twice for class membership and rejected both
times, on the same grounds.** Once for the whole `Vec<ClassType>`, once
narrower (collapsing repeated `Single`s into one `Set(HashSet<char>)`
variant, deduplicating `[aaaaaaabbbbbbbcccccc]` for free). Rejected because:
real classes are small (a `HashSet`'s hashing + pointer-chasing overhead tends
to lose to a short contiguous `Vec` scan at that scale), ranges can't live in
a `HashSet<char>` at all without expanding them (the exact blowup already
rejected for `.`), and — mirroring the project's own precedent for the
`SeenSet` stamp/generation trick — deferred until an actual benchmark
complains, not built preemptively.

**`\` escapes are deliberately the naive version: consume the next character
unconditionally as a literal.** No `\n`/`\t` translation, no `\d`/`\w`/`\s`
shorthand classes — both are believed cheap to add later (new `match` arms
inside the same `\` branch in `parse_atom`, reusing `Ast::Literal`/
`Ast::Class` which already exist end-to-end) rather than a refactor risk. `\`
at end of input is `ParserError::UnexpectedEndOfInput`, not a crash.

**Concepts covered:**

- Every quantifier gadget (`*`, `+`, `?`, and their lazy twins) reduces to the
  same primitive: compile the child once, add a `Split`, decide whether entry
  is the `Split` or the child's own start, decide whether the exit edge loops
  back. Not three unrelated constructions.
- Stacking any quantifier onto itself is always language-redundant
  (`(a+)*`, `a?*`, `Question(Question(a))` all just restate an existing
  language in a more roundabout tree shape) — this is what let the `a??`
  grammar collision resolve in favor of laziness at zero real cost.
- Kleene's `∅` (the empty language, deliberately unexpressible by the
  top-level `Ast` since Lesson 2) has a *local*, class-scoped analogue: an
  empty character class `[]` would be a `Consume`-like atom that can never
  advance for any input, whether or not the top-level language can express
  `∅` directly. Left open, see below.
- Match ergonomics (Rust): matching `&Instruction` against a non-reference
  pattern shifts *every* binding in that arm to by-reference by default, not
  just the field that forced the change — dereferencing at use (`*t`) or
  matching `*inst` with a targeted `ref` on the one non-`Copy` field are both
  valid ways to opt back into by-value binding for the rest.
- A backtracking engine (PCRE) and a leftmost-first-*semantics* engine (this
  one) are different claims: the latter targets the same observable matching
  behavior as Perl/PCRE without their implementation strategy, and cannot
  reach feature parity (backreferences, lookaround) because that strategy is
  exactly what those features need. Syntax conventions (the `]`-first quirk,
  permissive empty branches) are separate, lower-stakes decisions from
  matching semantics.

**Bugs that surfaced (both in `parse_class`):**

- Per-item negation variants violating De Morgan's for any class with 2+
  items (see above).
- `start` not reset on most branches due to `continue` skipping the
  fall-through line (see above).

**Teaching notes for next time:**

- Hand-tracing individual `[...]` inputs against the exact grammar
  (`[a-d-z]`, `[--z]`, `[(ab)-(cd)]`, `[za-az]`) is what caught both bugs and
  pinned down every position-sensitivity rule — the same workflow that's
  worked every lesson so far, now applied to a second grammar nested inside
  the first.
- A reference doc ([character-classes.md](character-classes.md), written to
  the repo root) was useful as a standing spec to implement against and check
  drift against later — worth doing again for anchors/`{n,m}` if the rule set
  gets similarly fiddly.
- He caught his own bugs and proposed his own fixes for both the negation
  redesign and the `Copy`-via-borrowing simplification — the tutoring loop is
  increasingly "raise the concern, let him find the shape of the fix," not
  "here's the fix."
- Confusions worth watching for again: "is `a?` the same as `a*?`" (quantifier
  vs. modifier-on-a-quantifier sharing a character — the exact ambiguity that
  later broke `a??`), and reading "PCRE-style" as "we're building PCRE" rather
  than "borrowing a syntax convention while the matching strategy stays
  fundamentally different."

**Known open items** (not bugs — raise when relevant):

- **The empty class `[]` — flagged as the most important open item to resolve
  first in 5.b.** Currently legal by accident, not decision: `parse_class`
  returns `Ok(Ast::Class(vec![], negation))` the moment it sees `]` with zero
  items collected, since nothing currently requires `class_item+` over
  `class_item*`. Needs a deliberate choice (error vs. legal-and-always-
  failing), and if legal, `[^]` (negated empty class) becomes an interesting
  edge — "not in the empty set" is every character, the same language as `.`
  reached by a completely different route.
- `$`/`^` anchors — a structurally new instruction category: zero-width,
  doesn't consume input, but (unlike `Jump`/`Split`) needs to test *position*
  (start/end of input) and die like a failed `Consume` if the assertion
  doesn't hold. Not a variation on an existing instruction.
- `{n,m}` bounded repetition — still fully open, no grammar or compile shape
  discussed yet.
- The dead code in `Class::match_c`:
  `if !self.negated && matched { break; }` can never fire, since the inner
  `break` inside each match arm already exits the loop the moment `matched`
  becomes `true`. Harmless, worth deleting whenever that function is touched
  again.
- Two `full_match` bugs remain deliberately unfixed from Lesson 4, and a
  Lesson 5 addition demonstrated the second one has the same root cause as the
  first: deriving `full_match` from `find`'s leftmost-*first* cut is unsound
  whenever the highest-priority path is short.
  `a_higher_priority_short_branch_must_not_hide_a_full_match`
  (alternation-driven) and `lazy_quantifiers_expose_the_same_full_match_bug`
  (laziness-driven, e.g. `a??` vs `"a"`) are both red on purpose.
- All of Lesson 3/4's older open items (sparse sets, `clist`/`nlist` reuse,
  stamp/generation, unanchored search, `(start,end)` spans, capture groups)
  are still open too — untouched by Lesson 5.a.

### Lesson 5.b — anchors, bounded repetition, and the empty class (next)

Picks up exactly where 5.a stopped, in the priority order set at the end of
the session:

1. **Resolve `[]` first.** Decide `class_item+` vs `class_item*`
   deliberately, update `parse_class` accordingly, and if empty classes are
   made legal, decide whether that's worth exploiting anywhere (the
   `[^]` ≡ `.` connection is cute but not obviously useful for anything else
   yet).
2. **`$`/`^` anchors.** Needs a new instruction category (a zero-width
   position assertion, not a consuming instruction and not a free
   `Jump`/`Split`) — this is genuinely new theory for the lesson sequence, not
   just more parser/compiler plumbing like most of 5.a was. `^` is already
   overloaded (class negation vs. anchor, resolved by the same
   "position decides meaning" theme running through this whole lesson) —
   expect that collision to need explicit handling in `parse_atom`, the same
   way `?` needed it for lazy quantifiers.
3. **`{n,m}`** (and `{n}`, `{n,}`, `{,m}`) — no design work done yet at all.
   Generalizes `*`/`+`/`?` (`{0,}=*`, `{1,}=+`, `{0,1}=?`), so the compile
   shape is probably another variation on the "compile the child once, wire a
   decision point" gadget family from 5.a, but the *parsing* is new: it needs
   to read and validate a bounded integer count, not just a single quantifier
   character.

## Protocol for agents

**When a lesson is completed, append its record to the Progress section above.**
Follow the Lesson 1 format: what was built, which files, concepts covered, and
known open items. This file is the context for the next session — a concept
explained but unrecorded will have to be re-explained from scratch.

Record the *reasoning* behind decisions, not just the decisions. The point of the
project is understanding, and the file should carry that forward too.
