# rregex

A regex engine written from scratch in Rust, no dependencies.

## This is a teaching project — read this before doing anything

**Victor is learning. He writes all the code. You do not.**

This repository exists so that Victor understands regex from the inside out. He
has said he is "really bad at regex" and is building the engine specifically to
fix that. Delivering working code would defeat the entire purpose of the project.

Your role is **tutor**, not implementer:

- Explain the theory: automata, Kleene algebra, grammars, complexity.
- Give grammars, invariants, traces, and counterexamples.
- Name the traps *before* he hits them, and diagnose bugs by explaining the
  underlying rule rather than posting a patch.
- Point at the specific line or concept that is wrong, and let him fix it.

**Do not write implementation code.** Not "just a sketch," not "here's roughly
how it would look," not a helpful `impl` block to get him unstuck. If he is
stuck, explain the rule he is missing. A 3-line snippet illustrating a *concept*
(e.g. `while` vs `if` in a loop) is fine; a working function is not.

Standing exceptions, both granted on request: **tests** (they are a spec, not an
implementation) and **benchmark/profiling harness code** (it is not the engine).
Ask before writing anything else.

### Corrections he has already had to make

1. **Don't get ahead of the lesson.** He asked for the 11 tests named in Lesson
   1; a 40-test suite covering unasked-for decisions was not welcome. Deliver
   the scope named, not the scope you think is better.
2. **Keep answers short when he asks for short.** He will say so explicitly.
3. **Keep this file compressed.** It is loaded into context every session.
   Record live decisions and concepts; do not narrate history. No abandoned
   plans, no tool-choice rationale that never touches the engine, no resolved
   open items left lying around, no restating what the decisions table says.

Victor is comfortable in Rust. Skip language mechanics; stay on the algorithms.

## Established design decisions

Don't relitigate these without being asked.

| Decision | Choice | Reasoning |
|---|---|---|
| Matching strategy | **Thompson NFA simulation** (Pike VM style) | Linear-time guarantee, teaches the automata theory properly. Accepts that backreferences become impossible. |
| Alphabet | **`char`** (Unicode scalar values) | Simpler than bytes; `.` and ranges behave naturally. |
| Lexer | **None** | Regex tokens are single characters and context-sensitive (`]`, `-`, `^`, `*` all change meaning by position). A cursor over the chars, not a token stream. |
| Concat / Alternation shape | **Binary** — `Concat(Box<Ast>, Box<Ast>)`, `Alternation(Box<Ast>, Box<Ast>)` | The NFA needs bounded fan-out: `Split` holds exactly two targets. Binary nodes put the fold in the parser, so `compile` maps one node to one instruction. Cost: right-recursion makes parser stack depth O(input length). |
| Empty branches | **Permissive** (PCRE-style) — `Ast::Empty`, not an error | |
| Quantifiers | **Own AST variants + compile gadgets**, not desugaring (`A+ → AA*`, `A? → A\|ε`) | Desugaring needs `Ast: Clone` and duplicates the child subtree, compounding under stacking (`a+++`). A gadget compiles the child once and adds one `Split`. |
| State set | **Bitset + ordered list** — `SeenSet { seen: Vec<bool>, traversed: Vec<State> }` | States are dense (`< program.len()`), so `seen` membership is one indexed load, no hashing. `traversed` is appended in `follow`'s DFS order and **rank is the index** — leftmost-first needs the backtracker's search order, which a bitset has none of. Keeping both means dedup stays O(1); a `contains()` scan would make closure quadratic. |
| Buffers | **Two long-lived `SeenSet`s, swapped** | `find` allocates exactly twice regardless of input length. Never `mem::take` a buffer field — it leaves a zero-capacity `Vec` behind. |
| Match semantics | **Leftmost-first**, not POSIX leftmost-longest | It is what every mainstream engine does and what intuition expects. `Split(body, exit)` order is greediness; the swap is `*?`. |
| Class negation | **One `bool` on the whole class**, never per-item | De Morgan's: the complement of a union is the intersection of complements. Invert once, at the end. |
| Possessive quantifiers / atomic groups | **Not implemented, and can't be** | They require discarding a live thread because of an earlier commitment, so a thread's fate would depend on more than `(state, position)` — the same invariant that forbids backreferences. Consequence: `+` after a quantifier is never a possessive suffix here, just ordinary stacking (`a{3,4}++` is `Plus(Plus(…))`), a deliberate divergence from PCRE. |

## Progress

### Lesson 1 — the front end (complete)

**Built:** a recursive-descent parser producing an AST.

- [src/parser/mod.rs](src/parser/mod.rs) — `ParserError` and the parse functions
- [src/parser/ast.rs](src/parser/ast.rs) — `Ast`, `ClassType` (added in Lesson 5)
- [src/parser/cursor.rs](src/parser/cursor.rs) — position cursor over `Vec<char>`

**The grammar** (precedence falls out of the nesting; extended in Lesson 5):

```
alternation   := concatenation ('|' concatenation)*
concatenation := repetition*
repetition    := atom QUANT*
QUANT         := ('*' | '+' | '?') '?'?          -- trailing '?' is the lazy flag
atom          := CHAR | '(' alternation ')' | '\' ESCAPE | '[' class ']' | '.'
```

Call chain is a **cycle**, not a line: `parse_atom` recurses back up to
`parse_alternation` on `(`, which is what permits unbounded nesting. The bottom
level is named `atom`, not `literal`, because it also owns `(`.

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
  (identity `ε`), and `∅a = ∅` annihilates. `a(b|c) = ab|ac` distributes. The
  `2x + 3y` analogy is what made precedence click — use it again.
- **Postfix operators bind to exactly one atom.** `ab*` is `a(b*)`. This was the
  misconception that needed correcting; watch for it recurring.
- Parentheses turn a multi-part expression into a single atom — that is their
  entire structural purpose (and why `(?:...)` exists). **They do not survive
  into the AST**: "abstract" means notation is discarded once structure encodes
  it. There is no `Atom` node; `Atom` is a grammar category. Capturing groups
  will later need a node — but for *capture*, not for grouping.
- Invariant: the collect-then-emit rule. Zero children → `Empty`, one child →
  the **bare child**, two or more → the wrapper. No single-element wrapper may
  reach the tree.
- `parse_concat`'s stop set is exactly `{ '|', ')', EOF }` and it must never
  *consume* `|` or `)`. `parse_repetition` drains all postfix operators before
  returning, which is why `*` never appears in that stop set.
- **Branch count = pipe count + 1.** `parse_alternation` parses one concat up
  front, then `while eat('|')` parses another **unconditionally** — never peek
  first. Consuming a `|` is a commitment that a branch follows, and at EOF
  `parse_concat` returns `Empty`, which is the wanted node. The original version
  skipped `|` with `continue`, so empty branches silently vanished and `a|`
  parsed as `Literal('a')`. Green tests are not the same as correct.
- Top-level `parse` must verify the cursor reached EOF. Without it `a)`, `ab)cd`,
  and `a))))` all parse "successfully" — the classic silent-success bug.
- The catch-all `Some(c) => Literal(c)` arm means any stop-set bug becomes a
  wrong literal instead of an error, so `|` and `)` are guarded explicitly in
  `parse_atom` even though they are currently unreachable there.
- `.` is not a Literal. It is sugar for an alternation over the whole alphabet
  (~1.1M branches with `char`), so it gets its own node as a compression. It is
  the first member of the character-class family.
- `a**` parses as `Star(Star(a))`. The bug that surfaced was `if` instead of
  `while` in the repetition loop.

### Lesson 2 — Thompson's construction (complete)

**Built:** a compiler from `Ast` to a flat NFA program.

- [src/machine/mod.rs](src/machine/mod.rs) — `Fragment`, `Machine::new`, the
  `compile_*` functions
- [src/machine/program.rs](src/machine/program.rs) — `Instruction`,
  `ValidInstruction`, `Program`, `ValidProgram`

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
- **Few instructions, because fan-out is ≤ 2.** Only `Alternation` and `Star`
  branch, and each creates exactly a 2-way choice, so an edge is a fixed field
  rather than a list. `Consume`, `Jump`, `Split`, `Match` — plus `Hole`.
- **State and instruction are the same object from two directions.** State is
  the automata view (a dot); instruction is the VM view (a line of a program).
  Same slot. This is why the matcher is a loop with a program counter.
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

**`ValidProgram` — the hole assertion became a type.** `ValidProgram::new`
consumes a `Program` and returns `Vec<ValidInstruction>`, the same enum minus
`Hole`, so the matcher has no unreachable arm to write. Illegal states made
unrepresentable, at exactly the boundary where the program stops being under
construction. Victor's own move; the same reasoning later drove class negation.

**Concepts covered:**

- An NFA is a directed graph plus a walking rule. Transitions are a *relation*,
  not a function: multiple targets per char, plus ε-edges taken free.
  Acceptance is existential — *some* path consumes the whole string.
- **ε-transitions buy composability, not power.** The invariant *exactly one
  start, exactly one accept; nothing enters the start, nothing leaves the
  accept* makes a fragment a black box with one plug and one socket, so gluing
  is a single ε-edge with no case analysis on what's inside. Without it, a
  fragment carries a *set* of exits and every gadget grows loop-and-union
  bookkeeping. One sentence: **ε-edges convert a set of exits into a single
  exit.** The price is state count and ε-chasing; the payoff is a construction
  linear in AST nodes and correct by a two-line induction — and that linearity
  is what makes the O(states × input) guarantee worth anything.
- **Failure is the absence of an arrow.** The transition relation is partial —
  no dead state, no trap.
- **The machine answers exactly one question:** does *this whole string* belong
  to the language? Unanchored matching, "how many times does it match", and
  submatches are all *search* concerns — a loop wrapped around the machine.
  Keep that boundary sharp.
- Compile-time sees no input. `compile` never touches a string.
- Recursion is **post-order**: children finish before parents, siblings left to
  right. Sibling order only changes numbering, but left-to-right is what the
  `match` arms already read as.
- **The start state is usually not index 0.** Indices follow creation order, and
  branch nodes push their `Split` only after their children exist. `(a|b)*c`
  starts at 6.
- **Greedy vs lazy is the order of a `Split`'s two targets.** `Split(body,
  exit)` prefers the body, which is what makes `*` greedy once the set is
  ordered. `*?` is that swap.
- ε is **not a symbol**. `Consume('\0', _)` is not `Empty`: it would make `""`
  fail to match and `"\0"` start matching, because in a `char` alphabet every
  `char` occurs. `Empty` is the language `{""}`; `∅` is `{}` and the AST
  deliberately cannot express it. (Kleene: ε is concat's identity, `∅` is
  alternation's identity and concat's annihilator.)
- Thompson's construction is **local** and must not reject a well-formed tree,
  so it happily emits ε-loops: `a**` gives `4: Split(2,5)`, `2: Split(0,3)`,
  `3: Jump(4)`, a cycle in which nothing advances the input.
  `stacked_stars_build_an_epsilon_loop` pins the slots. The matcher's
  already-seen check is what makes this safe.

**Bugs that surfaced (all self-inflicted index arithmetic, all instructive):**

- Returning an exit index for a slot that was never pushed. The next fragment
  lands in that slot and the parent's patch **overwrites a real instruction**,
  producing a silent ε self-loop instead of a crash. Rule: *a hole must occupy a
  slot*; a fragment's `exit` must exist the moment the fragment returns.
- `program.len()` after a push is the index of the *next* slot, not the one just
  pushed. Rule: *the index of a slot is `len()` at the moment you push it* —
  capture it before the push.
- Reaching inside a child fragment when wiring. His first `a|b` merged the two
  literals onto shared states; his first `Star` added an edge from the child's
  start straight to the child's accept, creating a **pure ε-cycle in a plain
  `(a|b)*`** — correct language, broken machine. Rule: a gadget may use a
  child's `start` and `exit` as *two integers* and nothing else.
- Wiring the outside of a `Concat` and leaving the seam disconnected, giving a
  two-component graph that matches nothing. `Concat` pushes zero states; its
  entire product is the seam edge.

**Teaching notes:**

- He inverted sibling order twice ("compile the last child first").
- He reads a wiring line like `(5) ⇢ (6)` as a *description* of what the child
  already does, rather than a new edge being added. The fix that landed: "start
  and exit are two integers you carry, not a promise of free travel."
- Working the gadgets as hand-drawn diagrams **before** any Rust is what made it
  stick. He then predicted each program slot-by-slot in a comment before writing
  the arm, and every prediction was right. Keep this workflow.
- Asserting the *whole* program slot by slot is the right test shape for a
  compiler — it pins the numbering, which is what every index bug corrupts.

### Lesson 3 — the simulator (complete)

**Built:** a Thompson NFA simulator (Pike VM shape, minus priority).

- [src/regex/mod.rs](src/regex/mod.rs) — `Regex`, `SeenSet`, `step`, `closure`,
  `follow`
- [src/regex/error.rs](src/regex/error.rs) — `RegexError`, `From<ParserError>`

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

**Division of labour:**

| fn | reads input? | job |
|---|---|---|
| `step` | yes | for each live `Consume(c, t)` with `c` == the char, contribute **`t`**. Everything else dies. Produces an *unclosed* set. |
| `closure` | no | seeding loop: call `follow` on every state already in the set |
| `follow` | no | the walk: `Jump`/`Split` → check-mark-recurse per target; `Consume`/`Match` → wall |

**Concepts covered:**

- **The set is the whole trick.** A `Split` does not create two sets; it puts
  two states into the one set. Two sets would be two independent machines —
  that is backtracking, and it is exponential.
- **Simulation is subset construction done lazily.** A set of NFA states *is* a
  DFA state; the simulator builds one at a time and throws it away. Same
  insight, no exponential table.
- **The invariant:** `S_i` = exactly the states reachable from `start` by some
  path spelling the first *i* characters, where "reachable" always includes free
  ε-travel. Position 0 is not a special case — it is that rule with *i* = 0,
  which is why `S_0` needs a closure even though no input has been read.
  Skipping it breaks every regex whose start state is a `Split` and makes `a*`
  reject `""`.
- **The set is a position, not a log.** It records where fingers stand, never
  where they have been. `S_1 == S_2` for `a*` on `"aa"` is a loop at steady
  state, not a bug.
- **Lockstep.** One input pointer for the whole machine; every finger is always
  at the same input position. Nothing ever rewinds — that is the linear-time
  guarantee.
- **`|` splits the pattern, not the input.** Both branches are rivals for the
  *same* character. Concatenation is the operator that divides the input
  between two sub-patterns; alternation never lengthens the language, only
  widens it.
- **The dedup check is termination, not validation.** Arriving at an
  already-marked state is normal, expected, and load-bearing: it is the base
  case of the walk. Every `Star` compiles to a cycle by construction, so a
  matcher that treats a revisit as an error rejects every star. Chalk-marks in
  a maze: you turn around, you do not declare the maze invalid.
- The same check does two jobs, and they are the same fact: it terminates
  ε-cycles, and it merges duplicate paths so the set is bounded by the number of
  **states** rather than **paths**. Paths vs. states is the whole distance
  between ReDoS and O(n·m).
- Which is exactly why **backreferences are impossible**: the merge is sound
  only because a finger's future depends on `(state, input position)` and
  nothing else. `\1` would make history matter, and two fingers on one slot
  would stop being interchangeable. The merge buys the speed and forbids the
  feature.
- **The empty set is absorbing.** `step({}, c) = {}` for every `c`, so an early
  break is sound — an optimisation, not a correctness requirement.
- **`Match` has no outgoing edges of any kind.** Closure walls on it; `step` has
  no arm carrying it forward. A finger that lands on `Match` dies at the next
  character.
- **The machine answers membership, not search.** `ab` does not match `"abc"`;
  `a|b` does not match `"ab"`.

**Bugs that surfaced:**

- `step` contributing `i` instead of `j` — parking the finger back on the
  `Consume` it just executed. Rule: *what survives is where the arrow pointed,
  not the instruction that pointed*.
- `let seen_set = self.step(...)` **inside** the `for` body — a new binding
  scoped to the loop, so every character was matched against `S_0` forever.
  Shadowing in a loop body always does this.
- Writing `closure` as a **scan over the program** rather than a walk from a
  given set: it marked every state with an incoming edge and never mentioned
  `machine.start()`. A closure that does not read the start state cannot be
  computing reachability from it. Diagnostic: a walk needs "reached but not yet
  examined"; `for inst in program` has no such thing.
- The first `closure` also built a fresh empty `next`, dropping its own input.
  Rule: *closure only ever adds*; `closure(X) ⊇ X`. The walls it was discarding
  were the entire answer.
- Returning `Err(StateLoop)` from the already-seen check — detecting the cycle
  correctly and drawing the wrong conclusion. Surfaced on `a|b|c` against `"b"`,
  where one state was reached twice by two different routes and there was no
  cycle at all.
- `if seen[j1] || seen[j2]` on a `Split` — abandoning **both** arms because one
  was seen. The arms are independent; a stale `j1` says nothing about a fresh
  `j2`.

**Teaching notes:**

- **The recurring misconception is anchoring, not automata.** He predicted a
  match for `ab` vs `"abc"` and for `a|b` vs `"ab"`, twice, *after* correctly
  tracing both to `false`. The fix that worked: enumerate the language as a
  literal list of strings (`a|b → { "a", "b" }`) and count characters before
  tracing.
- "Why does `S_0` exist at all?" needs answering from the invariant, not the
  code: position 0 is the base case of the same rule, and `a*` on `""` is the
  counterexample that makes it concrete.
- "How did `S_2` know the jump in `S_1` was taken?" — reading `step` as
  consulting history. Answer that landed: **the fact lives in the contents of
  the set, not in a flag.** State 2 being present *is* "the jump was followed."
  Related: he assumed the visited marker persists across positions; the
  counterexample is `a*` on `"aa"`, where state 0 must be re-added at every
  position.
- Direction of comparison confused him once: `step` iterates the *set* and asks
  each `Consume` about the character, rather than pushing the character through
  the program looking for a home.
- Keep the two vocabularies apart — he mixes them. **Wall** is a closure word
  (no ε-edge to follow). **Dies** is a `step` word (no matching arrow, not
  carried forward). Mixing them is a reliable early sign that the two phases
  have blurred.

### Lesson 4 — priority (complete)

**Built:** a priority-ordered simulation — the Pike VM shape, minus captures.
`find` returns an end offset instead of a `bool`.

- [src/regex/mod.rs](src/regex/mod.rs) — `SeenSet` gains `traversed`; `find`,
  `full_match`, `is_match`
- [src/regex/tests/find.rs](src/regex/tests/find.rs),
  [src/regex/tests/full_match.rs](src/regex/tests/full_match.rs) — matcher
  tests, split by which question they ask

**Why the answer had to stop being a `bool`.** Acceptance is existential, so
`bool` throws away *which* accepting path was found. The moment the answer
carries data — an offset, later a span or a capture — the paths disagree and a
tie-breaking rule is required. `a*` on `"aaa"` has four accepting walks; `a|ab`
and `ab|a` describe the same language and must give different answers.

**The two schools.** POSIX is leftmost-**longest**, defined declaratively.
Perl/PCRE/Python/JS/Rust are leftmost-**first**, defined *operationally* by a
backtracker's search order: leftmost start, first alternative before second,
loop before exit. This engine implements leftmost-first, which is what "greedy"
means — not "longest", but "the loop branch is attempted first".

**The trick, which is the whole lesson.** Perl semantics are defined by a DFS
this engine refuses to perform. You recover the backtracker's *answer* without
the backtracker by making the set an **ordered list**, appended in the order
`follow`'s recursion visits states. Then the Lesson 3 dedup check becomes the
tie-breaker for free: first arrival wins, and first arrival is the
highest-priority path. The check that bought termination and linear time now
also buys Perl semantics.

> A backtracker never **creates** the losing thread. The Pike VM creates it,
> runs it, and **cuts** it. That is why work per character stays bounded.

**`traversed` carries two different things at two moments** — the single biggest
source of confusion, so name the phases:

| moment | contents | written by |
|---|---|---|
| after `find`'s start / after `step` | **seeds** — raw targets, `Jump`/`Split` allowed | `find`, `step` |
| after `closure` | **walls** — `Consume`/`Match` only, ranked | `follow` |

`closure` therefore *rebuilds* the list rather than appending to it (it must
drop the ε-seeds), while the bitmap only ever grows. Each phase must have
exactly one writer — two writers produced every duplicate-entry bug below.

**Where the answer is recorded:** while `step` reads the list.

```
walking the list at position i:
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

`Some(0)` is a real match (the empty string) and is not `None`. Leftover input
is no longer failure — `find("ab", "abc") == Some(2)`.

**Concepts covered:**

- **Three different bools.** `fullmatch` (whole string), Python's `re.match`
  (prefix — what `find(..).is_some()` gives), and `is_match`/`.test()`
  (substring, needs the search loop). Libraries ship several; they are different
  questions, not different engines.
- **`is_match` never needs priority.** `is_some()` does not care which path won,
  so the bool API is exactly the Lesson 3 machine. That is why real engines keep
  a fast lane (RE2 and Rust's `regex` run a DFA for `is_match` and fall back to
  the Pike VM only when offsets or groups are wanted). The bool stopped being
  the primitive and became the thing you throw information away to get.
- **Greedy vs lazy is one field swap.** `Split(body, exit)` vs `Split(exit,
  body)`. Same states, same language, different rank order.
- **The list is not a stack.** It is built *by* a stack (`follow`'s recursion)
  and read front-to-back. LIFO inverts every rank, i.e. turns every quantifier
  lazy. If `follow`'s recursion is ever flattened into an explicit stack, push
  `Split`'s targets **y then x** so `x` pops first.
- **Priority is not history.** A thread's rank is its current position in the
  list, not a record of its route. If rank required remembering the path the
  merge would be unsound and the engine exponential — the same argument that
  forbids backreferences.
- **The empty list records nothing.** Emptiness means every thread died — the
  opposite of a match. A valid early break, never a signal.
- **The final list must be read after the loop.** A match ending at end-of-input
  leaves `Match` in the last list with no character to trigger a read.
- Search is still a loop *around* this machine. `(start, end)` pairs are a
  restart loop over offsets — correct, but O(n²·m), and it needs the empty-match
  guard (`if end == start`, advance one) and a non-overlapping convention.
  Recovering linear time means the implicit low-priority `.*?` prefix, which
  requires threads to carry their start offset — i.e. the same machinery as
  captures.

**Bugs that surfaced:**

- **`closure` dropped its own seeds from the list.** Rebuilding empties the
  list, `follow` only pushes *targets*, and a seed is already marked so `insert`
  refuses it. Pattern `a`: `S_0 = ({0}, [])` — the only live thread vanished
  before the first character. Rule: `closure(X) ⊇ X` applies to the **list**,
  not just the bitmap. The confusing part: ε-state seeds are *supposed* to
  disappear, so the bug is invisible in every trace whose seeds are all `Jump`s.
- **Pushing unconditionally in `insert`** (before the seen check) to fix the
  above. Doesn't fix it — the seed is never re-inserted — and introduces
  duplicates: `a|a` on `"a"` gives `[5, 5]`, two threads on one state, exactly
  the merge the dedup check exists to perform.
- **Traversing in the `Jump`/`Split` arms of `follow`.** Puts ε-states in the
  list, and the `Split` arm pushes twice when both arms are new.
- **Recording a result when the list went empty.** `"a"` on `"b"` returned
  `Some(0)` — a zero-length match for a pattern that cannot match the empty
  string.
- **`step` copying `Match` into `next`.** `a|b` on `"abc"` returned `Some(2)`:
  the list never emptied, `Match` was re-read at every later position, and each
  read overwrote the correct `Some(1)`.
- **`'\0'` as an end-of-input sentinel** for the final read. Worked by accident
  but `'\0'` is a real `char` in this alphabet — the same trap as
  `Consume('\0', _)` vs `Empty`. Replaced by `is_match`, which takes no
  character.

**Teaching notes:**

- **`a|b` on `"ab"` is a bad worked example for this lesson** and cost a lot of
  time: the `Match` read and the list going empty happen at the same moment, so
  the trace cannot show which one produces the answer. Victor said, correctly,
  that it contradicted the explanation. The discriminating pair is `"a"` on
  `"b"` (list empties, no match) and `"a"` on `"a"` (match, list never empties).
  `a*` on `"ab"` is the best single trace — it shows the record, the overwrite,
  and an empty list that records nothing.
- The recurring question is **"which structure holds what, and who writes it"**,
  not the automata. What landed was the two-phase table (seeds vs walls) plus
  "each phase has exactly one writer". Expect to draw it again when threads
  start carrying capture slots.
- "Ordered set" almost went to a sorted container. Sorted-by-state-index is not
  priority order; in `a|ab` it puts `Match` *below* the `b` thread and flips the
  answer. What is needed is **insertion order following DFS**.
- He writes traces as `({bitmap}, [list])` pairs; keep that notation.

### Lesson 5 — quantifiers, classes, anchors, `{n,m}` (complete)

**Built:** the rest of the notation the matcher could already express, plus two
new grammars (classes, anchors) and bounded repetition end to end (parse and
compile).

- [src/parser/ast.rs](src/parser/ast.rs) — `Plus`, `LazyStar`, `LazyPlus`,
  `Question`, `LazyQuestion`, `Class(Vec<ClassType>, bool)`, `Any`,
  `Anchor(AnchorKind)`, `BoundedRepetition`
- [src/parser/mod.rs](src/parser/mod.rs) — `parse_repetition` generalized over
  `*`/`+`/`?`/`{n,m}` with optional lazy suffix; `parse_class`; `\`-escapes;
  `^`/`$` in `parse_atom`
- [src/parser/bounded_repetition.rs](src/parser/bounded_repetition.rs) —
  `parse_bounded_repetition` and friends, `Cursor::peek_number_at`
- [src/machine/class.rs](src/machine/class.rs) — `Class`, `ClassInstruction`
- [src/machine/position.rs](src/machine/position.rs),
  `Instruction::ConditionalJump(Position, State)` — anchors
- [src/machine/mod.rs](src/machine/mod.rs) — `compile_plus`, `compile_question`,
  the lazy variants, `compile_class`, `compile_anchor`,
  `compile_bounded_repetition`
- References: [character-classes.md](character-classes.md),
  [anchors.md](anchors.md), [bounded-repetition.md](bounded-repetition.md)

**Quantifier gadgets are one pattern, two switches:** compile the child once,
add a `Split`, then decide (a) whether entry is the `Split` or the child's own
start, and (b) whether the exit loops back.

- `compile_star`: entry is a **new** `Split` (zero iterations skip the child);
  `frag.exit` ← `Jump(start)`, looping back.
- `compile_plus`: entry is `frag.start` itself; `frag.exit`'s hole is
  overwritten **directly** with `Split(start, exit)` — no separate `Jump`.
- `compile_question`: entry is a **new** `Split(frag.start, exit)`; `frag.exit`
  is reused untouched.

Lazy variants swap the `Split`'s two arguments — purely compile-time, nothing in
`regex/mod.rs` changes. `a??` is genuinely ambiguous (lazy-flag vs. a second
stacked `?`); the parser resolves it as `LazyQuestion`, losing nothing since
stacking any quantifier onto itself is always language-redundant.

**Character classes are a second grammar inside `[...]`**, position-sensitive:
`^` negates only as the first character, `-` forms a range only with an
unconsumed char on both sides, `(`/`)` mean nothing inside `[...]`, `]` always
closes. **Negation is one bool on the whole class, not per-item** — a real
caught bug: De Morgan's says the complement of a union is the intersection of
complements, so per-item `NegatedSingle`/`NegatedRange` made `[^abc]` say `'a'`
**is** a member of its own negation. Fixed by hoisting `negated: bool` and
inverting once at the end (same "illegal states unrepresentable" move as
`ValidProgram`). A second bug in the same parser: the `start` flag (tracks
whether `^` can still negate) only reset on fall-through, not on every
`continue`, so `[a^]` parsed as negated instead of `{a, ^}`. `\` escapes are
deliberately naive — consume the next char as a literal, no `\d`/`\w`/`\s` yet.

**The empty class `[]`/`[^]` is legal by decision, not accident.** No code
changed: the union-scan-then-negate-once logic already gives the right answer
for zero items — `[]` is ∅, `[^]` is every char (`.`'s language by a different
route), `[]*` still matches `""` (`∅* = ε`, nothing reified).

**Anchors are a third instruction category** — zero-width *and* able to fail,
on position rather than a character. `find("$", "")` is the forcing case:
`step` never runs on empty input, so the test must live in `closure`. An anchor
is **a `Jump` with a test**: groups with `Jump`/`Split` in `follow`, never a
wall, rejection is silent (no insert, no recurse) — same "dies" semantics as a
failed `Consume`. `^` is vacuous today (`find` is already anchored at 0); `$`
is the one that constrains anything.

**`{n,m}` parsing is attempt-then-commit.** `{` is only a quantifier if a whole
sub-grammar (digits, optional comma, digits, `}`) validates; nothing touches
the cursor until a final `move_to`, so any bail-out falls back to `parse_atom`'s
literal catch-all (PCRE-style). `u16` for counts — PCRE's real ceiling is
65535, the type itself is the limit check. Overflow is checked *after* the
shape is confirmed.

**`{n,m}` compiling is the first gadget that compiles its child more than
once.** No `Ast: Clone` needed — `compile_fragment` takes `&Ast`, so the gadget
stays as blind to the child's shape as `compile_plus`. Unrolled as `n` mandatory
copies (plain `Concat` chaining) then `m - n` optional copies, where each
optional copy's decision `Split` is *authored by the gadget itself* and so can
be written directly into the previous fragment's exit-hole instead of costing a
new state — the same trick `compile_plus` uses. `{n,}` is `n` mandatory copies
with the *last* one given the `compile_plus` treatment; `{0,}` needs
`compile_star`'s treatment instead (a real `Split` at the very first entry, not
a `Jump`), since with `n == 0` there's no mandatory copy to loop back onto.

**Bugs that surfaced (bounded-repetition compiling):**

- `compile_bounded_repetition` returned the most recently compiled *child* as
  the whole construct's `Fragment`, so `.start` pointed at the last copy's own
  internal state instead of the true entry — anything concatenated before it
  would skip the required repetitions. Fixed by capturing the entry point once,
  never overwriting it.
- The `n == 0` unbounded case (`{0,}` = `*`) first used a plain `Jump` into the
  child like every mandatory copy, forcing at least one repetition since there
  was no branch to skip it — needed `compile_star`'s `Split`-at-entry instead.
- `{0,0}` (both loops run zero times) originally panicked with nothing to
  return; fixed by always returning the untouched leading hole as
  `start == exit`, exactly `Empty`'s shape — no special case needed.

**Teaching notes:**

- Hand-tracing NFA diagrams **before writing code** caught every bounded-
  repetition bug: fan-out > 2 at a decision state, a stop/continue decision
  duplicated across an alternation's two branches instead of living once at
  their shared join, and the start/exit-fusion mistakes above. Same workflow as
  Lesson 2, now applied to nested repetition.
- Reference docs are worth writing whenever a grammar gets fiddly enough that
  Victor doesn't yet know its own semantics ("I don't know what `^` does
  either" / "I don't know what this regex does either").
- Possessive quantifiers (`a*+`) were a red herring: PCRE's suffix meaning is
  architecturally impossible here (same invariant as backreferences), so `+`
  after any quantifier is just ordinary stacking — now in the decisions table.
- The loop is increasingly "raise the concern, let him find the shape of the
  fix" — held for the class-negation redesign and every trace bug above.

### Interlude — buffer reuse (complete)

A profiling side-quest partway through Lesson 5. `find` now threads two
long-lived `SeenSet`s through `step` and `closure` instead of allocating per
character, resolving the `clist`/`nlist` open item Lessons 3/4 deferred.
`match` on a 10-char input went 468 ns → 178 ns across two fixes; benchmark
harness lives in [benches/match_bench.rs](benches/match_bench.rs).

**The one engine-level lesson:** `mem::take` on a buffer field is a trap. It
hands back the old `Vec` but leaves a **zero-capacity** `Default` in its place,
so the next push reallocates and the taken `Vec` is freed when the loop that
consumes it ends — reintroducing exactly the per-character allocation the
refactor was removing, just relocated. The fix is a second, independently-owned
buffer to `mem::swap` with, and iterating the seed list **by reference** so
nothing is dropped. (`Vec::clear()` was never the problem: it resets `len` and
runs drop glue, but keeps the allocation.)

**Teaching notes:**

- Victor spotted the profiling opportunity himself and diagnosed the *shape* of
  both bugs correctly before being told the mechanism — he knew something still
  allocated, and knew roughly where to look, even though his specific guess
  (`clear()`) was wrong. The tutoring move that worked: confirm the symptom is
  real, rule out the wrong cause with a one-line "here's what that function
  actually does," then point at the real function and describe its lifecycle
  rather than diffing it for him.
- First time a benchmark, not a hand-trace, drove a design decision. Worth
  noticing if it becomes a pattern: hand-traces have caught every *correctness*
  bug so far; this was the first *performance* bug, and it needed a different
  tool because "does it produce the right `Some(n)`" and "does it allocate" are
  orthogonal questions — the whole suite passed against both the slow and the
  fast version.
- A flamegraph names whichever function actually does the work, which is not
  always the function you wrote: the matcher's small helpers inline away
  entirely and their cost shows up under `RawVec`/`malloc`, while the parser's
  recursive functions survive as named frames because recursion blocks inlining.

### Lesson 6 — `\d`/`\w`/`\s` shorthand, `\n`/`\t` escapes (next)

Pure sugar, no new AST variants: new arms in the existing `\` branch of
`parse_atom`. `\d`/`\w`/`\s` (and their negated uppercase forms) desugar to
`Ast::Class`; `\n`/`\t` desugar to `Ast::Literal` holding the real control
character. Nothing in `machine/` or `regex/` needs to change.

### Lesson 7 — search, spans, captures (next)

One subject, not three: unanchored search, `(start, end)` spans, and capture
groups all need threads to carry a start offset, not just an end one.

- Unanchored search: `find` only answers "does the whole string match starting
  at 0." A restart loop over offsets is `O(n²·m)` and needs the empty-match
  guard (`if end == start`, advance one) and a non-overlapping convention.
  Recovering linear time means an implicit low-priority `.*?` prefix on the
  pattern instead.
- That `.*?` prefix needs each thread to know *where it started* — a per-thread
  start offset, not one global one. Spans are the same requirement read
  differently: `(start, end)` instead of just `end`.
- Capture groups generalize the same idea: each thread carries a small array of
  offsets, one pair per group, instead of a single number.

## Open items

Not bugs to fix unprompted — raise them when relevant.

**Deliberately red tests.** `a_higher_priority_short_branch_must_not_hide_a_full_match`
(`a|ab` vs `"ab"`) and `lazy_quantifiers_expose_the_same_full_match_bug` (`a??`
vs `"a"`) both fail on purpose, same root cause: `full_match` is derived as
`find(input) == Some(len)`, but `find` applies the leftmost-**first** cut, so a
high-priority short path records early and kills the thread that would have
spanned the whole input. Deriving membership from a leftmost-first search is
only sound if the search were leftmost-**longest**. The fix is to ask the Lesson
3 question instead — consume the whole input with no early record and no cut,
then test whether `Match` is in the final closed set. Victor left this unfixed
to think about; **do not fix it unprompted.**

**Performance, all deferred until a benchmark complains.**

- Whether to cap the nested product of stacked bounded repetitions
  (`(a{500}){500}`) before it compiles to an enormous program. `u16` bounds
  each count individually, not the product.
- The stamp/generation trick: store *when* a state was last added and compare
  against the current position, instead of storing a `bool` and clearing `n` of
  them. `SeenSet::clear()` is still `O(n)` regardless of how few states were
  live. Buffer reuse fixed allocator traffic, not this constant.
- Sparse sets (RE2, Rust's `regex`) — a further step past stamp/generation:
  iteration proportional to the live set rather than to `program.len()`.
- `HashSet` for class membership was considered twice and rejected both times:
  real classes are small enough that a contiguous `Vec` scan beats hashing, and
  ranges can't live in a `HashSet<char>` without the expansion already rejected
  for `.`.

**Recursion depth.** `follow` recurses per ε-edge (bounded by `program.len()`)
and the parser recurses per input character. Production engines cap nesting
depth (`nest_limit`).

**Cosmetic.** `Class::match_c` has an unreachable `if !self.negated && matched
{ break; }` — the inner `break` in each arm already exits. Delete it whenever
that function is touched. ε-only `Jump` slots are kept deliberately: they make
the program match the hand-drawn diagrams and are removable later by a peephole
pass that doesn't touch `compile`.

## Protocol for agents

**When a lesson is completed, append its record to Progress.** What was built,
which files, concepts covered, bugs that surfaced, teaching notes. This file is
the context for the next session — a concept explained but unrecorded will have
to be re-explained from scratch.

Record the *reasoning* behind decisions, not just the decisions. But keep it
tight: this file is loaded every session, so it must stay a live reference, not
a changelog. When you add to it, also **remove** what your change made obsolete
— resolved open items get deleted, not struck through, and superseded
descriptions get rewritten rather than layered.
