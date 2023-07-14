use std::{
    collections::{HashMap, HashSet},
    iter,
    ops::Deref,
};

use crate::Instruction;

#[derive(Debug, Clone)]
// An instructions chunk with a label at the beginning and a terminator at the end and no label or terminator in the middle.
pub struct BasicBlock(Vec<Instruction>);

impl Into<Vec<Instruction>> for BasicBlock {
    fn into(self) -> Vec<Instruction> {
        self.0
    }
}

impl Deref for BasicBlock {
    type Target = [Instruction];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl BasicBlock {
    pub fn new_blocks(instrs: &[Instruction]) -> Vec<Self> {
        let mut partitioned = partition(instrs);
        add_label(&mut partitioned);
        add_terminatior(&mut partitioned);
        partitioned
            .into_iter()
            .map(|block| Self(block))
            .collect::<Vec<_>>()
    }

    pub fn insert_phi(&mut self, phis: &HashMap<String, HashMap<String, String>>) {
        self.0 = iter::once(self[0].clone())
            .chain(
                phis.into_iter()
                    .map(|(var_name, aliases)| {
                        let aliases = aliases.iter().collect::<Vec<_>>();

                        Instruction {
                            op: Some("phi".to_string()),
                            labels: Some(
                                aliases.iter().map(|(label, _)| label.to_string()).collect(),
                            ),
                            dest: Some(var_name.to_string()),
                            args: Some(aliases.iter().map(|(_, var)| var.to_string()).collect()),
                            ..Default::default()
                        }
                    })
                    .chain(self[1..].iter().cloned()),
            )
            .collect::<Vec<_>>();
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

pub fn partition(instrs: &[Instruction]) -> Vec<Vec<Instruction>> {
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
