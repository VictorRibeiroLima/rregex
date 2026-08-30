use crate::{
    machine::class::{Class, ClassInstruction},
    parser::ast::{Ast, ClassType},
};
use program::{Instruction as Inst, Program, ValidInstruction, ValidProgram};

mod class;
mod program;

pub type State = usize;
pub type Instruction = ValidInstruction;

struct Fragment {
    start: State,
    exit: State,
}

pub struct Machine {
    start: State,
    program: ValidProgram,
}

impl Machine {
    pub fn new(ast: Ast) -> Self {
        let mut program = Program::new();
        let fragment = compile_fragment(&ast, &mut program);
        program[fragment.exit] = Inst::Match;

        let program = ValidProgram::new(program).expect("Program contains a hole");
        Self {
            start: fragment.start,
            program,
        }
    }

    pub fn start(&self) -> State {
        self.start
    }

    pub fn program(&self) -> &ValidProgram {
        &self.program
    }
}

fn compile_fragment(ast: &Ast, program: &mut Program) -> Fragment {
    match ast {
        Ast::Empty => compile_empty(program),
        Ast::Literal(c) => compile_literal(*c, program),
        Ast::Concat(left, right) => compile_concat(left, right, program),
        Ast::Alternation(left, right) => compile_alternation(left, right, program),
        Ast::Star(ast) => compile_star(ast, program),
        Ast::LazyStar(ast) => compile_lazy_star(ast, program),
        Ast::Plus(ast) => compile_plus(ast, program),
        Ast::LazyPlus(ast) => compile_lazy_plus(ast, program),
        Ast::Question(ast) => compile_question(ast, program),
        Ast::LazyQuestion(ast) => compile_lazy_question(ast, program),
        Ast::Class(c, negated) => compile_class(c, *negated, program),
        Ast::Any => compile_any(program),
    }
}

fn compile_empty(program: &mut Program) -> Fragment {
    let exit = program.len();
    program.push(Inst::Hole);
    Fragment { start: exit, exit }
}

fn compile_literal(c: char, program: &mut Program) -> Fragment {
    let start = program.len();
    program.push(Inst::Consume(c, start + 1));
    let exit = program.len();
    program.push(Inst::Hole);
    Fragment { start, exit }
}

fn compile_class(c: &Vec<ClassType>, negated: bool, program: &mut Program) -> Fragment {
    let start = program.len();
    let instructions: Vec<ClassInstruction> = c.iter().map(|class| class.into()).collect();
    let exit = start + 1;
    let class = Class::new(instructions, negated, exit);

    program.push(Inst::ConsumeClass(class));
    program.push(Inst::Hole);
    Fragment { start, exit }
}

fn compile_concat(left: &Ast, right: &Ast, program: &mut Program) -> Fragment {
    let left = compile_fragment(left, program);
    let right = compile_fragment(right, program);
    program[left.exit] = Inst::Jump(right.start);
    Fragment {
        start: left.start,
        exit: right.exit,
    }
}

fn compile_alternation(left: &Ast, right: &Ast, program: &mut Program) -> Fragment {
    let left = compile_fragment(left, program);
    let right = compile_fragment(right, program);

    program.push(Inst::Split(left.start, right.start));
    let alt_start = program.len() - 1;
    program.push(Inst::Hole); //Here is where The "exit" will live
    let alt_exit = program.len() - 1;

    program[left.exit] = Inst::Jump(alt_exit);
    program[right.exit] = Inst::Jump(alt_exit);

    Fragment {
        start: alt_start,
        exit: alt_exit,
    }
}

fn compile_star(ast: &Ast, program: &mut Program) -> Fragment {
    let frag = compile_fragment(ast, program);

    /*
    "a*"
    programLen = 0
    (0) --a---> (1)
    programLen = 2
    program[2] = Split(0,3)
    program[3]= Hole

    (2) -> (0)
    (2) -> (3)
    (1) -> (2)

    start=2 exit =3
     */
    let start = program.len();
    let exit = start + 1;

    program[frag.exit] = Inst::Jump(start);
    program.push(Inst::Split(frag.start, exit));
    program.push(Inst::Hole);

    Fragment { start, exit }
}

fn compile_lazy_star(ast: &Ast, program: &mut Program) -> Fragment {
    //same as compile_star but the split is reversed
    let frag = compile_fragment(ast, program);
    let start = program.len();
    let exit = start + 1;

    program[frag.exit] = Inst::Jump(start);
    program.push(Inst::Split(exit, frag.start));
    program.push(Inst::Hole);

    Fragment { start, exit }
}

fn compile_plus(ast: &Ast, program: &mut Program) -> Fragment {
    let frag = compile_fragment(ast, program);
    /*
    "a+"
    programLen = 0
    (0) --a---> (1)
    programLen = 2
    program[1] = Split(0,2)
    program[2]= Hole

    (1) -> (0)
    (1) -> (2)

    start=0 exit =2
     */
    let start = frag.start;
    let exit = program.len();
    program[frag.exit] = Inst::Split(start, exit);
    program.push(Inst::Hole);

    Fragment { start, exit }
}

fn compile_lazy_plus(ast: &Ast, program: &mut Program) -> Fragment {
    //same as compile_plus but the split is reversed
    let frag = compile_fragment(ast, program);
    let start = frag.start;
    let exit = program.len();
    program[frag.exit] = Inst::Split(exit, start);
    program.push(Inst::Hole);

    Fragment { start, exit }
}

fn compile_question(ast: &Ast, program: &mut Program) -> Fragment {
    let frag = compile_fragment(ast, program);
    /*
    "a?"
    programLen = 0
    (0) --a---> (1)
    programLen = 2

    program[2] = Split(0,3)
    program[3]= Hole

    (2) -> (0)
    (2) -> (3)
    (1) -> (3)

    start=2 exit =3
     */
    let start = program.len();
    let exit = frag.exit;

    program.push(Inst::Split(frag.start, exit));
    Fragment { start, exit }
}

fn compile_lazy_question(ast: &Ast, program: &mut Program) -> Fragment {
    //same as compile_question but the split is reversed
    let frag = compile_fragment(ast, program);
    let start = program.len();
    let exit = frag.exit;

    program.push(Inst::Split(exit, frag.start));
    Fragment { start, exit }
}

fn compile_any(program: &mut Program) -> Fragment {
    let start = program.len();
    program.push(Inst::ConsumeAny(start + 1));
    let exit = program.len();
    program.push(Inst::Hole);
    Fragment { start, exit }
}

#[cfg(test)]
mod test {
    use crate::{
        machine::{
            Machine, ValidInstruction,
            class::{Class, ClassInstruction},
        },
        parser::{ast::Ast, parse},
    };

    #[test]
    fn empty_regex() {
        // A lone `Empty` is one slot: the hole IS both the start and the exit,
        // so `Machine::new` overwrites it with `Match` and the whole program is
        // a single instruction that matches "" and nothing else.
        let ast = parse("").unwrap();
        let machine = Machine::new(ast);
        assert_eq!(machine.start, 0);
        assert_eq!(machine.program.len(), 1);
        assert_eq!(machine.program[0], ValidInstruction::Match);
    }

    #[test]
    fn simple_concat_regex() {
        let regex = "ab";
        let ast = parse(regex).unwrap();
        let machine = Machine::new(ast);
        assert_eq!(machine.start, 0);
        assert_eq!(machine.program.len(), 4);
        assert_eq!(machine.program[0], ValidInstruction::Consume('a', 1));
        assert_eq!(machine.program[1], ValidInstruction::Jump(2));
        assert_eq!(machine.program[2], ValidInstruction::Consume('b', 3));
        assert_eq!(machine.program[3], ValidInstruction::Match);
    }

    #[test]
    fn dot_regex() {
        // "." compiles to exactly one ConsumeAny, same shape as a Literal's
        // Consume: push the instruction, push a Hole for the exit.
        let ast = parse(".").unwrap();
        let machine = Machine::new(ast);
        assert_eq!(machine.start, 0);
        assert_eq!(machine.program.len(), 2);
        assert_eq!(machine.program[0], ValidInstruction::ConsumeAny(1));
        assert_eq!(machine.program[1], ValidInstruction::Match);
    }

    #[test]
    fn dot_after_literal() {
        // "a." -- same seam-wiring as "ab" (simple_concat_regex), just with
        // the second atom compiled by compile_any instead of compile_literal.
        let ast = parse("a.").unwrap();
        let machine = Machine::new(ast);
        assert_eq!(machine.start, 0);
        assert_eq!(machine.program.len(), 4);
        assert_eq!(machine.program[0], ValidInstruction::Consume('a', 1));
        assert_eq!(machine.program[1], ValidInstruction::Jump(2));
        assert_eq!(machine.program[2], ValidInstruction::ConsumeAny(3));
        assert_eq!(machine.program[3], ValidInstruction::Match);
    }

    #[test]
    fn simple_alt_regex() {
        /*
          the regex:
        "a|b"

        arrayLen =0

        (0) --a--> (1)   arrayLen = 2
        (2) --b--> (3)   arrayLen = 4

        push a split instruction arrayLen = 5
        push a exit arrayLen = 6

        (4)   ⇢    (0)
        (4)   ⇢    (2)
        (1)   ⇢    (5)
        (3)   ⇢    (5)

        start = (4)     exit = (5) */
        let regex = "a|b";
        let ast = parse(regex).unwrap();
        let machine = Machine::new(ast);
        assert_eq!(machine.start, 4);
        assert_eq!(machine.program.len(), 6);
        assert_eq!(machine.program[0], ValidInstruction::Consume('a', 1));
        assert_eq!(machine.program[1], ValidInstruction::Jump(5));
        assert_eq!(machine.program[2], ValidInstruction::Consume('b', 3));
        assert_eq!(machine.program[3], ValidInstruction::Jump(5));
        assert_eq!(machine.program[4], ValidInstruction::Split(0, 2));
        assert_eq!(machine.program[5], ValidInstruction::Match);
    }

    #[test]
    fn empty_as_an_alternation_branch() {
        /* the regex: "a|"   ->  Alternation(Literal('a'), Empty)

        compile 'a'      0: Consume('a', 1)   1: Hole      frag = (0, 1)
        compile Empty    2: Hole                           frag = (2, 2)
        push the split   3: Split(0, 2)
        push the exit    4: Hole
        fill both holes  1 -> Jump(4)   2 -> Jump(4)
        top level        4: Match

        Slot 2 is the whole of the empty branch: entering it and leaving it
        are the same act. */
        let ast = parse("a|").unwrap();
        let machine = Machine::new(ast);
        assert_eq!(machine.start, 3);
        assert_eq!(machine.program.len(), 5);
        assert_eq!(machine.program[0], ValidInstruction::Consume('a', 1));
        assert_eq!(machine.program[1], ValidInstruction::Jump(4));
        assert_eq!(machine.program[2], ValidInstruction::Jump(4));
        assert_eq!(machine.program[3], ValidInstruction::Split(0, 2));
        assert_eq!(machine.program[4], ValidInstruction::Match);
    }

    #[test]
    fn empty_on_the_left_of_a_seam() {
        /* The parser never builds `Concat(Empty, _)`, so this AST is written by
        hand. It is the case that proves start == exit is safe: the concat
        fills the empty fragment's exit, and that same slot is the fragment's
        entry point, so control flows straight into 'a'.

        compile Empty    0: Hole                           frag = (0, 0)
        compile 'a'      1: Consume('a', 2)   2: Hole      frag = (1, 2)
        seam             0 -> Jump(1)
        top level        2: Match                          frag = (0, 2) */
        let ast = Ast::Concat(Box::new(Ast::Empty), Box::new(Ast::Literal('a')));
        let machine = Machine::new(ast);
        assert_eq!(machine.start, 0);
        assert_eq!(machine.program.len(), 3);
        assert_eq!(machine.program[0], ValidInstruction::Jump(1));
        assert_eq!(machine.program[1], ValidInstruction::Consume('a', 2));
        assert_eq!(machine.program[2], ValidInstruction::Match);
    }

    #[test]
    fn simple_star_regex() {
        /*
        "a*"
        programLen = 0
        (0) --a---> (1)
        programLen = 2
        program[2] = Split(0,3)
        program[3]= Hole

        (2) -> (0)
        (2) -> (3)
        (1) -> (2)

        start=2 exit =3
         */
        let ast = parse("a*").unwrap();
        let machine = Machine::new(ast);
        assert_eq!(machine.start, 2);
        assert_eq!(machine.program.len(), 4);
        assert_eq!(machine.program[0], ValidInstruction::Consume('a', 1));
        assert_eq!(machine.program[1], ValidInstruction::Jump(2));
        assert_eq!(machine.program[2], ValidInstruction::Split(0, 3));
        assert_eq!(machine.program[3], ValidInstruction::Match);
    }

    #[test]
    fn star_of_an_alternation_then_a_literal() {
        /* the regex: "(a|b)*c"  ->  Concat(Star(Alternation('a','b')), 'c')

        This is the machine drawn by hand in Lesson 2, Exercise 4 — ten states,
        same shape, now produced by the compiler. The parens left no trace.

        compile 'a'        0: Consume('a', 1)   1: Hole        frag = (0, 1)
        compile 'b'        2: Consume('b', 3)   3: Hole        frag = (2, 3)
        alternation        4: Split(0, 2)       5: Hole
                           1 -> Jump(5)         3 -> Jump(5)   frag = (4, 5)
        star               6: Split(4, 7)       7: Hole
                           5 -> Jump(6)                        frag = (6, 7)
        compile 'c'        8: Consume('c', 9)   9: Hole        frag = (8, 9)
        concat seam        7 -> Jump(8)                        frag = (6, 9)
        top level          9: Match                                          */
        let ast = parse("(a|b)*c").unwrap();
        let machine = Machine::new(ast);
        assert_eq!(machine.start, 6);
        assert_eq!(machine.program.len(), 10);
        assert_eq!(machine.program[0], ValidInstruction::Consume('a', 1));
        assert_eq!(machine.program[1], ValidInstruction::Jump(5));
        assert_eq!(machine.program[2], ValidInstruction::Consume('b', 3));
        assert_eq!(machine.program[3], ValidInstruction::Jump(5));
        assert_eq!(machine.program[4], ValidInstruction::Split(0, 2));
        assert_eq!(machine.program[5], ValidInstruction::Jump(6));
        assert_eq!(machine.program[6], ValidInstruction::Split(4, 7));
        assert_eq!(machine.program[7], ValidInstruction::Jump(8));
        assert_eq!(machine.program[8], ValidInstruction::Consume('c', 9));
        assert_eq!(machine.program[9], ValidInstruction::Match);
    }

    #[test]
    fn simple_plus_regex() {
        /*
        "a+"
        programLen = 0
        (0) --a---> (1)
        programLen = 2
        program[1] = Split(0,2)   -- fills 'a's own exit hole, no extra slot
        program[2] = Hole

        (1) -> (0)
        (1) -> (2)

        start=0 exit=2
         */
        let ast = parse("a+").unwrap();
        let machine = Machine::new(ast);
        assert_eq!(machine.start, 0);
        assert_eq!(machine.program.len(), 3);
        assert_eq!(machine.program[0], ValidInstruction::Consume('a', 1));
        assert_eq!(machine.program[1], ValidInstruction::Split(0, 2));
        assert_eq!(machine.program[2], ValidInstruction::Match);
    }

    #[test]
    fn plus_then_a_literal() {
        /* the regex: "a+b"  ->  Concat(Plus(Literal('a')), Literal('b'))

        Same seam-wiring as simple_concat_regex, but the left fragment is now
        a Plus instead of a bare Literal -- proves compile_concat doesn't care
        what produced left.exit, only that it's a hole to fill.

        compile 'a'    0: Consume('a', 1)   1: Hole            frag = (0, 1)
        plus           1: Split(0, 2)       2: Hole            frag = (0, 2)
        compile 'b'    3: Consume('b', 4)   4: Hole            frag = (3, 4)
        concat seam    2 -> Jump(3)                             frag = (0, 4)
        top level      4: Match                                              */
        let ast = parse("a+b").unwrap();
        let machine = Machine::new(ast);
        assert_eq!(machine.start, 0);
        assert_eq!(machine.program.len(), 5);
        assert_eq!(machine.program[0], ValidInstruction::Consume('a', 1));
        assert_eq!(machine.program[1], ValidInstruction::Split(0, 2));
        assert_eq!(machine.program[2], ValidInstruction::Jump(3));
        assert_eq!(machine.program[3], ValidInstruction::Consume('b', 4));
        assert_eq!(machine.program[4], ValidInstruction::Match);
    }

    #[test]
    fn plus_of_an_alternation() {
        /* the regex: "(a|b)+"  ->  Plus(Alternation('a','b'))

        Proves compile_plus only ever touches frag.start/frag.exit as two
        integers -- it works the same whether the child is a bare Literal or
        a whole Alternation subtree underneath it.

        compile 'a'    0: Consume('a', 1)   1: Hole              frag = (0, 1)
        compile 'b'    2: Consume('b', 3)   3: Hole              frag = (2, 3)
        alternation    4: Split(0, 2)       5: Hole
                       1 -> Jump(5)         3 -> Jump(5)         frag = (4, 5)
        plus           5: Split(4, 6)       6: Hole              frag = (4, 6)
        top level      6: Match                                                */
        let ast = parse("(a|b)+").unwrap();
        let machine = Machine::new(ast);
        assert_eq!(machine.start, 4);
        assert_eq!(machine.program.len(), 7);
        assert_eq!(machine.program[0], ValidInstruction::Consume('a', 1));
        assert_eq!(machine.program[1], ValidInstruction::Jump(5));
        assert_eq!(machine.program[2], ValidInstruction::Consume('b', 3));
        assert_eq!(machine.program[3], ValidInstruction::Jump(5));
        assert_eq!(machine.program[4], ValidInstruction::Split(0, 2));
        assert_eq!(machine.program[5], ValidInstruction::Split(4, 6));
        assert_eq!(machine.program[6], ValidInstruction::Match);
    }

    #[test]
    fn simple_question_regex() {
        let ast = parse("a?").unwrap();
        let machine = Machine::new(ast);
        assert_eq!(machine.start, 2);
        assert_eq!(machine.program.len(), 3);
        assert_eq!(machine.program[0], ValidInstruction::Consume('a', 1));
        assert_eq!(machine.program[1], ValidInstruction::Match);
        assert_eq!(machine.program[2], ValidInstruction::Split(0, 1));
    }

    #[test]
    fn question_then_a_literal() {
        /* the regex: "a?b"  ->  Concat(Question(Literal('a')), Literal('b'))

        compile 'a'    0: Consume('a', 1)   1: Hole            frag = (0, 1)
        question       2: Split(0, 1)                          frag = (2, 1)
        compile 'b'    3: Consume('b', 4)   4: Hole            frag = (3, 4)
        concat seam    1 -> Jump(3)                            frag = (2, 4)
        top level      4: Match                                            */
        let ast = parse("a?b").unwrap();
        let machine = Machine::new(ast);
        assert_eq!(machine.start, 2);
        assert_eq!(machine.program.len(), 5);
        assert_eq!(machine.program[0], ValidInstruction::Consume('a', 1));
        assert_eq!(machine.program[1], ValidInstruction::Jump(3));
        assert_eq!(machine.program[2], ValidInstruction::Split(0, 1));
        assert_eq!(machine.program[3], ValidInstruction::Consume('b', 4));
        assert_eq!(machine.program[4], ValidInstruction::Match);
    }

    #[test]
    fn question_of_an_alternation() {
        /* the regex: "(a|b)?"  ->  Question(Alternation('a','b'))

        compile 'a'    0: Consume('a', 1)   1: Hole              frag = (0, 1)
        compile 'b'    2: Consume('b', 3)   3: Hole              frag = (2, 3)
        alternation    4: Split(0, 2)       5: Hole
                       1 -> Jump(5)         3 -> Jump(5)         frag = (4, 5)
        question       6: Split(4, 5)                            frag = (6, 5)
        top level      5: Match                                                */
        let ast = parse("(a|b)?").unwrap();
        let machine = Machine::new(ast);
        assert_eq!(machine.start, 6);
        assert_eq!(machine.program.len(), 7);
        assert_eq!(machine.program[0], ValidInstruction::Consume('a', 1));
        assert_eq!(machine.program[1], ValidInstruction::Jump(5));
        assert_eq!(machine.program[2], ValidInstruction::Consume('b', 3));
        assert_eq!(machine.program[3], ValidInstruction::Jump(5));
        assert_eq!(machine.program[4], ValidInstruction::Split(0, 2));
        assert_eq!(machine.program[5], ValidInstruction::Match);
        assert_eq!(machine.program[6], ValidInstruction::Split(4, 5));
    }

    #[test]
    fn simple_lazy_star_regex() {
        // "a*?" -- identical to simple_star_regex except the Split's two
        // targets are swapped: (3, 0) instead of (0, 3). Same states, same
        // language, different priority order.
        let ast = parse("a*?").unwrap();
        let machine = Machine::new(ast);
        assert_eq!(machine.start, 2);
        assert_eq!(machine.program.len(), 4);
        assert_eq!(machine.program[0], ValidInstruction::Consume('a', 1));
        assert_eq!(machine.program[1], ValidInstruction::Jump(2));
        assert_eq!(machine.program[2], ValidInstruction::Split(3, 0));
        assert_eq!(machine.program[3], ValidInstruction::Match);
    }

    #[test]
    fn simple_lazy_plus_regex() {
        // "a+?" -- identical to simple_plus_regex except Split(0, 2) becomes
        // Split(2, 0). Entry is still frag.start (0): laziness only changes
        // what's preferred AFTER the mandatory first 'a', never whether it's
        // mandatory.
        let ast = parse("a+?").unwrap();
        let machine = Machine::new(ast);
        assert_eq!(machine.start, 0);
        assert_eq!(machine.program.len(), 3);
        assert_eq!(machine.program[0], ValidInstruction::Consume('a', 1));
        assert_eq!(machine.program[1], ValidInstruction::Split(2, 0));
        assert_eq!(machine.program[2], ValidInstruction::Match);
    }

    #[test]
    fn simple_lazy_question_regex() {
        // "a??" -- identical to simple_question_regex except Split(0, 1)
        // becomes Split(1, 0). Same reused exit hole, same new entry slot.
        let ast = parse("a??").unwrap();
        let machine = Machine::new(ast);
        assert_eq!(machine.start, 2);
        assert_eq!(machine.program.len(), 3);
        assert_eq!(machine.program[0], ValidInstruction::Consume('a', 1));
        assert_eq!(machine.program[1], ValidInstruction::Match);
        assert_eq!(machine.program[2], ValidInstruction::Split(1, 0));
    }

    #[test]
    fn simple_class_regex() {
        // "[abc]" -- one Class instruction plus the shared Hole/exit, same
        // shape as compile_literal and compile_any: one atom, one slot to
        // consume, one slot for whoever comes next.
        let ast = parse("[abc]").unwrap();
        let machine = Machine::new(ast);
        let expected = Class::new(
            vec![
                ClassInstruction::Single('a'),
                ClassInstruction::Single('b'),
                ClassInstruction::Single('c'),
            ],
            false,
            1,
        );
        assert_eq!(machine.start, 0);
        assert_eq!(machine.program.len(), 2);
        assert_eq!(machine.program[0], ValidInstruction::ConsumeClass(expected));
        assert_eq!(machine.program[1], ValidInstruction::Match);
    }

    #[test]
    fn class_of_a_range() {
        let ast = parse("[a-z]").unwrap();
        let machine = Machine::new(ast);
        let expected = Class::new(vec![ClassInstruction::Range('a', 'z')], false, 1);
        assert_eq!(machine.start, 0);
        assert_eq!(machine.program.len(), 2);
        assert_eq!(machine.program[0], ValidInstruction::ConsumeClass(expected));
        assert_eq!(machine.program[1], ValidInstruction::Match);
    }

    #[test]
    fn negated_class_regex() {
        let ast = parse("[^a]").unwrap();
        let machine = Machine::new(ast);
        let expected = Class::new(vec![ClassInstruction::Single('a')], true, 1);
        assert_eq!(machine.start, 0);
        assert_eq!(machine.program.len(), 2);
        assert_eq!(machine.program[0], ValidInstruction::ConsumeClass(expected));
        assert_eq!(machine.program[1], ValidInstruction::Match);
    }

    #[test]
    fn class_then_a_literal() {
        // "[ab]c" -- same seam-wiring as simple_concat_regex: compile_concat
        // only ever reads left.exit, it doesn't care that this fragment came
        // from compile_class instead of compile_literal.
        let ast = parse("[ab]c").unwrap();
        let machine = Machine::new(ast);
        let expected = Class::new(
            vec![ClassInstruction::Single('a'), ClassInstruction::Single('b')],
            false,
            1,
        );
        assert_eq!(machine.start, 0);
        assert_eq!(machine.program.len(), 4);
        assert_eq!(machine.program[0], ValidInstruction::ConsumeClass(expected));
        assert_eq!(machine.program[1], ValidInstruction::Jump(2));
        assert_eq!(machine.program[2], ValidInstruction::Consume('c', 3));
        assert_eq!(machine.program[3], ValidInstruction::Match);
    }

    #[test]
    fn class_composes_with_plus() {
        // "[abc]+" -- compile_plus doesn't know or care that its child is a
        // Class fragment instead of a Literal one; it only touches
        // frag.start/frag.exit as two integers, same proof as
        // plus_of_an_alternation.
        let ast = parse("[abc]+").unwrap();
        let machine = Machine::new(ast);
        let expected = Class::new(
            vec![
                ClassInstruction::Single('a'),
                ClassInstruction::Single('b'),
                ClassInstruction::Single('c'),
            ],
            false,
            1,
        );
        assert_eq!(machine.start, 0);
        assert_eq!(machine.program.len(), 3);
        assert_eq!(machine.program[0], ValidInstruction::ConsumeClass(expected));
        assert_eq!(machine.program[1], ValidInstruction::Split(0, 2));
        assert_eq!(machine.program[2], ValidInstruction::Match);
    }

    #[test]
    fn escaped_literal_compiles_like_any_other_literal() {
        // "\*" and a bare literal 'a' produce the identical shape --
        // compile_literal can't tell, and doesn't need to, whether the parser
        // got this char via an escape. The distinction is fully absorbed by
        // the time the AST exists.
        let ast = parse("\\*").unwrap();
        let machine = Machine::new(ast);
        assert_eq!(machine.start, 0);
        assert_eq!(machine.program.len(), 2);
        assert_eq!(machine.program[0], ValidInstruction::Consume('*', 1));
        assert_eq!(machine.program[1], ValidInstruction::Match);
    }

    #[test]
    fn stacked_stars_build_an_epsilon_loop() {
        /* the regex: "a**"  ->  Star(Star(Literal('a')))

        This MUST compile. The construction is local and correct; nothing in it
        is allowed to reject a well-formed tree.

        But look at slots 2, 3 and 4:

            4: Split(2, 5)   ->  2
            2: Split(0, 3)   ->  3
            3: Jump(4)       ->  4

        Not one of those three advances the input, so 4 -> 2 -> 3 -> 4 is a
        cycle a finger can walk forever while reading nothing. A matcher that
        chases epsilon edges without remembering where it has already been will
        hang here on every input, including "".

        That is the matcher's problem, not the compiler's, and the fix is to
        track the set of states already added at the current input position.  */
        let ast = parse("a**").unwrap();
        let machine = Machine::new(ast);
        assert_eq!(machine.start, 4);
        assert_eq!(machine.program.len(), 6);
        assert_eq!(machine.program[0], ValidInstruction::Consume('a', 1));
        assert_eq!(machine.program[1], ValidInstruction::Jump(2));
        assert_eq!(machine.program[2], ValidInstruction::Split(0, 3));
        assert_eq!(machine.program[3], ValidInstruction::Jump(4));
        assert_eq!(machine.program[4], ValidInstruction::Split(2, 5));
        assert_eq!(machine.program[5], ValidInstruction::Match);
    }
}
