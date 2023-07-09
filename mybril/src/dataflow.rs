use std::{
    collections::{HashMap, HashSet},
    iter::Successors,
};

use crate::{add_label, add_terminatior, partition, Function};

fn defined(func: Function) -> HashMap<String, (HashSet<String>, HashSet<String>)> {
    let mut blocks = partition(&func.instrs);
    add_label(&mut blocks);
    add_terminatior(&mut blocks);

    let label_map = blocks
        .iter()
        .map(|block| (block[0].label.as_deref().unwrap(), block))
        .collect::<HashMap<_, _>>();

    let mut predecessors = HashMap::new();
    let mut successors = HashMap::new();

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

    let mut work_list = blocks
        .iter()
        .map(|block| block[0].label.as_deref().unwrap())
        .collect::<Vec<_>>();

    let mut defined: HashMap<String, (HashSet<String>, HashSet<String>)> = work_list
        .iter()
        .map(|label| (label.to_string(), Default::default()))
        .collect();

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

    defined
}
