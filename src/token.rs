use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug,PartialEq)]
pub enum Token {
    Close(char),
    Number(i32),
    Open(char),
    Operator(char),
    Variable(String),
}

pub struct Tokenizer<'a> {
    chars: Peekable<Chars<'a>>,
}

impl<'a> Tokenizer<'a> {
    pub fn from_str(s: &'a str) -> Self {
        Self { chars: s.chars().peekable() }
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Token;
    fn next(&mut self) -> Option<Token> {
        while let Some(' ') = self.chars.peek() {
            self.chars.next();
        }
        if let Some(c) = self.chars.next() {
            let token = match c {
                '0'..='9' => {
                    let mut v = c.to_digit(10).unwrap() as i32;
                    while let Some(&c) = self.chars.peek() {
                        if !c.is_ascii_digit() { break; }
                        self.chars.next();
                        v = v * 10 + c.to_digit(10).unwrap() as i32;
                    }
                    Token::Number(v)
                },
                'a'..='z' => {
                    let mut v = c.to_string();
                    while let Some(&c) = self.chars.peek() {
                        if !c.is_ascii_lowercase() { break; }
                        self.chars.next();
                        v += &c.to_string();
                    }
                    Token::Variable(v)
                },
                '+'|'-'|'*'|'/' => Token::Operator(c),
                '(' => Token::Open(c),
                ')' => Token::Close(c),
                _ => panic!("Invalid character: '{c}'"),
            };
            Some(token)
        } else {
            None
        }
    }
}

#[test]
fn test_tokenizer() {
    use Token::*;
    assert_eq!(Tokenizer::from_str("").next(), None);
    assert_eq!(Tokenizer::from_str("x").next(), Some(Variable("x".to_string())));
    assert_eq!(Tokenizer::from_str("xy").next(), Some(Variable("xy".to_string())));
    assert_eq!(Tokenizer::from_str("123").next(), Some(Number(123)));
    assert_eq!(Tokenizer::from_str("+").next(), Some(Operator('+')));
    let ts: Vec<_> = Tokenizer::from_str("(+-*/a1)").collect();
    assert_eq!(ts, [Open('('), Operator('+'), Operator('-'), Operator('*'),
                    Operator('/'), Variable("a".into()), Number(1), Close(')')]);
    let ts: Vec<_> = Tokenizer::from_str("ab xy 12 34l").collect();
    assert_eq!(ts, [Variable("ab".into()), Variable("xy".into()),
                    Number(12), Number(34), Variable("l".into())]);
}

