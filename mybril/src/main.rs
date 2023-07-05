use std::{
    collections::{HashMap, HashSet},
    io::{stdin, Read},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize, Debug, Serialize)]
struct Bril {
    functions: Vec<Function>,
}

#[derive(Deserialize, Debug, Serialize)]
struct Function {
    instrs: Vec<Instruction>,
    name: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
struct Instruction {
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    labels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    op: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    args: Option<Vec<String>>,
}

fn main() {
    let mut buffer = String::new();
    stdin().read_to_string(&mut buffer).unwrap();
    let bril: Bril = serde_json::from_str(&buffer).unwrap();

    for func in bril.functions {
        let mut adds = 0;
        for instr in func.instrs {
            if instr.op.as_deref() == Some("add") {
                adds += 1;
            }
        }
        println!("Function {} has {} adds", func.name, adds);
    }
}

fn trivial_dce(function: &mut Function) {
    let mut used = HashSet::new();

    for inst in &function.instrs {
        if let Some(args) = inst.args.as_ref() {
            for arg in args {
                used.insert(arg.clone());
            }
        }
    }

    function.instrs.retain(|inst| {
        if let Some(dest) = inst.dest.as_ref() {
            used.contains(dest.as_str())
        } else {
            true
        }
    });
}

fn my_trivial_dce_graph(function: &mut Function) {
    let mut used_by_effects = HashSet::new();
    let mut uses: HashMap<String, HashSet<String>> = HashMap::new();

    for inst in &function.instrs {
        if let Some(args) = inst.args.as_ref() {
            if let Some(dest) = inst.dest.as_ref() {
                uses.entry(dest.clone())
                    .or_default()
                    .extend(args.iter().cloned());
            }
        }
        if matches!(inst.op.as_deref(), Some("print") | Some("br")) {
            used_by_effects.extend(inst.args.as_ref().unwrap().iter().cloned());
        }
    }

    let mut used = HashSet::new();

    let mut stack = used_by_effects.into_iter().collect::<Vec<_>>();
    while let Some(dest) = stack.pop() {
        if !used.contains(&dest) {
            if let Some(args) = uses.get(&dest) {
                stack.extend(args.iter().cloned());
            }
            used.insert(dest);
        }
    }

    function.instrs.retain(|inst| {
        if let Some(dest) = inst.dest.as_ref() {
            used.contains(dest.as_str())
        } else {
            true
        }
    });
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
struct InstValue {
    op: String,
    args: Vec<usize>,
}

struct ValueTable {
    var2num: HashMap<String, usize>,
    value_table: HashMap<InstValue, (usize, String)>,
}

fn local_value_numbering(instrs: &mut Vec<Instruction>) {}

struct Labeler {
    banned: HashSet<String>,
    counters: HashMap<String, usize>,
}

impl Labeler {
    fn new(partitioned: &[Vec<Instruction>]) -> Self {
        let banned: HashSet<String> = partitioned
            .iter()
            .filter_map(|block| block[0].label.clone())
            .collect();

        Self {
            banned,
            counters: HashMap::new(),
        }
    }

    fn label(&mut self, prefix: &str) -> String {
        let counter = self.counters.entry(prefix.to_string()).or_insert(0);
        loop {
            let label = format!("{}{}", prefix, counter);
            *counter += 1;
            if !self.banned.contains(&label) {
                return label;
            }
        }
    }
}

fn partition(instrs: &[Instruction]) -> Vec<Vec<Instruction>> {
    let mut blocks = Vec::new();
    let mut block = Vec::new();
    for instr in instrs {
        if instr.label.is_some() {
            if !block.is_empty() {
                blocks.push(block);
            }
            block = Vec::new();
            block.push(instr.clone());
        } else {
            match instr.op.as_deref() {
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

fn add_label(partitioned: &mut [Vec<Instruction>]) {
    let mut labeler = Labeler::new(&partitioned);

    for block in partitioned {
        if block[0].label.is_none() {
            block.insert(
                0,
                Instruction {
                    label: Some(labeler.label("b")),
                    ..Default::default()
                },
            );
        }
    }
}

fn add_terminatior(labeled: &mut [Vec<Instruction>]) {
    for i in 0..labeled.len() {
        let next = labeled
            .get(i + 1)
            .map(|block| block[0].label.as_deref().unwrap());

        match labeled[i].last().unwrap().op.as_deref() {
            Some("br") | Some("jmp") | Some("ret") => {}
            _ => {
                if let Some(next) = next {
                    labeled[i].push(Instruction {
                        op: Some("jmp".to_string()),
                        labels: Some(vec![next.to_string()]),
                        ..Default::default()
                    })
                } else {
                    labeled[i].push(Instruction {
                        op: Some("ret".to_string()),
                        ..Default::default()
                    })
                }
            }
        }
    }
}

fn dot(bril: &Bril) -> String {
    use std::fmt::Write;

    let mut dot = String::new();

    for function in &bril.functions {
        writeln!(dot, "digraph {} {{", function.name).unwrap();

        let mut partitioned = partition(&function.instrs);
        add_label(&mut partitioned);
        add_terminatior(&mut partitioned);

        for block in &partitioned {
            let label = block[0].label.as_deref().unwrap();
            writeln!(dot, "  {label};").unwrap();
        }

        for block in &partitioned {
            let from = block[0].label.as_deref().unwrap();
            if let Some(labels) = block.last().unwrap().labels.as_ref() {
                for to in labels {
                    writeln!(dot, "  {} -> {};", from, to).unwrap();
                }
            }
        }

        writeln!(dot, "}}").unwrap();
    }

    dot
}

#[cfg(test)]
mod test {

    use insta::{assert_display_snapshot, glob};
    use std::{
        io::Write,
        process::{Command, Stdio},
    };

    use super::*;

    fn bril2json(src: &str) -> String {
        let mut child = Command::new("bril2json")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(src.as_bytes()).unwrap();
        drop(stdin);

        String::from_utf8(child.wait_with_output().unwrap().stdout).unwrap()
    }

    fn bril2txt(src: &str) -> String {
        let mut child = Command::new("bril2txt")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(src.as_bytes()).unwrap();
        drop(stdin);

        String::from_utf8(child.wait_with_output().unwrap().stdout).unwrap()
    }

    fn brili(src: &str) -> (String, usize) {
        let mut child = Command::new("brili")
            .arg("-p")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(src.as_bytes()).unwrap();
        drop(stdin);

        let result = child.wait_with_output().unwrap();
        let stdout = String::from_utf8(result.stdout).unwrap();
        let stderr = String::from_utf8(result.stderr).unwrap();

        (
            stdout,
            stderr
                .trim_start_matches("total_dyn_inst: ")
                .trim()
                .parse()
                .unwrap(),
        )
    }

    #[test]
    fn test_dot() {
        glob!("..", "tests/test/interp/core/*.bril", |path| {
            let txt = std::fs::read_to_string(&path).unwrap();
            let json = bril2json(&txt);
            let bril: Bril = serde_json::from_str(&json).unwrap();
            let dot = dot(&bril);
            assert_display_snapshot!(dot);
        });
    }

    #[test]
    fn test_trivial_dce() {
        glob!("..", "tests/examples/tdce/*.bril", |path| {
            let txt = std::fs::read_to_string(&path).unwrap();
            let json = bril2json(&txt);
            let mut bril: Bril = serde_json::from_str(&json).unwrap();

            for function in &mut bril.functions {
                trivial_dce(function);
            }

            let json_after = serde_json::to_string_pretty(&bril).unwrap();

            let orig = brili(&json);
            let after = brili(&json_after);

            assert_eq!(orig.0, after.0);
            assert!(orig.1 >= after.1);

            assert_display_snapshot!(format!(
                "{}\n\n{} -> {}\n\n{}",
                txt,
                orig.1,
                after.1,
                bril2txt(json_after.as_str())
            ));
        });
    }

    #[test]
    fn test_my_trivial_dce_graph() {
        glob!("..", "tests/examples/tdce/*.bril", |path| {
            let txt = std::fs::read_to_string(&path).unwrap();
            let json = bril2json(&txt);
            let mut bril: Bril = serde_json::from_str(&json).unwrap();

            for function in &mut bril.functions {
                my_trivial_dce_graph(function);
            }

            let json_after = serde_json::to_string_pretty(&bril).unwrap();

            let orig = brili(&json);
            let after = brili(&json_after);

            assert_eq!(orig.0, after.0);
            assert!(orig.1 >= after.1);

            assert_display_snapshot!(format!(
                "{}\n\n{} -> {}\n\n{}",
                txt,
                orig.1,
                after.1,
                bril2txt(json_after.as_str())
            ));
        });
    }
}
