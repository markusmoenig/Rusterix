pub mod ast;
pub mod astvalue;
pub mod compile;
pub mod context;
pub mod environment;
pub mod errors;
pub mod idverifier;
pub mod module;
pub mod node;
pub mod objectd;
pub mod optimize;
pub mod parser;
pub mod scanner;
pub mod textures;

pub type Value = vek::Vec3<f32>;

pub use crate::{
    ast::{
        AssignmentOperator, BinaryOperator, ComparisonOperator, EqualityOperator, Expr, Location,
        LogicalOperator, Stmt, UnaryOperator, Visitor,
    },
    astvalue::ASTValue,
    compile::CompileVisitor,
    context::Context,
    environment::Environment,
    errors::{ParseError, RuntimeError},
    idverifier::IdVerifier,
    module::PreModule,
    node::execution::Execution,
    node::{nodeop::NodeOp, program::Program},
    optimize::optimize,
    parser::Parser,
    scanner::{Scanner, Token, TokenType},
    textures::{
        TexStorage,
        patterns::{PatternKind, ensure_patterns_initialized},
    },
};

use rustc_hash::FxHashMap;
use std::path::PathBuf;

pub struct Rusteria {
    path: PathBuf,
    pub context: Context,
    defaults: Option<PreModule>,
}

impl Default for Rusteria {
    fn default() -> Self {
        Self::new()
    }
}

impl Rusteria {
    pub fn new() -> Self {
        Self {
            path: PathBuf::new(),
            context: Context::new(FxHashMap::default()),
            defaults: None,
        }
    }

    // Parse the source code into a module.
    pub fn parse(&mut self, path: PathBuf) -> Result<PreModule, ParseError> {
        self.path = path.clone();
        let mut parser = Parser::new();
        let module = parser.compile(path.clone())?;

        Ok(module)
    }

    // Parse the source code into a module.
    pub fn parse_str(&mut self, str: &str) -> Result<PreModule, ParseError> {
        self.path = PathBuf::from("string_based.shpz");
        let mut parser: Parser = Parser::new();

        let module = parser.compile_module("main".into(), str.into(), self.path.clone())?;

        Ok(module)
    }

    // Compile the source code
    pub fn compile(&mut self, module: &PreModule) -> Result<(), RuntimeError> {
        let mut visitor: CompileVisitor = CompileVisitor::new();
        self.context = Context::new(module.globals.clone());

        // Add default materials
        if let Some(defs) = &self.defaults {
            for statement in defs.stmts.clone() {
                _ = statement.accept(&mut visitor, &mut self.context)?;
            }
        }

        for statement in module.stmts.clone() {
            _ = statement.accept(&mut visitor, &mut self.context)?;
        }

        // println!("{:?}", self.context.program.user_functions);
        optimize(&mut self.context.program.body);

        self.context.program.globals = self.context.globals.len();

        Ok(())
    }

    /// Compile the voxels into the VoxelGrid.
    pub fn execute(&mut self) -> Option<Value> {
        let mut execution = Execution::new(self.context.globals.len());

        // Execute the main program to compile all voxels.
        execution.execute(&&self.context.program.body, &self.context.program);

        execution.stack.pop()
    }

    pub fn execute_string(&mut self, str: &str) -> Option<Value> {
        let result = self.parse_str(str);
        match result {
            Ok(module) => {
                let result = self.compile(&module);
                match result {
                    Ok(_) => {
                        return self.execute();
                    }
                    Err(err) => println!("{}", err.to_string()),
                }
            }
            Err(err) => println!("{}", err.to_string()),
        }

        None
    }

    // /// Write the image to disc.
    // pub fn write_image(&self) {
    //     let mut path = self.path.clone();
    //     path.set_extension("png");

    //     let b = self.buffer.lock().unwrap();
    //     b.save_srgb(path.clone());
    // }

    // /// Write the image to an u array.
    // pub fn write_image_to_array(&self) -> Vec<u8> {
    //     let b = self.buffer.lock().unwrap();
    //     b.to_u8_vec_gamma()
    // }

    // /// Get the current time in ms.
    // pub fn get_time(&self) -> u128 {
    //     self.tracer.get_time()
    // }

    /// Imported paths
    pub fn imported_paths(&self) -> Vec<PathBuf> {
        self.context.imported_paths.clone()
    }

    /// Get the current time
    pub fn get_time(&self) -> u128 {
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::window().unwrap().performance().unwrap().now() as u128
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let stop = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards");
            stop.as_millis()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addition() {
        let mut script = Rusteria::default();
        let result = script.execute_string("let a = 2; a + 2;".into());
        assert_eq!(result.unwrap().x, 4.0);
    }

    #[test]
    fn fib() {
        let mut script = Rusteria::default();
        let fib = r#"
        fn fib(n) {
            if n <= 1 {
                return n;
            } else {
                return fib(n - 1) + fib(n - 2);
            }
        }
        fib(27);
        "#;
        let result = script.execute_string(fib.into());
        assert_eq!(result.unwrap().x, 196418.0);
    }
}
