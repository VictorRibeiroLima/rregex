use crate::regex::Regex;

mod cursor;
mod machine;
mod parser;
mod regex;

fn main() {
    let regex = Regex::compile("abc").unwrap();
    assert!(regex.full_match("abc").unwrap());
    assert!(!regex.full_match("abcd").unwrap());
    assert!(!regex.full_match("ab").unwrap());
}
