use std::{
    collections::HashMap,
    io::{stdin, Read},
};

use serde::{Deserialize, Serialize};
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

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Instruction<'a> {
    op: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dest: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
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

struct Cfg<'a> {
    entry_label: String,
    blocks: HashMap<String, Vec<Instruction<'a>>>,
    predecessors: HashMap<String, Vec<String>>,
    successors: HashMap<String, Vec<String>>,
}

fn basic_blocks<'a>(instrs: &[Instruction<'a>]) -> Vec<Vec<Instruction<'a>>> {
    let mut blocks = Vec::new();
    let mut block = Vec::new();
    for instr in instrs {
        match instr.op {
            "br" | "jmp" => {
                block.push(instr.clone());
                blocks.push(block);
                block = Vec::new();
            }
            "label" => {
                if !block.is_empty() {
                    blocks.push(block);
                }
                block = Vec::new();
                block.push(instr.clone());
            }
            _ => {
                block.push(instr.clone());
            }
        }
    }
    if !block.is_empty() {
        blocks.push(block);
    }
    blocks
}

#[test]
fn test_parse() {
    const JSON: &str = include_str!("../assets/add.json");
    let _bril: Bril = serde_json::from_str(JSON).unwrap();
}

#[test]
fn test_basic_blocks() {
    const JSON: &str = include_str!("../assets/add.json");
    let bril: Bril = serde_json::from_str(JSON).unwrap();

    let basic_blocks = basic_blocks(&bril.functions[0].instrs);
    insta::assert_json_snapshot!(basic_blocks);
}
