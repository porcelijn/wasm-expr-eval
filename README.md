# WASM Expression Evaluator

License: [NON-AI-CC0](LICENSE.txt)

A minimalist experiment messing around with WASM code generation and expression parsing in Rust.

```bash
cargo build
cargo run -- '1+2' '1+$x*3' '($x+4)*2' '($x*$x-10)/3'

```

With `$x = 7` currently hard-code, we get:

```
Expression:     1+2
WASM Text:      i32.const 1
                i32.const 2
                i32.add
Evaluation:     3

Expression:     1+$x*3
WASM Text:      i32.const 1
                local.get $x
                i32.const 3
                i32.mul
                i32.add
Evaluation:     22

Expression:     ($x+4)*2
WASM Text:      local.get $x
                i32.const 4
                i32.add
                i32.const 2
                i32.mul
Evaluation:     22

Expression:     ($x*$x-10)/3
WASM Text:      local.get $x
                local.get $x
                i32.mul
                i32.const 10
                i32.sub
                i32.const 3
                i32.div_u
Evaluation:     13
```
