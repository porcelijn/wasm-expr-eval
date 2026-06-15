use std::iter::Peekable;
use std::str::Chars;
use wasmtime::*;

enum Expr {
    Add(Box<Expr>, Box<Term>),
    Subtract(Box<Expr>, Box<Term>),
    Term(Box<Term>),
}

impl Expr {
    fn parse(i: &mut Peekable<impl Iterator<Item = Token>>) -> Self {
        let mut term = Self::Term(Box::new(Term::parse(i)));
        loop {
            match i.peek() {
                Some(Token::Operator('+')) => {
                    i.next();
                    term = Self::Add(Box::new(term), Box::new(Term::parse(i)));
                },
                Some(Token::Operator('-')) => {
                    i.next();
                    term = Self::Subtract(Box::new(term), Box::new(Term::parse(i)));
                },
                _ => break,
            }
        }
        term
    }

    fn to_wat(&self) -> String {
        match &self {
            Self::Add(l, r) => format!("{}\n{}\ni32.add", l.to_wat(), r.to_wat()),
            Self::Subtract(l, r) => format!("{}\n{}\ni32.sub", l.to_wat(), r.to_wat()),
            Self::Term(t) => t.to_wat(),
        }
    }

    fn to_wasm(&self) -> Vec<u8> {
        match &self {
            Self::Add(l, r) => {
                let mut out = l.to_wasm();
                out.extend(&r.to_wasm());
                out.push(0x6a); // i32.add
                out
            },
            Self::Subtract(l, r) => {
                let mut out = l.to_wasm();
                out.extend(&r.to_wasm());
                out.push(0x6b); // i32.sub
                out
            },
            Self::Term(t) => t.to_wasm(),
        }
    }
}

enum Term {
    Multiply(Box<Term>, Box<Factor>),
    Divide(Box<Term>, Box<Factor>),
    Factor(Box<Factor>),
}

impl Term {
    fn parse(i: &mut Peekable<impl Iterator<Item = Token>>) -> Self {
        let mut factor = Self::Factor(Box::new(Factor::parse(i)));
        loop {
            match i.peek() {
                Some(Token::Operator('*')) => {
                    i.next();
                    factor = Self::Multiply(Box::new(factor),
                                            Box::new(Factor::parse(i)));
                },
                Some(Token::Operator('/')) => {
                    i.next();
                    factor = Self::Divide(Box::new(factor),
                                          Box::new(Factor::parse(i)));
                },
                _ => break,
            }
        }
        factor
    }

    fn to_wat(&self) -> String {
        match &self {
            Self::Multiply(l, r) => format!("{}\n{}\ni32.mul", l.to_wat(), r.to_wat()),
            Self::Divide(l, r) => format!("{}\n{}\ni32.div_u", l.to_wat(), r.to_wat()),
            Self::Factor(f) => f.to_wat(),
        }
    }

    fn to_wasm(&self) -> Vec<u8> {
        match &self {
            Self::Multiply(l, r) => {
                let mut out = l.to_wasm();
                out.extend(&r.to_wasm());
                out.push(0x6c); // i32.mul
                out
            },
            Self::Divide(l, r) => {
                let mut out = l.to_wasm();
                out.extend(&r.to_wasm());
                out.push(0x6e); // i32.div_u
                out
            },
            Self::Factor(f) => f.to_wasm(),
        }
    }
}

enum Factor {
    Const(i32),
    Param(String),
    Expr(Box<Expr>), // braced: ( Expr )
}

impl Factor {
    fn parse(i: &mut Peekable<impl Iterator<Item = Token>>) -> Self {
        let token = i.next().expect("out of tokens, exected Factor");
        match token {
            Token::Number(n) => Self::Const(n),
            Token::Variable(v) => Self::Param(v),
            Token::Open('(') => {
                let expr = Expr::parse(i);
                let c = i.next().expect("out of tokens, expected ')'");
                assert_eq!(c, Token::Close(')'));
                Self::Expr(Box::new(expr))
            },
            _ => panic!("Invalid token for Factor: '{token:?}'"),
        }
    }

    fn to_wat(&self) -> String {
        match &self {
            Self::Const(c) => format!("i32.const {c}"),
            Self::Param(p) => format!("local.get ${p}"),
            Self::Expr(e) => e.to_wat(),
        }
    }

    fn to_wasm(&self) -> Vec<u8> {
        match &self {
            Self::Const(c) => {
                let mut out = vec![0x41]; // i32.const
                write_leb128((*c).into(), &mut out);
                out
            },
            Self::Param(p) => {
                let mut out = vec![0x20]; // local.get
                assert_eq!(p, "x"); // FIXME
                out.push(0); // first param
                out
            },
            Self::Expr(e) => e.to_wasm(),
        }
    }
}

#[derive(Debug,PartialEq)]
enum Token {
    Number(i32),
    Operator(char),
    Open(char),
    Close(char),
    Variable(String),
}

struct Tokenizer<'a> {
    chars: Peekable<Chars<'a>>,
}

impl<'a> Tokenizer<'a> {
    fn from_str(s: &'a str) -> Self {
        Self { chars: s.chars().peekable() }
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Token;
    fn next(&mut self) -> Option<Token> {
        if let Some(c) = self.chars.next() {
            let token = match c {
                '0'..='9' => {
                    let mut v = c.to_digit(10).unwrap() as i32;
                    while let Some(&c) = self.chars.peek() {
                        if !('0'..='9').contains(&c) { break; }
                        self.chars.next();
                        v = v * 10 + c.to_digit(10).unwrap() as i32;
                    }
                    Token::Number(v)
                },
                'a'..='z' => {
                    let mut v = c.to_string();
                    while let Some(&c) = self.chars.peek() {
                        if !('a'..='z').contains(&c) { break; }
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
}

fn write_leb128(mut i: i128, out: &mut Vec<u8>) {
    const MORE: u8 = 1 << 7;
    const SIGN: u8 = 1 << 6;
    const DATA: u8 = !MORE;
    loop {
        let byte = (i & DATA as i128) as u8;
        let has_sign = (byte & SIGN) != 0;
        i >>= 7;
        if (i == 0 && !has_sign) || (i == -1 && has_sign) {
            out.push(byte);
            break;
        }
        out.push(byte | MORE);
    }
}

#[test]
fn test_write_leb128() {
    // examples from wikipedia:
    let mut out = Vec::new();
    write_leb128(624485, &mut out);
    assert_eq!(out, &[0xE5, 0x8E, 0x26]);
    out.clear();
    write_leb128(-123456, &mut out);
    assert_eq!(out, &[0xC0, 0xBB, 0x78]);
}

#[test]
fn test_to_wat() {
    let expected = r#"i32.const 1
i32.const 2
i32.add
local.get $x
i32.add"#;
    let expr = Expr::Add(
        Box::new(Expr::Add(
            Box::new(Expr::Term(Box::new(Term::Factor(Box::new(Factor::Const(
                1,
            )))))),
            Box::new(Term::Factor(Box::new(Factor::Const(2)))),
        )),
        Box::new(Term::Factor(Box::new(Factor::Param("x".to_string())))),
    );
    assert_eq!(expr.to_wat(), expected);
}


#[test]
fn test_parse() {
    fn compile(s: &str) -> String {
        let expr = Expr::parse(&mut Tokenizer::from_str(s).peekable());
        expr.to_wat().replace('\n', " ")
    }
    assert_eq!(compile("0001"), "i32.const 1");
    assert_eq!(compile("2+3"), "i32.const 2 i32.const 3 i32.add");
    assert_eq!(compile("2*3"), "i32.const 2 i32.const 3 i32.mul");
    assert_eq!(compile("1+2*3"), "i32.const 1 i32.const 2 i32.const 3 i32.mul i32.add");
    assert_eq!(compile("1*2+3"), "i32.const 1 i32.const 2 i32.mul i32.const 3 i32.add");
    assert_eq!(compile("(1+2)*3"), "i32.const 1 i32.const 2 i32.add i32.const 3 i32.mul");
    assert_eq!(compile("1+x"), "i32.const 1 local.get $x i32.add");
    assert_eq!(compile("123/x"), "i32.const 123 local.get $x i32.div_u");
    assert_eq!(compile("variable"), "local.get $variable");
}

#[test]
fn test_eval() {
    fn evaluate(s: &str) -> i32 {
        let expr = Expr::parse(&mut Tokenizer::from_str(s).peekable());
        eprintln!("{}", expr.to_wat());
        // eval_wat(&expr).expect("Failed to evaluate")
        eval_wasm(&expr).expect("Failed to evaluate")
    }
    assert_eq!(evaluate("1+2+3+4+5+6+7+8+9"), 45);
    assert_eq!(evaluate("1*2*3*4*5*6*7*8*9"), 362880);
    assert_eq!(evaluate("0*x"), 0);
    assert_eq!(evaluate("x-x"), 0);
    assert_eq!(evaluate("((x))"), 7);
    assert_eq!(evaluate("0-x"), -7);
    assert_eq!(evaluate("2+2*9/3-1"), 7);
    assert_eq!(evaluate("100"), 100);
    assert_eq!(evaluate("100/3"), 33);
}

fn add_fluff(expr_wat: &str) -> String {
    let mut wat = r#"
        (module
            (import "host" "log" (func $host_log (param i32)))
            (func (export "calc") (result i32)
                i32.const 123 ;; <-- Closure `param`
                call $host_log

                i32.const 7 ;; <-- WASM function param `$x`
                call $eval_expr
                return)
    "#.to_string();

    wat.push_str("(func $eval_expr (param $x i32) (result i32)");
    wat.push_str(expr_wat);
    wat.push_str(" return)");
    wat.push_str(")");
    wat
}

fn eval_wat(expr: &Expr) -> Result<i32> {
    let wat = add_fluff(&expr.to_wat()); // wrap raw WAT in Module

    let engine = Engine::default();
    let module = Module::new(&engine, wat)?;

    type HostData = (u32,);
    let mut store = Store::new(&engine, (42,));
    let host_log = Func::wrap(&mut store, |caller: Caller<'_, HostData>, param: i32| {
        assert_eq!(param, 123);
        // eprintln!("WebAssembly param: {}", param);
        assert_eq!(*caller.data(), (42,));
        // eprintln!("HostData state is: {:?}", caller.data());
    });

    let instance = Instance::new(&mut store, &module, &[host_log.into()])?;
    let calc = instance.get_typed_func::<(), i32>(&mut store, "calc")?;

    calc.call(&mut store, ())
}

fn make_func(body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    write_leb128(body.len() as i128, &mut out);
    out.extend(body);
    out
}

fn make_code_section(functions: &[&[u8]]) -> Vec<u8> {
    let function_count = functions.len();
    let code_section_size = 1 + functions.iter().map(|v|v.len()).sum::<usize>();
    let mut out = Vec::new();
    write_leb128(code_section_size.try_into().unwrap(), &mut out);
    write_leb128(function_count.try_into().unwrap(), &mut out);
    for &function in functions {
        out.extend(function);
    }
    out 
}

fn eval_wasm(expr: &Expr) -> Result<i32>{
    let mut wasm = vec![
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x0e, 0x03, 0x60,
        0x01, 0x7f, 0x00, 0x60, 0x00, 0x01, 0x7f, 0x60, 0x01, 0x7f, 0x01, 0x7f,
        0x02, 0x0c, 0x01, 0x04, 0x68, 0x6f, 0x73, 0x74, 0x03, 0x6c, 0x6f, 0x67,
        0x00, 0x00, 0x03, 0x03, 0x02, 0x01, 0x02, 0x07, 0x08, 0x01, 0x04, 0x63,
        0x61, 0x6c, 0x63, 0x00, 0x01, 0x0a ];

    let calc = make_func(&[
            0x00, // localdeclcount(0)
            0x41, 0xfb, 0x00, // i32.const 123
            0x10, 0x00, // call function index=0 (host_log)
            0x41, 0x07, // i32.const 7
            0x10, 0x02, // call function index=2 (eval_expr)
            0x0f, // return
            0x0b, // end
    ]);

    let mut body = Vec::new();
    body.push(0x00); // no local decl count
    body.extend(expr.to_wasm());
    body.extend(&[0x0f, // return
                  0x0b, // end
                  ]);
    let eval_expr = make_func(&body);

    let code_section = make_code_section(&[ &calc, &eval_expr ]);
    wasm.extend(&code_section);

    let engine = Engine::default();
    let module = Module::new(&engine, wasm)?;

    type HostData = (u32,);
    let mut store = Store::new(&engine, (42,));
    let host_log = Func::wrap(&mut store, |caller: Caller<'_, HostData>, param: i32| {
        assert_eq!(param, 123);
        // eprintln!("WebAssembly param: {}", param);
        assert_eq!(*caller.data(), (42,));
        // eprintln!("HostData state is: {:?}", caller.data());
    });

    let instance = Instance::new(&mut store, &module, &[host_log.into()])?;
    let calc = instance.get_typed_func::<(), i32>(&mut store, "calc")?;

    calc.call(&mut store, ())
}

fn main() -> Result<()> {
    let args = std::env::args();
    for arg in args.skip(1) {
        println!();
        println!("Expression:\t{arg}");
        let tokens: Vec<_> = Tokenizer::from_str(&arg).collect();
        println!("Tokenized:\t{tokens:?}");
        let expr = Expr::parse(&mut tokens.into_iter().peekable());
        println!("WASM Text:\t{}", expr.to_wat().replace('\n', "\n\t\t"));
        let result = eval_wat(&expr)?;
        println!("Text result:\t{result}");
        let result = eval_wasm(&expr)?;
        println!("Binary result:\t{result}");
    }

    Ok(())
}
