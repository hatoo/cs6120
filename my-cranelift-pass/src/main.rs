fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use cranelift_codegen::{
        cursor::Cursor,
        ir::{InstBuilderBase, InstInserterBase, InstructionData, Opcode},
        isa::TargetIsa,
        settings::{self, Configurable},
        Context,
    };
    use cranelift_reader::parse_functions;
    use insta::assert_display_snapshot;
    use target_lexicon::triple;

    fn isa() -> Arc<dyn TargetIsa> {
        let mut shared_builder = settings::builder();
        shared_builder.set("opt_level", "speed").unwrap();
        let shared_flags = settings::Flags::new(shared_builder);
        cranelift_codegen::isa::lookup(triple!("x86_64"))
            .unwrap()
            .finish(shared_flags)
            .unwrap()
    }

    fn add_to_mul(ctx: &mut Context) {
        let mut cursor = cranelift_codegen::cursor::FuncCursor::new(&mut ctx.func);

        while let Some(_block) = cursor.next_block() {
            while let Some(inst) = cursor.next_inst() {
                let mut cursor = &mut cursor;
                let dfg = cursor.data_flow_graph_mut();
                let data: cranelift_codegen::ir::InstructionData = dfg.insts[inst];

                match data {
                    InstructionData::Binary { opcode, args } if opcode == Opcode::Iadd => {
                        let ctrl_typevar = dfg.ctrl_typevar(inst);
                        dfg.replace(inst).build(
                            InstructionData::Binary {
                                opcode: Opcode::Imul,
                                args,
                            },
                            ctrl_typevar,
                        );
                        return;
                    }
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn test() {
        const SRC: &str = r#"
        function %f(i32, i32) -> i32 {

            block0(v0: i32, v1: i32):
                v2 = iadd v0, v1
                return v2
        }
        "#;

        let functions = parse_functions(SRC).unwrap();

        let mut ctx = Context::for_function(functions[0].clone());

        add_to_mul(&mut ctx);

        assert_display_snapshot!(format!("{:?}\n{:?}", &functions[0], &ctx.func));
    }
}
