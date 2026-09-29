use luna_mvir::Module;
use crate::pass::Pass;

pub struct PassManager {
    passes: Vec<Box<dyn Pass>>,
}

impl PassManager {
    pub fn new() -> Self {
        Self {
            passes: Vec::new(),
        }
    }

    pub fn add_pass(&mut self, pass: Box<dyn Pass>) {
        self.passes.push(pass);
    }

    pub fn run(&mut self, module: &mut Module) {
        // Run each pass once. Fixed-point iteration is a separate optimizer policy.
        for pass in &mut self.passes {
            for func in &mut module.functions {
                pass.run_on_function(func);
            }
        }
    }
}
