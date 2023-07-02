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
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    op: Option<&'a str>,
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
            if instr.op == Some("add") {
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

fn partition<'a>(instrs: &[Instruction<'a>]) -> Vec<Vec<Instruction<'a>>> {
    let mut blocks = Vec::new();
    let mut block = Vec::new();
    for instr in instrs {
        if let Some(label) = instr.label {
            if !block.is_empty() {
                blocks.push(block);
            }
            block = Vec::new();
            block.push(instr.clone());
        } else {
            match instr.op {
                Some("br") | Some("jmp") => {
                    block.push(instr.clone());
                    blocks.push(block);
                    block = Vec::new();
                }
                _ => {
                    block.push(instr.clone());
                }
            }
        }
    }
    if !block.is_empty() {
        blocks.push(block);
    }
    blocks
}

#[cfg(test)]
mod test {
    use std::fs::read_dir;

    use insta::assert_json_snapshot;

    use super::*;

    fn brils() -> Vec<Bril<'static>> {
        read_dir("tests")
            .unwrap()
            .into_iter()
            .map(|entry| {
                let entry = entry.unwrap();
                let path = entry.path();
                let json = std::fs::read_to_string(path).unwrap();
                serde_json::from_str(Box::leak(Box::new(json))).unwrap()
            })
            .collect()
    }

    #[test]
    fn test_parse() {
        brils();
    }

    #[test]
    fn test_basic_blocks() {
        brils().into_iter().for_each(|bril| {
            bril.functions.into_iter().for_each(|func| {
                let blocks = partition(&func.instrs);
                assert_json_snapshot!(blocks);
            })
        })
    }
}
