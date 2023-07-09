use std::collections::{HashMap, HashSet};

use crate::{add_label, add_terminatior, partition, Function};

fn defined(func: &Function) -> Vec<(String, (HashSet<String>, HashSet<String>))> {
    let blocks = {
        let mut blocks = partition(&func.instrs);
        add_label(&mut blocks);
        add_terminatior(&mut blocks);
        blocks
    };

    let label_map = blocks
        .iter()
        .map(|block| (block[0].label.as_deref().unwrap(), block))
        .collect::<HashMap<_, _>>();

    let labels: Vec<&str> = label_map.keys().map(|&l| l).collect();

    let (predecessors, successors) = {
        let mut predecessors = labels
            .iter()
            .map(|&label| (label, Default::default()))
            .collect::<HashMap<_, _>>();
        let mut successors = labels
            .iter()
            .map(|&label| (label, Default::default()))
            .collect::<HashMap<_, _>>();

        for b in &blocks {
            let start = b[0].label.as_deref().unwrap();

            if let Some(labels) = b.last().unwrap().labels.as_ref() {
                for dest in labels {
                    let dest = dest.as_str();
                    successors.entry(start).or_insert_with(Vec::new).push(dest);
                    predecessors
                        .entry(dest)
                        .or_insert_with(Vec::new)
                        .push(start);
                }
            }
        }
        (predecessors, successors)
    };

    let mut defined: HashMap<String, (HashSet<String>, HashSet<String>)> = labels
        .iter()
        .map(|label| (label.to_string(), Default::default()))
        .collect();

    let mut work_list = labels;

    while let Some(label) = work_list.pop() {
        let in_vars: HashSet<String> = predecessors[label]
            .iter()
            .flat_map(|&p| defined[p].1.iter().cloned())
            .collect();

        let out_vars: HashSet<String> = label_map[label]
            .iter()
            .filter_map(|i| i.dest.clone())
            .chain(in_vars.iter().cloned())
            .collect();

        let entry = defined.entry(label.to_string()).or_default();
        entry.0 = in_vars;
        if entry.1 != out_vars {
            entry.1 = out_vars;
            work_list.extend(successors[label].iter().cloned());
        }
    }

    blocks
        .iter()
        .map(|b| {
            let label = b[0].label.as_deref().unwrap();
            (label.to_string(), defined.remove(label).unwrap())
        })
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