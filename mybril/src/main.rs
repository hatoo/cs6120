use std::{
    collections::{HashMap, HashSet},
    io::{stdin, Read},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::basic_block::{partition, BasicBlock};

mod basic_block;
mod dataflow;

#[derive(Deserialize, Debug, Serialize)]
struct Bril {
    pub functions: Vec<Function>,
}

#[derive(Deserialize, Debug, Serialize)]
struct Function {
    pub instrs: Vec<Instruction>,
    pub name: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Instruction {
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
    let mut bril: Bril = serde_json::from_str(&buffer).unwrap();

    for function in &mut bril.functions {
        let mut partitioned = partition(&function.instrs);
        partitioned.iter_mut().for_each(|p| {
            // drop_kill(p);
            local_value_numbering(p);
        });
        function.instrs = partitioned.into_iter().flatten().collect();
        /*
        my_trivial_dce_graph(function);
        let mut partitioned = partition(&function.instrs);
        partitioned.iter_mut().for_each(|p| {
            drop_kill(p);
        });
        function.instrs = partitioned.into_iter().flatten().collect();
        */
    }

    let json_after = serde_json::to_string_pretty(&bril).unwrap();
    print!("{json_after}");
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

#[derive(Default)]
struct ValueTable {
    var2num: HashMap<String, usize>,
    table: HashMap<InstValue, String>,
    num2var: HashMap<usize, String>,
    counter: usize,
}

impl ValueTable {
    fn num(&mut self, var: &str) -> usize {
        *self.var2num.entry(var.to_string()).or_insert_with(|| {
            let num = self.counter;
            self.counter += 1;
            self.num2var.insert(num, var.to_string());
            num
        })
    }

    fn root(&mut self, var: &str) -> String {
        let num = self.num(var);
        self.num2var[&num].clone()
    }

    fn value(
        &mut self,
        mut inst_value: InstValue,
        dest: &str,
        overwritten_after: bool,
    ) -> (Option<String>, Option<String>) {
        match inst_value.op.as_str() {
            "add" => inst_value.args.sort_unstable(),
            _ => {}
        }

        // Redifine occured. Remove old edge.
        if let Some(&num) = self.var2num.get(dest) {
            self.var2num.retain(|_, v| *v != num);
        }

        let old_name = dest;
        let dest = if overwritten_after {
            let new_dest = format!("{}_prime", dest);
            let num = self.num(&new_dest);
            self.var2num.insert(dest.to_string(), num);
            new_dest
        } else {
            dest.to_string()
        };

        let rename = if overwritten_after {
            Some(dest.clone())
        } else {
            None
        };

        if inst_value.op == "id" {
            let arg = inst_value.args[0];
            self.var2num.insert(dest.to_string(), arg);
            return (Some(self.num2var[&arg].clone()), rename);
        }

        if let Some(var) = self.table.get(&inst_value) {
            self.var2num.insert(dest.to_string(), self.var2num[var]);
            if overwritten_after {
                self.var2num.insert(old_name.to_string(), self.var2num[var]);
            }
            (Some(var.clone()), rename)
        } else {
            if !overwritten_after {
                let num = self.counter;
                self.counter += 1;
                self.num2var.insert(num, dest.to_string());
                self.var2num.insert(dest.to_string(), num);
            }
            self.table.insert(inst_value.clone(), dest.to_string());

            (None, rename)
        }
    }
}

fn local_value_numbering(instrs: &mut [Instruction]) {
    let mut table = ValueTable::default();

    let mut dest_map = HashMap::new();
    for (i, inst) in instrs.iter().enumerate() {
        if let Some(dest) = inst.dest.as_deref() {
            dest_map.insert(dest.to_string(), i);
        }
    }

    for (i, inst) in instrs.iter_mut().enumerate() {
        if inst.args.is_none() {
            continue;
        }

        if let Some(dest) = inst.dest.as_deref() {
            let inst_value = InstValue {
                op: inst.op.as_ref().unwrap().clone(),
                args: inst
                    .args
                    .as_ref()
                    .unwrap()
                    .iter()
                    .map(|arg| table.num(arg))
                    .collect(),
            };

            let (alias, rename) = table.value(inst_value, dest, dest_map[dest] > i);
            if let Some(rename) = rename {
                inst.dest = Some(rename);
            }

            if let Some(alias) = alias {
                *inst = Instruction {
                    dest: inst.dest.clone(),
                    op: Some("id".to_string()),
                    args: Some(vec![alias]),
                    ..Default::default()
                };
                continue;
            }
        }

        inst.args = inst
            .args
            .as_ref()
            .map(|args| args.iter().map(|arg| table.root(&arg)).collect::<Vec<_>>());
    }
}

fn drop_kill(instrs: &mut Vec<Instruction>) {
    let mut unused = HashMap::new();
    let mut kill = HashSet::new();

    for (i, inst) in instrs.iter().enumerate() {
        if let Some(args) = inst.args.as_ref() {
            for arg in args {
                unused.remove(arg);
            }
        }
        if let Some(dest) = inst.dest.as_ref() {
            if let Some(old) = unused.insert(dest, i) {
                kill.insert(old);
            }
        }
    }

    *instrs = instrs
        .into_iter()
        .enumerate()
        .filter_map(|(i, inst)| {
            if kill.contains(&i) {
                None
            } else {
                Some(inst.clone())
            }
        })
        .collect();
}

fn dot(bril: &Bril) -> String {
    use std::fmt::Write;

    let mut dot = String::new();

    for function in &bril.functions {
        writeln!(dot, "digraph {} {{", function.name).unwrap();

        let basic_blocks = BasicBlock::new_blocks(&function.instrs);

        for block in &basic_blocks {
            let label = block[0].label.as_deref().unwrap();
            writeln!(dot, "  {label};").unwrap();
        }

        for block in &basic_blocks {
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

    pub fn bril2json(src: &str) -> String {
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

    pub fn bril2txt(src: &str) -> String {
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

    pub fn brili(src: &str) -> (String, usize) {
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
                .unwrap_or(114514),
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

    #[test]
    fn test_lvn() {
        glob!("..", "tests/examples/lvn/*.bril", |path| {
            let txt = std::fs::read_to_string(&path).unwrap();
            let json = bril2json(&txt);
            let mut bril: Bril = serde_json::from_str(&json).unwrap();

            for function in &mut bril.functions {
                let mut partition = partition(&function.instrs);
                partition.iter_mut().for_each(|p| {
                    local_value_numbering(p);
                    drop_kill(p)
                });
                function.instrs = partition.into_iter().flatten().collect();
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
