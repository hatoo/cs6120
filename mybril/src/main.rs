use std::{
    collections::{HashMap, HashSet},
    io::{stdin, Read},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize, Debug)]
struct Bril {
    functions: Vec<Function>,
}

#[derive(Deserialize, Debug)]
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
    use std::{collections::BTreeMap, fs::read_dir};

    use insta::{assert_display_snapshot, assert_json_snapshot, Settings};

    use super::*;

    fn brils() -> Vec<(Settings, Bril)> {
        read_dir("tests")
            .unwrap()
            .into_iter()
            .map(|entry| {
                let entry = entry.unwrap();
                let path = entry.path();
                let json = std::fs::read_to_string(&path).unwrap();
                let mut settings = Settings::new();
                settings.set_input_file(entry.file_name());
                (
                    entry.file_name(),
                    (settings, serde_json::from_str(&json).unwrap()),
                )
            })
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .collect()
    }

    #[test]
    fn test_partition() {
        brils().into_iter().for_each(|(settings, bril)| {
            bril.functions.into_iter().for_each(|func| {
                let blocks = partition(&func.instrs);

                settings.bind(|| {
                    assert_json_snapshot!(blocks);
                })
            })
        })
    }

    #[test]
    fn test_dot() {
        for (settings, bril) in brils() {
            settings.bind(|| {
                assert_display_snapshot!(dot(&bril));
            })
        }
    }
}
