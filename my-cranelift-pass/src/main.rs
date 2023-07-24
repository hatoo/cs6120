fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use cranelift_codegen::{
        cursor::{Cursor, FuncCursor},
        data_value::DataValue,
        ir::{Function, InstBuilderBase, InstInserterBase, InstructionData, Opcode, Type, Value},
        isa::TargetIsa,
        settings::{self, Configurable},
        Context,
    };
    use cranelift_interpreter::{
        environment::FunctionStore,
        interpreter::{Interpreter, InterpreterState},
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

    /// replace the first add to mul
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

    fn loop_invariant_code_motion(ctx: &mut Context) {
        ctx.flowgraph();
        ctx.compute_loop_analysis();

        for lp in ctx.loop_analysis.loops() {
            let header = ctx.loop_analysis.loop_header(lp);
            let pre_header = ctx.func.dfg.make_block();

            let mut cursor = FuncCursor::new(&mut ctx.func);
            let block_call = cursor.func.dfg.block_call(header, &[]);

            cursor.insert_block(pre_header);
            cursor.ins().build(
                InstructionData::Jump {
                    opcode: Opcode::Jump,
                    destination: block_call,
                },
                cranelift_codegen::ir::types::INVALID,
            );

            let dfg = &mut ctx.func.dfg;
            for pred in ctx.cfg.pred_iter(header) {
                for dest in dfg.insts[pred.inst].branch_destination_mut(&mut dfg.jump_tables) {
                    if dest.block(&dfg.value_lists) == header {
                        dest.set_block(pre_header, &mut dfg.value_lists);
                    }
                }
            }
        }
    }

    fn call_i32(func: &Function, v: i32) -> i32 {
        let mut function_store = FunctionStore::default();
        function_store.add("f".to_string(), func);

        let interpreter_state = InterpreterState::default().with_function_store(function_store);
        let mut interpreter = Interpreter::new(interpreter_state);

        let control_flow = interpreter.call_by_name("f", &[DataValue::I32(v)]).unwrap();

        match control_flow {
            cranelift_interpreter::step::ControlFlow::Return(d) => d[0].clone().try_into().unwrap(),
            _ => panic!(),
        }
    }

    #[test]
    fn test_add_to_mul() {
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

    #[test]
    fn test_loop_invariant_code_motion() {
        // Add pre-header
        const SRC: &str = r#"
        function %f(i32) -> i32 {

            block0(v0: i32):
                v1 = iconst.i32 0
                v2 = icmp eq v0, v1
                brif v2, block1, block2
            
            block1:
                jump block3 ; loop header
            
            block2:
                jump block3 ; loop header

            block3(v5: i32):
                v6 = icmp eq v5, v1
                v7 = iconst.i32 1
                v8 = isub v5, v7
                brif v6, block3(v7), block4

            block4:
                return v8
        }
        "#;

        let functions = parse_functions(SRC).unwrap();

        let mut ctx = Context::for_function(functions[0].clone());

        loop_invariant_code_motion(&mut ctx);

        assert_display_snapshot!(format!("{:?}\n{:?}", &functions[0], &ctx.func));
    }
}
