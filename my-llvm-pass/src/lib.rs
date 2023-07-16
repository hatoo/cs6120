// See https://github.com/banach-space/llvm-tutor/blob/main/HelloWorld/HelloWorld.cpp
// for a more detailed explanation.

use llvm_plugin::inkwell::values::FunctionValue;
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
        eprintln!("(llvm-tutor) Hello from: {:?}", function.get_name());
        eprintln!(
            "(llvm-tutor)   number of arguments: {}",
            function.count_params()
        );
        PreservedAnalyses::All
    }
}
