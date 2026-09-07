pub struct Cursor {
    input: Vec<char>,
    pos: usize,
}

pub struct OverFlowResult {
    pub result: Result<u16, ()>,
    pub offset: usize,
}

impl OverFlowResult {
    #[allow(dead_code)]
    pub fn is_err(&self) -> bool {
        return self.result.is_err();
    }

    #[allow(dead_code)]
    pub fn unwrap(&self) -> u16 {
        return *self.result.as_ref().unwrap();
    }
}

impl Cursor {
    pub fn new(input: &str) -> Self {
        Cursor {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    pub fn move_to(&mut self, offset: usize) {
        self.pos += offset;
    }

    pub fn peek(&self) -> Option<char> {
        if self.pos >= self.input.len() {
            return None;
        }
        Some(self.input[self.pos])
    }

    pub fn peek_at(&self, offset: usize) -> Option<char> {
        let pos = self.pos + offset;
        if pos >= self.input.len() {
            return None;
        }
        Some(self.input[pos])
    }

    #[allow(dead_code)]
    pub fn peek_number(&self) -> Option<OverFlowResult> {
        return self.peek_number_at(0);
    }

    pub fn string_offset(&self, limit: usize) -> String {
        let mut result = String::with_capacity(limit);
        let mut i = 0;
        while let Some(c) = self.peek_at(i) {
            if i >= limit {
                break;
            }
            result.push(c);
            i += 1;
        }
        return result;
    }

    pub fn peek_number_at(&self, offset: usize) -> Option<OverFlowResult> {
        let at_digit = self.at_digit(offset);
        if !at_digit {
            return None;
        }
        let mut i = offset;
        let mut result: u16 = 0;
        let mut error_state = false;
        while let Some(c) = self.peek_at(i) {
            let n = match c.to_digit(10) {
                None => break,
                Some(n) => n as u16,
            };
            result = match result.checked_mul(10) {
                None => {
                    error_state = true;
                    0
                }
                Some(r) => r,
            };
            result = match result.checked_add(n) {
                None => {
                    error_state = true;
                    0
                }
                Some(r) => r,
            };
            i += 1;
        }
        if error_state {
            return Some(OverFlowResult {
                result: Err(()),
                offset: i,
            });
        }
        return Some(OverFlowResult {
            result: Ok(result),
            offset: i,
        });
    }

    #[allow(dead_code)]
    pub fn next_number(&mut self) -> Option<OverFlowResult> {
        let result = match self.peek_number() {
            None => return None,
            Some(r) => r,
        };

        self.pos = result.offset;
        Some(result)
    }

    pub fn offset_whitespace_at(&self, offset: usize) -> usize {
        let mut i = offset;
        while let Some(c) = self.peek_at(i) {
            if !c.is_whitespace() {
                break;
            }
            i += 1;
        }
        return i;
    }

    #[allow(dead_code)]
    pub fn rewind(&mut self, steps: usize) {
        if steps > self.pos {
            self.pos = 0;
        } else {
            self.pos -= steps;
        }
    }

    pub fn offset(&self) -> usize {
        self.pos
    }

    pub fn is_eof(&self) -> bool {
        return self.peek().is_none();
    }

    pub fn eat(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.next();
            return true;
        }
        false
    }

    fn at_digit(&self, offset: usize) -> bool {
        match self.peek_at(offset) {
            None => return false,
            Some(c) => return c.is_numeric(),
        }
    }
}

impl Iterator for Cursor {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.input.len() {
            return None;
        }
        let c = self.input[self.pos];
        self.pos += 1;
        Some(c)
    }
}

#[cfg(test)]
mod test {

    use crate::cursor::Cursor;

    #[test]
    fn test_peek_number() {
        let input = "1234";
        let cursor = Cursor::new(input);
        let result = cursor.peek_number().unwrap();
        assert_eq!(result.unwrap(), 1234);
        assert_eq!(result.offset, 4);
    }

    #[test]
    fn test_peek_number_overflow() {
        let input = "655351a";
        let cursor = Cursor::new(input);
        let result = cursor.peek_number();
        assert!(result.is_some());
        let overflow_result = result.unwrap();
        assert!(overflow_result.is_err());
        let offset = overflow_result.offset;
        let a = cursor.peek_at(offset).unwrap();
        assert_eq!(a, 'a')
    }

    #[test]
    fn test_peek_number_with_char() {
        let input = "12a34";
        let cursor = Cursor::new(input);
        let result = cursor.peek_number().unwrap();
        assert_eq!(result.unwrap(), 12);
        assert_eq!(result.offset, 2);
        let a = cursor.peek_at(result.offset).unwrap();
        assert_eq!(a, 'a')
    }

    #[test]
    fn test_peek_number_no_number() {
        let input = "a12a34";
        let cursor = Cursor::new(input);
        assert!(cursor.peek_number().is_none())
    }

    #[test]
    fn test_next_number() {
        let input = "1234";
        let mut cursor = Cursor::new(input);
        let num = cursor.next_number().unwrap().unwrap();
        assert_eq!(num, 1234);
        assert!(cursor.next().is_none());
    }

    #[test]
    fn test_next_number_with_char() {
        let input = "12a34";
        let mut cursor = Cursor::new(input);
        let num = cursor.next_number().unwrap().unwrap();
        assert_eq!(num, 12);
        let a = cursor.next().unwrap();
        assert_eq!(a, 'a')
    }

    #[test]
    fn test_next_number_no_number() {
        let input = "a12a34";
        let mut cursor = Cursor::new(input);
        assert!(cursor.next_number().is_none());
        let a = cursor.next().unwrap();
        assert_eq!(a, 'a')
    }

    #[test]
    fn test_next_number_number_after_advance() {
        let input = "a12a34";
        let mut cursor = Cursor::new(input);
        cursor.next();
        let result = cursor.peek_number().unwrap();
        assert_eq!(result.unwrap(), 12);
        assert_eq!(result.offset, 2);
    }
}
