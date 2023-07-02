use std::io::{stdin, Read};

use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Debug)]
struct Bril<'a> {
    #[serde(borrow)]
    functions: Vec<Function<'a>>,
}

#[derive(Deserialize, Debug)]
struct Function<'a> {
    instrs: Vec<Instruction<'a>>,
    name: &'a str,
}

#[derive(Deserialize, Debug, Clone)]
struct Instruction<'a> {
    op: &'a str,
    r#type: Option<&'a str>,
    dest: Option<&'a str>,
    value: Option<Value>,
    args: Option<Vec<&'a str>>,
}

fn main() {
    let mut buffer = String::new();
    stdin().read_to_string(&mut buffer).unwrap();
    let bril: Bril = serde_json::from_str(&buffer).unwrap();

    for func in bril.functions {
        let mut adds = 0;
        for instr in func.instrs {
            if instr.op == "add" {
                adds += 1;
            }
        }
        println!("Function {} has {} adds", func.name, adds);
    }
}

#[test]
fn test_parse() {
    const Json: &str = include_str!("../assets/add.json");
    let bril: Bril = serde_json::from_str(Json).unwrap();
}
