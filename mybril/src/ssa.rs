use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
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

    pub fn dominators(&self) -> HashMap<&str, HashSet<&str>> {
        let mut dominators = HashMap::new();
        let order = self.reverse_post_order();

        for &label in &order {
            dominators.insert(label, order.iter().copied().collect::<HashSet<_>>());
        }

        let mut changed = true;
        while changed {
            changed = false;
            for &label in &order {
                let predecessors = &self.graph[label].predesessors;
                let mut new_doominators = if predecessors.is_empty() {
                    Default::default()
                } else {
                    let mut new_dominants = order.iter().copied().collect::<HashSet<_>>();
                    for pred in &self.graph[label].predesessors {
                        new_dominants = new_dominants
                            .intersection(&dominators[pred.as_str()])
                            .copied()
                            .collect::<HashSet<_>>();
                    }
                    new_dominants
                };
                new_doominators.insert(label);
                let entry = dominators.entry(label).or_default();

                if new_doominators != *entry {
                    changed = true;
                    *entry = new_doominators;
                }
            }
        }

        dominators
    }

    pub fn dominant_fronteers(&self) -> HashMap<&str, HashSet<&str>> {
        let mut dominant_fronteers: HashMap<&str, HashSet<&str>> = HashMap::new();
        let dominators = self.dominators();

        for (&b, dom) in &dominators {
            for &a in dom {
                // a dominates b
                for c in &self.graph[b].successors {
                    if !dominators[c.as_str()].contains(b) {
                        dominant_fronteers.entry(a).or_default().insert(c.as_str());
                    }
                }
            }
        }

        dominant_fronteers
    }
}

#[cfg(test)]
mod test {
    use std::collections::{BTreeMap, BTreeSet};

    use insta::{assert_display_snapshot, glob};

    use crate::{test::bril2json, Bril};

    #[test]
    fn test_dominators_dot() {
        glob!("..", "tests/examples/df/*.bril", |path| {
            let txt = std::fs::read_to_string(&path).unwrap();
            let json = bril2json(&txt);
            let bril: Bril = serde_json::from_str(&json).unwrap();

            let mut output = txt.clone();
            output.push_str("\n\n");

            for function in bril.functions {
                let cfg = crate::ssa::Cfg::new(&function);
                let dominators = cfg.dominators();

                output.push_str(&format!("function {}:\n", function.name));
                for (label, dominators) in dominators.into_iter().collect::<BTreeMap<_, _>>() {
                    output.push_str(&format!("  {}: ", label));
                    for dominator in dominators.into_iter().collect::<BTreeSet<_>>() {
                        output.push_str(&format!("{} ", dominator));
                    }
                    output.push_str("\n");
                }
                output.push_str("\n");
            }

            assert_display_snapshot!(output);
        });
    }
}
