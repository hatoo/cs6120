use std::{
    collections::{HashMap, HashSet},
    iter::Successors,
};

use crate::{basic_block::BasicBlock, Argument, Function};

pub struct CfgEntry {
    basic_block: BasicBlock,
    predesessors: HashSet<String>,
    successors: HashSet<String>,
}

pub struct Cfg {
    arguments: Vec<Argument>,
    entry: String,
    graph: HashMap<String, CfgEntry>,
}

impl Cfg {
    pub fn new(function: &Function) -> Self {
        let basic_blocks = BasicBlock::new_blocks(&function.instrs);

        let arguments = function.args.clone().unwrap_or_default();
        let entry = basic_blocks[0][0].label.clone().unwrap();

        let mut predesessors = HashMap::new();
        let mut successors = HashMap::new();

        for block in &basic_blocks {
            let label = block[0].label.clone().unwrap();
            if let Some(nexts) = block.last().unwrap().labels.as_ref() {
                for next in nexts {
                    let next = next.clone();
                    successors
                        .entry(label.clone())
                        .or_insert_with(HashSet::new)
                        .insert(next.clone());
                    predesessors
                        .entry(next)
                        .or_insert_with(HashSet::new)
                        .insert(label.clone());
                }
            }
        }

        let graph = basic_blocks
            .into_iter()
            .map(|block| {
                let label = block[0].label.clone().unwrap();
                let predesessors = predesessors.remove(&label).unwrap_or_default();
                let successors = successors.remove(&label).unwrap_or_default();
                (
                    label,
                    CfgEntry {
                        basic_block: block,
                        predesessors,
                        successors,
                    },
                )
            })
            .collect();

        Self {
            arguments,
            entry,
            graph,
        }
    }

    pub fn reverse_post_order(&self) -> Vec<&str> {
        let mut visited = HashSet::new();
        let mut order = vec![];

        fn rec<'a>(
            label: &'a str,
            visited: &mut HashSet<&'a str>,
            order: &mut Vec<&'a str>,
            graph: &'a HashMap<String, CfgEntry>,
        ) {
            if visited.contains(label) {
                return;
            }
            visited.insert(label);
            for next in &graph[label].successors {
                rec(next, visited, order, graph);
            }
            order.push(label);
        }

        rec(&self.entry, &mut visited, &mut order, &self.graph);
        order.reverse();
        order
    }

    pub fn dominants(&self) -> HashMap<&str, HashSet<&str>> {
        let mut dominants = HashMap::new();
        let order = self.reverse_post_order();

        for &label in &order {
            dominants.insert(label, order.iter().copied().collect::<HashSet<_>>());
        }

        let mut changed = true;
        while changed {
            changed = false;
            for &label in &order {
                let predecessors = &self.graph[label].predesessors;
                let mut new_dominants = if predecessors.is_empty() {
                    Default::default()
                } else {
                    let mut new_dominants = order.iter().copied().collect::<HashSet<_>>();
                    for pred in &self.graph[label].predesessors {
                        new_dominants = new_dominants
                            .intersection(&dominants[pred.as_str()])
                            .copied()
                            .collect::<HashSet<_>>();
                    }
                    new_dominants
                };
                new_dominants.insert(label);
                let entry = dominants.entry(label).or_default();

                if new_dominants != *entry {
                    changed = true;
                    *entry = new_dominants;
                }
            }
        }

        dominants
    }
}
