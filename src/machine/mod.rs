use crate::parser::Ast::{self};
use program::{Instruction as Inst, Program, ValidInstruction, ValidProgram};

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

#[cfg(test)]
mod test {
    use crate::{
        machine::{Machine, ValidInstruction},
        parser::{Ast, parse},
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
