use std::collections::{HashMap, HashSet};

use crate::{add_label, add_terminatior, partition, Function};

fn defined(func: &Function) -> Vec<(String, (HashSet<String>, HashSet<String>))> {
    let blocks = {
        let mut blocks = partition(&func.instrs);
        add_label(&mut blocks);
        add_terminatior(&mut blocks);
        blocks
    };

    let labels: Vec<&str> = blocks
        .iter()
        .map(|block| block[0].label.as_deref().unwrap())
        .collect();

    let label_map = blocks
        .iter()
        .map(|block| (block[0].label.as_deref().unwrap(), block.as_slice()))
        .collect::<HashMap<_, _>>();

    let (predecessors, successors) = {
        let mut predecessors = HashMap::new();
        let mut successors = HashMap::new();

        for b in &blocks {
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

    let mut defined: HashMap<String, (HashSet<String>, HashSet<String>)> = HashMap::new();

    let mut work_list = labels.clone();

    while let Some(label) = work_list.pop() {
        let in_vars: HashSet<String> = predecessors
            .get(label)
            .unwrap_or(&Default::default())
            .iter()
            .flat_map(|&p| {
                defined
                    .get(p)
                    .into_iter()
                    .flat_map(|(_, out_vars)| out_vars.iter())
            })
            .cloned()
            .collect();

        let out_vars: HashSet<String> = label_map
            .get(label)
            .unwrap_or(&Default::default())
            .iter()
            .filter_map(|i| i.dest.clone())
            .chain(in_vars.iter().cloned())
            .collect();

        let entry = defined.entry(label.to_string()).or_default();
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

    labels
        .into_iter()
        .map(|l| (l.to_string(), defined.remove(l).unwrap()))
        .collect()
}

#[cfg(test)]
mod test {
    use std::collections::BTreeSet;

    use insta::{assert_display_snapshot, glob};

    use crate::{dataflow::defined, test::bril2json, Bril};

    #[test]
    fn test_defined() {
        glob!("..", "tests/examples/df/*.bril", |path| {
            let txt = std::fs::read_to_string(&path).unwrap();
            let json = bril2json(&txt);
            let bril: Bril = serde_json::from_str(&json).unwrap();

            let mut output = String::new();

            for func in &bril.functions {
                output.push_str(&format!("{}:\n", func.name));
                for (label, (var_in, var_out)) in defined(func).iter() {
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
