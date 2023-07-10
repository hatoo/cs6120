use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    marker::PhantomData,
};

use crate::{basic_block::BasicBlock, Function, Instruction};

trait Merger<S>
where
    S: 'static,
{
    fn merge<'a, I: Iterator<Item = &'a S>>(&self, iter: I) -> S;
}

trait Tranfer<S> {
    fn transfer(&self, instrs: &[Instruction], in_vars: &S) -> S;
}

struct Forward<S, M, T> {
    m: M,
    t: T,
    _s: PhantomData<S>,
}

impl<S, M, T> Forward<S, M, T>
where
    S: 'static + PartialEq,
    M: Merger<S>,
    T: Tranfer<S>,
{
    fn analyze(&self, blocks: &[BasicBlock]) -> HashMap<String, (S, S)> {
        let labels: Vec<&str> = blocks
            .iter()
            .map(|block| block[0].label.as_deref().unwrap())
            .collect();

        let label_map = blocks
            .iter()
            .map(|block| (block[0].label.as_deref().unwrap(), block))
            .collect::<HashMap<_, _>>();

        let (predecessors, successors) = {
            let mut predecessors = HashMap::new();
            let mut successors = HashMap::new();

            for b in blocks {
                let start = b[0].label.as_deref().unwrap();

                if let Some(labels) = b.last().unwrap().labels.as_ref() {
                    for dest in labels {
                        let dest = dest.as_str();
                        successors
                            .entry(start)
                            .or_insert_with(HashSet::new)
                            .insert(dest);
                        predecessors
                            .entry(dest)
                            .or_insert_with(HashSet::new)
                            .insert(start);
                    }
                }
            }
            (predecessors, successors)
        };

        let mut result: HashMap<String, (S, S)> = HashMap::new();

        let mut work_list = labels.clone();

        while let Some(label) = work_list.pop() {
            let in_vars: S = self.m.merge(
                predecessors
                    .get(label)
                    .unwrap_or(&Default::default())
                    .iter()
                    .flat_map(|&p| result.get(p).into_iter().map(|(_, out_vars)| out_vars)),
            );

            let out_vars = self.t.transfer(label_map[label], &in_vars);

            match result.entry(label.to_string()) {
                Entry::Occupied(mut io) => {
                    let entry = io.get_mut();
                    entry.0 = in_vars;
                    if entry.1 != out_vars {
                        entry.1 = out_vars;
                        work_list.extend(
                            successors
                                .get(label)
                                .into_iter()
                                .flat_map(HashSet::iter)
                                .cloned(),
                        );
                    }
                }
                Entry::Vacant(io) => {
                    io.insert((in_vars, out_vars));
                    work_list.extend(
                        successors
                            .get(label)
                            .into_iter()
                            .flat_map(HashSet::iter)
                            .cloned(),
                    );
                }
            }
        }

        result
    }
}

struct DefinedMerger;
struct DefinedTransfer;

impl Merger<HashSet<String>> for DefinedMerger {
    fn merge<'a, I>(&self, iter: I) -> HashSet<String>
    where
        I: Iterator<Item = &'a HashSet<String>>,
    {
        iter.fold::<HashSet<String>, _>(Default::default(), |mut acc, set| {
            acc.extend(set.iter().cloned());
            acc
        })
    }
}

impl Tranfer<HashSet<String>> for DefinedTransfer {
    fn transfer(&self, instrs: &[Instruction], in_vars: &HashSet<String>) -> HashSet<String> {
        let mut out_vars = in_vars.clone();
        for instr in instrs {
            if let Some(dest) = &instr.dest {
                out_vars.insert(dest.clone());
            }
        }
        out_vars
    }
}

const DEFINED: Forward<HashSet<String>, DefinedMerger, DefinedTransfer> = Forward {
    m: DefinedMerger,
    t: DefinedTransfer,
    _s: PhantomData,
};

#[cfg(test)]
mod test {
    use std::collections::BTreeSet;

    use insta::{assert_display_snapshot, glob};

    use crate::{basic_block::BasicBlock, dataflow::DEFINED, test::bril2json, Bril, Instruction};

    #[test]
    fn test_defined_generic() {
        glob!("..", "tests/examples/df/*.bril", |path| {
            let txt = std::fs::read_to_string(&path).unwrap();
            let json = bril2json(&txt);
            let mut bril: Bril = serde_json::from_str(&json).unwrap();

            let mut output = String::new();

            for func in &mut bril.functions {
                let basic_blocks = BasicBlock::new_blocks(func.instrs.as_slice());
                let mut defined = DEFINED.analyze(&basic_blocks);
                let instrs = basic_blocks
                    .into_iter()
                    .flat_map(|b| Into::<Vec<Instruction>>::into(b).into_iter())
                    .collect::<Vec<_>>();
                let labels = instrs
                    .iter()
                    .filter_map(|i| i.label.as_deref())
                    .collect::<Vec<_>>();
                let defined = labels
                    .into_iter()
                    .map(|l| (l.to_string(), defined.remove(l).unwrap()))
                    .collect::<Vec<_>>();

                output.push_str(&format!("{}:\n", func.name));
                for (label, (var_in, var_out)) in defined.iter() {
                    output.push_str(&format!("  {}:\n", label));
                    output.push_str(&format!(
                        "    in: {:?}\n",
                        var_in.into_iter().collect::<BTreeSet<_>>()
                    ));
                    output.push_str(&format!(
                        "    out: {:?}\n",
                        var_out.into_iter().collect::<BTreeSet<_>>()
                    ));
                }
                output.push_str("\n");
            }

            assert_display_snapshot!(format!("{txt}\n{output}"));
        });
    }
}
