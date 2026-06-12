use std::iter::Peekable;
use wasmtime::*;

enum Expr {
    Add(Box<Expr>, Box<Term>),
    Subtract(Box<Expr>, Box<Term>),
    Term(Box<Term>),
}

impl Expr {
    fn parse(i: &mut Peekable<impl Iterator<Item = char>>) -> Self {
        let mut term = Self::Term(Box::new(Term::parse(i)));
        loop {
            match i.peek() {
                Some('+') => {
                    i.next();
                    term = Self::Add(Box::new(term), Box::new(Term::parse(i)));
                },
                Some('-') => {
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
}

enum Term {
    Multiply(Box<Term>, Box<Factor>),
    Divide(Box<Term>, Box<Factor>),
    Factor(Box<Factor>),
}

impl Term {
    fn parse(i: &mut Peekable<impl Iterator<Item = char>>) -> Self {
        let mut factor = Self::Factor(Box::new(Factor::parse(i)));
        loop {
            match i.peek() {
                Some('*') => {
                    i.next();
                    factor = Self::Multiply(Box::new(factor),
                                            Box::new(Factor::parse(i)));
                },
                Some('/') => {
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
}

enum Factor {
    Const(i32),
    Param(String),
    Expr(Box<Expr>), // braced: ( Expr )
}

impl Factor {
    fn parse(i: &mut Peekable<impl Iterator<Item = char>>) -> Self {
        let c = i.next().expect("out of tokens, exected Factor");
        match c {
            '0'..='9' => Self::Const(c.to_digit(10).unwrap() as i32),
            '$' => {
                let c = i.next().expect("out of tokens, expected [a-z]");
                assert!(('a'..='z').contains(&c));
                Self::Param("$".to_string() + &c.to_string())
            },
            '(' => {
                let expr = Expr::parse(i);
                let c = i.next().expect("out of tokens, expected ')'");
                assert_eq!(c, ')');
                Self::Expr(Box::new(expr))
            },
            _ => panic!("Invalid token for Factor: '{c}'"),
        }
    }

    fn to_wat(&self) -> String {
        match &self {
            Self::Const(c) => format!("i32.const {c}"),
            Self::Param(p) => format!("local.get {p}"),
            Self::Expr(e) => e.to_wat(),
        }
    }
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
        Box::new(Term::Factor(Box::new(Factor::Param("$x".to_string())))),
    );
    assert_eq!(expr.to_wat(), expected);
}


#[test]
fn test_parse() {
    fn compile(s: &str) -> String {
        let expr = Expr::parse(&mut s.chars().peekable());
        expr.to_wat().replace('\n', " ")
    }
    assert_eq!(compile("1"), "i32.const 1");
    assert_eq!(compile("2+3"), "i32.const 2 i32.const 3 i32.add");
    assert_eq!(compile("2*3"), "i32.const 2 i32.const 3 i32.mul");
    assert_eq!(compile("1+2*3"), "i32.const 1 i32.const 2 i32.const 3 i32.mul i32.add");
    assert_eq!(compile("1*2+3"), "i32.const 1 i32.const 2 i32.mul i32.const 3 i32.add");
    assert_eq!(compile("(1+2)*3"), "i32.const 1 i32.const 2 i32.add i32.const 3 i32.mul");
    assert_eq!(compile("1+$x"), "i32.const 1 local.get $x i32.add");
}

#[test]
fn test_eval() {
    fn evaluate(s: &str) -> i32 {
        let expr = Expr::parse(&mut s.chars().peekable());
        //eprintln!("{}", expr.to_wat());
        eval(&expr.to_wat()).expect("Failed to evaluate")
    }
    assert_eq!(evaluate("1+2+3+4+5+6+7+8+9"), 45);
    assert_eq!(evaluate("1*2*3*4*5*6*7*8*9"), 362880);
    assert_eq!(evaluate("0*$x"), 0);
    assert_eq!(evaluate("$x-$x"), 0);
    assert_eq!(evaluate("(($x))"), 7);
    assert_eq!(evaluate("0-$x"), -7);
    assert_eq!(evaluate("2+2*9/3-1"), 7);
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

fn eval(expr_wat: &str) -> Result<i32> {
    let wat = add_fluff(expr_wat); // wrap raw WAT in Module

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

fn main() -> Result<()> {
    let args = std::env::args();
    for arg in args.skip(1) {
        println!();
        println!("Expression:\t{arg}");
        let expr = Expr::parse(&mut arg.chars().peekable());
        let wat = expr.to_wat();
        println!("WASM Text:\t{}", wat.replace('\n', "\n\t\t"));
        let result = eval(&wat)?;
        println!("Evaluation:\t{result}");
    }

    Ok(())
}
