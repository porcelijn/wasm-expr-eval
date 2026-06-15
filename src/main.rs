mod ast;
mod token;

use crate::ast::{Expr, add_fluff, generate_wasm};
use crate::token::Tokenizer;

use wasmtime::{Caller, Engine, Func, Instance, Module, Result, Store};

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

fn eval_wasm(expr: &Expr) -> Result<i32>{
    let wasm = generate_wasm(expr);
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
