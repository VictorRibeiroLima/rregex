use crate::regex::Regex;

mod cursor;
mod machine;
mod parser;
mod regex;

fn main() {
    let regex = Regex::compile("abc").unwrap();
    assert!(regex.matches("abc").unwrap());
    assert!(!regex.matches("abcd").unwrap());
    assert!(!regex.matches("ab").unwrap());
}
