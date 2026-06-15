use crate::token::Token;
#[cfg(test)]
use crate::token::Tokenizer;

use std::iter::Peekable;

pub type Bindings = [String]; // vars
pub type Wat = String;
pub type Wasm = Vec<u8>;

pub trait ToWasm {
    fn to_wasm(&self, bindings: &Bindings) -> Wasm;
}

pub(crate) enum Expr {
    Add(Box<Expr>, Box<Term>),
    Subtract(Box<Expr>, Box<Term>),
    Term(Box<Term>),
}

impl Expr {
    pub fn parse(i: &mut Peekable<impl Iterator<Item = Token>>) -> Self {
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

    pub fn to_wat(&self) -> Wat {
        match &self {
            Self::Add(l, r) => format!("{}\n{}\ni32.add", l.to_wat(), r.to_wat()),
            Self::Subtract(l, r) => format!("{}\n{}\ni32.sub", l.to_wat(), r.to_wat()),
            Self::Term(t) => t.to_wat(),
        }
    }
}

impl ToWasm for Expr {
    fn to_wasm(&self, bindings: &Bindings) -> Wasm {
        match &self {
            Self::Add(l, r) => {
                let mut out = l.to_wasm(bindings);
                out.extend(&r.to_wasm(bindings));
                out.push(0x6a); // i32.add
                out
            },
            Self::Subtract(l, r) => {
                let mut out = l.to_wasm(bindings);
                out.extend(&r.to_wasm(bindings));
                out.push(0x6b); // i32.sub
                out
            },
            Self::Term(t) => t.to_wasm(bindings),
        }
    }
}

pub(crate) enum Term {
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

    fn to_wat(&self) -> Wat {
        match &self {
            Self::Multiply(l, r) => format!("{}\n{}\ni32.mul", l.to_wat(), r.to_wat()),
            Self::Divide(l, r) => format!("{}\n{}\ni32.div_u", l.to_wat(), r.to_wat()),
            Self::Factor(f) => f.to_wat(),
        }
    }
}

impl ToWasm for Term {
    fn to_wasm(&self, bindings: &Bindings) -> Wasm {
        match &self {
            Self::Multiply(l, r) => {
                let mut out = l.to_wasm(bindings);
                out.extend(&r.to_wasm(bindings));
                out.push(0x6c); // i32.mul
                out
            },
            Self::Divide(l, r) => {
                let mut out = l.to_wasm(bindings);
                out.extend(&r.to_wasm(bindings));
                out.push(0x6e); // i32.div_u
                out
            },
            Self::Factor(f) => f.to_wasm(bindings),
        }
    }
}

pub(crate) enum Factor {
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

    fn to_wat(&self) -> Wat {
        match &self {
            Self::Const(c) => format!("i32.const {c}"),
            Self::Param(p) => format!("local.get ${p}"),
            Self::Expr(e) => e.to_wat(),
        }
    }
}

impl ToWasm for Factor {
    fn to_wasm(&self, bindings: &Bindings) -> Wasm {
        match &self {
            Self::Const(c) => {
                let mut out = vec![0x41]; // i32.const
                write_leb128((*c).into(), &mut out);
                out
            },
            Self::Param(p) => {
                let mut out = vec![0x20]; // local.get
                if let Some(index) = bindings.iter().position(|b| b == p) {
                    write_leb128(index as i128, &mut out);
                } else {
                    panic!("Unknown binding for local '{p}'");
                }
                out
            },
            Self::Expr(e) => e.to_wasm(bindings),
        }
    }
}

pub fn write_leb128(mut i: i128, out: &mut Vec<u8>) {
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
    assert_eq!(compile("2 + 3"), "i32.const 2 i32.const 3 i32.add");
    assert_eq!(compile("2*3"), "i32.const 2 i32.const 3 i32.mul");
    assert_eq!(compile("1+2*3"), "i32.const 1 i32.const 2 i32.const 3 i32.mul i32.add");
    assert_eq!(compile("1*2+3"), "i32.const 1 i32.const 2 i32.mul i32.const 3 i32.add");
    assert_eq!(compile("(1+2)*3"), "i32.const 1 i32.const 2 i32.add i32.const 3 i32.mul");
    assert_eq!(compile("1+x"), "i32.const 1 local.get $x i32.add");
    assert_eq!(compile("123/x"), "i32.const 123 local.get $x i32.div_u");
    assert_eq!(compile("variable"), "local.get $variable");
}

pub fn add_fluff(expr_wat: &str) -> String {
    let mut wat = r#"
        (module
            (import "host" "log" (func $host_log (param i32)))
            (func (export "calc") (result i32)
                i32.const 123 ;; <-- Closure `param`
                call $host_log

                i32.const 7 ;; <-- WASM function param `$x`
                i32.const 9 ;; <-- WASM function param `$yy`
                call $eval_expr
                return)
    "#.to_string();

    wat.push_str("(func $eval_expr (param $x i32) (param $yy i32) (result i32)");
    wat.push_str(expr_wat);
    wat.push_str(" return)");
    wat.push_str(")");
    wat
}

fn make_func(body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    write_leb128(body.len() as i128, &mut out);
    out.extend(body);
    out
}

const SIGNATURE_TYPE: u8 = 0x01; // function signatures
const SIGNATURE_CODE: u8 = 0x0a; // function bodies, locals and opcodes

fn make_section<const SIGNATURE: u8>(payloads: &[&[u8]]) -> Vec<u8> {
    let count = payloads.len();
    assert!(count < 63); // FIXME assume 1 byte for leb128; might overflow
    let payload_size = 1 + payloads.iter().map(|v|v.len()).sum::<usize>();
    let mut out = Vec::new();
    out.push(SIGNATURE);
    write_leb128(payload_size.try_into().unwrap(), &mut out);
    write_leb128(count.try_into().unwrap(), &mut out);
    for &function in payloads {
        out.extend(function);
    }
    out
}

fn make_func_type(args: &[u8], result: &[u8]) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.push(0x60); // function signature
    write_leb128(args.len() as i128, &mut payload);
    payload.extend(args);
    write_leb128(result.len() as i128, &mut payload);
    payload.extend(result);
    payload
}

#[test]
fn test_make_func_type() {
    let f1 = make_func_type(&[0x7f], &[]);
    assert_eq!(f1, &[0x60, 0x01, 0x7f, 0x00]);
    let f2 = make_func_type(&[], &[0x7f]);
    assert_eq!(f2, &[0x60, 0x00, 0x01, 0x7f]);
    let f3 = make_func_type(&[0x7f, 0x7f], &[0x7f]);
    assert_eq!(f3, &[0x60, 0x02, 0x7f, 0x7f, 0x01, 0x7f]);
    let type_section = make_section::<SIGNATURE_TYPE>(&[ &f1, &f2, &f3]);
    assert_eq!(type_section,
            [0x01, 0x0f, 0x03, 0x60, 0x01, 0x7f, 0x00,
                                  0x60, 0x00, 0x01, 0x7f,
                                  0x60, 0x02, 0x7f, 0x7f, 0x01, 0x7f,]);
}

pub fn generate_wasm(expr: &impl ToWasm, bindings: &Bindings) -> Vec<u8> {
    let mut wasm = vec![
        0x00, 0x61, 0x73, 0x6d, // asm
        0x01, 0x00, 0x00, 0x00, // version
    ];
    wasm.extend(&make_section::<SIGNATURE_TYPE>(&[
        &make_func_type(&[0x7f], &[]),
        &make_func_type(&[], &[0x7f]),
        &make_func_type(&[0x7f, 0x7f], &[0x7f]),
    ]));
    wasm.extend(&[
        0x02, // import section signature
        0x0c, // 12B payload
        0x01, // one import
        0x04, // Module name: 4 chars
            0x68, 0x6f, 0x73, 0x74, // = "host"
        0x03, // Field name: 3 chars
            0x6c, 0x6f, 0x67, // == log
        0x00, // Import kind: function
        0x00, // Type kind index: 0: i32 -> ()

        0x03, // function section signarure
        0x03, // 3B payload
        0x02, // 2 functions:
            0x01, // Type kind index: 1: () -> i32
            0x02, // TYpe kind index: 2: i32, i32 -> i32

        0x07, // export section signature
        0x08, // 8 byte payload
        0x01, // one export
        0x04, // Field name: 4 chars
            0x63, 0x61, 0x6c, 0x63, // = "main"
        0x00, // Export kind: function
        0x01, // Function index: 1, () -> i32
    ]);

    let calc = make_func(&[
            0x00, // localdeclcount(0)
            0x41, 0xfb, 0x00, // i32.const 123
            0x10, 0x00, // call function index=0 (host_log)
            0x41, 0x07, // i32.const 7
            0x41, 0x09, // i32.const 9
            0x10, 0x02, // call function index=2 (eval_expr)
            0x0f, // return
            0x0b, // end
    ]);

    let mut body = Vec::new();
    body.push(0x00); // no local decl count
    body.extend(expr.to_wasm(bindings));
    body.extend(&[0x0f, // return
                  0x0b, // end
                  ]);
    let eval_expr = make_func(&body);

    let code_section = make_section::<SIGNATURE_CODE>(&[ &calc, &eval_expr ]);
    wasm.extend(&code_section);
    wasm
}
