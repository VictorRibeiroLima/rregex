use crate::parser::ast::ClassType;

#[allow(non_upper_case_globals)]
pub const ESCAPED_d: [ClassType; 1] = [ClassType::Range('0', '9')];
pub const ESCAPED_D: [ClassType; 2] = [
    ClassType::Range('\0', '/'),
    ClassType::Range(':', '\u{10FFFF}'),
];

#[allow(non_upper_case_globals)]
pub const ESCAPED_w: [ClassType; 4] = [
    ClassType::Range('a', 'z'),
    ClassType::Range('A', 'Z'),
    ClassType::Range('0', '9'),
    ClassType::Single('_'),
];

pub const ESCAPED_W: [ClassType; 5] = [
    ClassType::Range('\0', '/'),
    ClassType::Range(':', '@'),
    ClassType::Range('[', '^'),
    ClassType::Range('{', '\u{10FFFF}'),
    ClassType::Single('`'),
];

#[allow(non_upper_case_globals)]
pub const ESCAPED_s: [ClassType; 2] = [ClassType::Range('\t', '\r'), ClassType::Single(' ')];
pub const ESCAPED_S: [ClassType; 3] = [
    ClassType::Range('\0', '\x08'),
    ClassType::Range('\x0E', '\x1F'),
    ClassType::Range('!', '\u{10FFFF}'),
];
