pub struct Cursor {
    input: Vec<char>,
    pos: usize,
}

impl Cursor {
    pub fn new(input: &str) -> Self {
        Cursor {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    pub fn peek(&self) -> Option<char> {
        if self.pos >= self.input.len() {
            return None;
        }
        Some(self.input[self.pos])
    }

    #[allow(dead_code)]
    pub fn peek_at(&self, offset: usize) -> Option<char> {
        let pos = self.pos + offset;
        if pos >= self.input.len() {
            return None;
        }
        Some(self.input[pos])
    }

    #[allow(dead_code)]
    pub fn rewind(&mut self, steps: usize) {
        if steps > self.pos {
            self.pos = 0;
        } else {
            self.pos -= steps;
        }
    }

    #[allow(dead_code)]
    pub fn position(&self) -> usize {
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
