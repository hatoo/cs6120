// See https://github.com/banach-space/llvm-tutor/blob/main/HelloWorld/HelloWorld.cpp
// for a more detailed explanation.

use llvm_plugin::inkwell::context::Context;
use llvm_plugin::inkwell::values::{
    BasicValue, FunctionValue, InstructionOpcode, InstructionValue, IntValue,
};
use llvm_plugin::utils::InstructionIterator;
use llvm_plugin::{
    FunctionAnalysisManager, LlvmFunctionPass, PassBuilder, PipelineParsing, PreservedAnalyses,
};

#[llvm_plugin::plugin(name = "HelloWorld", version = "0.1")]
fn plugin_registrar(builder: &mut PassBuilder) {
    builder.add_function_pipeline_parsing_callback(|name, manager| {
        if name == "hello-world" {
            dbg!(name);
            manager.add_pass(HelloWorldPass);
            PipelineParsing::Parsed
        } else {
            PipelineParsing::NotParsed
        }
    });
    /*
    builder.add_module_pipeline_parsing_callback(|name, manager| {
        if name == "hello-world" {
            dbg!(name);
            manager.add_pass(HelloWorldPass);
            PipelineParsing::Parsed
        } else {
            PipelineParsing::NotParsed
        }
    });
    */
}

struct HelloWorldPass;

/*
impl llvm_plugin::LlvmModulePass for HelloWorldPass {
    fn run_pass(
        &self,
        module: &mut llvm_plugin::inkwell::module::Module<'_>,
        manager: &llvm_plugin::ModuleAnalysisManager,
    ) -> PreservedAnalyses {
        dbg!("called");
        for f in module.get_functions() {
            dbg!(f);
        }
        PreservedAnalyses::All
    }
}
*/
impl LlvmFunctionPass for HelloWorldPass {
    fn run_pass(
        &self,
        function: &mut FunctionValue,
        _manager: &FunctionAnalysisManager,
    ) -> PreservedAnalyses {
        // Replace the first add with mul
        for bb in &function.get_basic_blocks() {
            for instr in InstructionIterator::new(bb) {
                if instr.get_opcode() == InstructionOpcode::Add {
                    let context = bb.get_context();
                    let builder = context.create_builder();

                    let lhs = instr.get_operand(0).unwrap();
                    let rhs = instr.get_operand(1).unwrap();

                    builder.position_before(&instr);
                    let mul = builder.build_int_nsw_mul(
                        lhs.expect_left("value").into_int_value(),
                        rhs.expect_left("value").into_int_value(),
                        "mul",
                    );

                    instr.replace_all_uses_with(mul.as_instruction().as_ref().unwrap());

                    return PreservedAnalyses::None;
                }
            }
        }

        PreservedAnalyses::All
    }
}
