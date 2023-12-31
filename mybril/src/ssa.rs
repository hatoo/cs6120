use std::collections::{HashMap, HashSet};

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
                    if !dominators[c.as_str()].contains(b) && a != c {
                        dominant_fronteers.entry(a).or_default().insert(c.as_str());
                    }
                }
            }
        }

        dominant_fronteers
    }

    pub fn insert_phi(&mut self) {
        let mut phis: HashMap<String, HashMap<String, HashMap<String, String>>> = HashMap::new();

        let mut defs: HashMap<&str, HashSet<&str>> = HashMap::new();

        for (label, entry) in &self.graph {
            for instr in entry.basic_block.as_ref() {
                if let Some(dest) = &instr.dest {
                    defs.entry(dest).or_default().insert(label.as_str());
                }
            }
        }

        let dominant_fronteers = self.dominant_fronteers();
        for (var, defs) in defs {
            let mut visited = HashSet::new();
            let mut stack = defs.iter().copied().collect::<Vec<_>>();

            while let Some(d) = stack.pop() {
                if !visited.insert(d) {
                    continue;
                }
                for &block in dominant_fronteers.get(d).unwrap_or(&Default::default()) {
                    phis.entry(block.to_string())
                        .or_default()
                        .entry(var.to_string())
                        .or_default()
                        .insert(d.to_string(), var.to_string());

                    stack.push(block);
                }
            }
        }

        for (label, block) in self.graph.iter_mut() {
            if let Some(phis) = phis.remove(label.as_str()) {
                block.basic_block.insert_phi(&phis);
            }
        }
    }

    fn _rename(
        &mut self,
        block: &str,
        dominators: &HashMap<String, HashSet<String>>,
        stack: &mut HashMap<String, Vec<String>>,
        counter: &mut HashMap<String, usize>,
    ) {
        let old_stack = stack.clone();
        let cfg_entry = self.graph.get_mut(block).unwrap();
        for instr in &mut cfg_entry.basic_block.0 {
            // replace each argument to instr with stack[old name]
            if let Some(args) = &mut instr.args {
                if instr.op.as_deref() != Some("phi") {
                    for arg in args {
                        if let Some(stack) = stack.get(arg) {
                            *arg = stack.last().unwrap().clone();
                        }
                    }
                }
            }

            // replace instr's destination with a new name
            if let Some(dest) = instr.dest.as_mut() {
                let n = counter.entry(dest.clone()).or_default();
                *n += 1;
                let new_name = format!("{}.{}", dest, n);

                stack
                    .entry(dest.clone())
                    .or_default()
                    .push(new_name.clone());
                *dest = new_name.clone();
            }
        }

        let succs = cfg_entry.successors.clone();
        for s in &succs {
            for instr in &mut self.graph.get_mut(s.as_str()).unwrap().basic_block.0 {
                if instr.op.as_deref() == Some("phi") {
                    for i in 0..instr.args.as_ref().unwrap().len() {
                        if instr.labels.as_ref().unwrap()[i] == block {
                            let arg = &mut instr.args.as_mut().unwrap()[i];
                            if let Some(stack) = stack.get(arg.as_str()) {
                                *arg = stack.last().unwrap().clone();
                            }
                        }
                    }
                }
            }
        }

        for s in &succs {
            if dominators[s.as_str()].contains(block) {
                self._rename(s, dominators, stack, counter);
            }
        }

        *stack = old_stack;
    }

    pub fn rename(&mut self) {
        let entry = self.entry.clone();
        self._rename(
            &entry,
            &self
                .dominators()
                .into_iter()
                .map(|(k, v)| {
                    (
                        k.to_string(),
                        v.into_iter().map(|s| s.to_string()).collect(),
                    )
                })
                .collect(),
            &mut Default::default(),
            &mut Default::default(),
        );
    }
}

#[cfg(test)]
mod test {
    use std::collections::{BTreeMap, BTreeSet};

    use insta::{assert_display_snapshot, glob};

    use crate::{
        basic_block::BasicBlock,
        test::{bril2json, bril2txt, brili},
        Bril,
    };

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

    #[test]
    fn test_insert_phi() {
        glob!("..", "tests/examples/ssa/*.bril", |path| {
            let txt = std::fs::read_to_string(&path).unwrap();
            let json = bril2json(&txt);
            let mut bril: Bril = serde_json::from_str(&json).unwrap();

            let mut output = txt.clone();
            output.push_str("\n\n");

            for function in &mut bril.functions {
                let mut cfg = crate::ssa::Cfg::new(&function);
                cfg.insert_phi();

                // !!
                let basic_blocks = BasicBlock::new_blocks(function.instrs.as_slice());

                function.instrs = basic_blocks
                    .into_iter()
                    .flat_map(|block| {
                        let label = block[0].label.clone().unwrap();
                        cfg.graph[&label].basic_block.iter().cloned()
                    })
                    .collect::<Vec<_>>();
            }

            let json_after = serde_json::to_string_pretty(&bril).unwrap();

            let orig = brili(&json);
            let after = brili(&json_after);

            assert_eq!(orig.0, after.0);

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
    fn test_rename() {
        glob!("..", "tests/examples/ssa/loop-orig.bril", |path| {
            let txt = std::fs::read_to_string(&path).unwrap();
            let json = bril2json(&txt);
            let mut bril: Bril = serde_json::from_str(&json).unwrap();

            let mut output = txt.clone();
            output.push_str("\n\n");

            for function in &mut bril.functions {
                let mut cfg = crate::ssa::Cfg::new(&function);
                cfg.insert_phi();
                cfg.rename();

                // !!
                let basic_blocks = BasicBlock::new_blocks(function.instrs.as_slice());

                function.instrs = basic_blocks
                    .into_iter()
                    .flat_map(|block| {
                        let label = block[0].label.clone().unwrap();
                        cfg.graph[&label].basic_block.iter().cloned()
                    })
                    .collect::<Vec<_>>();
            }

            let json_after = serde_json::to_string_pretty(&bril).unwrap();

            println!("{}", bril2txt(json_after.as_str()));

            let orig = brili(&json);
            let after = brili(&json_after);

            assert_eq!(orig.0, after.0);

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
