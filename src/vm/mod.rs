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
pub mod renderbuffer;
pub mod scanner;

use std::ops::{Add, Div, Mul, Neg, Sub};
use vek::Vec3;

#[derive(Clone, Debug, PartialEq)]
pub struct VMValue {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub string: Option<String>,
}

impl VMValue {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            x,
            y,
            z,
            string: None,
        }
    }

    pub fn broadcast(v: f32) -> Self {
        Self {
            x: v,
            y: v,
            z: v,
            string: None,
        }
    }

    pub fn zero() -> Self {
        Self::broadcast(0.0)
    }

    pub fn from_vec3(v: Vec3<f32>) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
            string: None,
        }
    }

    pub fn to_vec3(&self) -> Vec3<f32> {
        Vec3::new(self.x, self.y, self.z)
    }

    pub fn from_string<S: Into<String>>(s: S) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            string: Some(s.into()),
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        self.string.as_deref()
    }

    pub fn is_truthy(&self) -> bool {
        if let Some(s) = &self.string {
            !s.is_empty()
        } else {
            self.x != 0.0 || self.y != 0.0 || self.z != 0.0
        }
    }

    pub fn magnitude(&self) -> f32 {
        self.to_vec3().magnitude()
    }

    pub fn map<F: Fn(f32) -> f32>(&self, f: F) -> Self {
        VMValue::new(f(self.x), f(self.y), f(self.z))
    }

    pub fn map2<F: Fn(f32, f32) -> f32>(&self, other: VMValue, f: F) -> Self {
        VMValue::new(f(self.x, other.x), f(self.y, other.y), f(self.z, other.z))
    }

    pub fn dot(&self, other: VMValue) -> f32 {
        self.to_vec3().dot(other.to_vec3())
    }

    pub fn cross(&self, other: VMValue) -> Self {
        VMValue::from_vec3(self.to_vec3().cross(other.to_vec3()))
    }
}

impl Add for VMValue {
    type Output = VMValue;

    fn add(self, rhs: VMValue) -> Self::Output {
        match (self.string, rhs.string) {
            (Some(a), Some(b)) => VMValue::from_string(format!("{a}{b}")),
            _ => VMValue::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z),
        }
    }
}

impl Sub for VMValue {
    type Output = VMValue;

    fn sub(self, rhs: VMValue) -> Self::Output {
        VMValue::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Mul for VMValue {
    type Output = VMValue;

    fn mul(self, rhs: VMValue) -> Self::Output {
        VMValue::new(self.x * rhs.x, self.y * rhs.y, self.z * rhs.z)
    }
}

impl Div for VMValue {
    type Output = VMValue;

    fn div(self, rhs: VMValue) -> Self::Output {
        VMValue::new(self.x / rhs.x, self.y / rhs.y, self.z / rhs.z)
    }
}

impl Neg for VMValue {
    type Output = VMValue;

    fn neg(self) -> Self::Output {
        VMValue::new(-self.x, -self.y, -self.z)
    }
}

pub use self::{
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
    module::Module,
    node::execution::Execution,
    node::{nodeop::NodeOp, program::Program},
    optimize::optimize,
    parser::Parser,
    renderbuffer::RenderBuffer,
    scanner::{Scanner, Token, TokenType},
};

use rustc_hash::FxHashMap;
use std::path::PathBuf;
use theframework::theui::ThePalette;

pub struct VM {
    path: PathBuf,
    pub context: Context,
    defaults: Option<Module>,
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

impl VM {
    pub fn new() -> Self {
        Self {
            path: PathBuf::new(),
            context: Context::new(FxHashMap::default()),
            defaults: None,
        }
    }

    // Parse the source code into a module.
    pub fn parse(&mut self, path: PathBuf) -> Result<Module, ParseError> {
        self.path = path.clone();
        let mut parser = Parser::new();
        let module = parser.compile(path.clone())?;

        Ok(module)
    }

    // Parse the source code into a module.
    pub fn parse_str(&mut self, str: &str) -> Result<Module, ParseError> {
        self.path = PathBuf::from("string_based.shpz");
        let mut parser: Parser = Parser::new();

        let module = parser.compile_module("main".into(), str.into(), self.path.clone())?;

        Ok(module)
    }

    // Compile the source code
    pub fn compile(&mut self, module: &Module) -> Result<(), RuntimeError> {
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
    pub fn execute(&mut self, palette: &ThePalette) -> Option<VMValue> {
        let mut execution = Execution::new(self.context.globals.len());

        // Execute the main program to compile all voxels.
        execution.execute(&&self.context.program.body, &self.context.program, palette);

        execution.stack.pop()
    }

    pub fn execute_string(&mut self, str: &str, palette: &ThePalette) -> Option<VMValue> {
        let result = self.parse_str(str);
        match result {
            Ok(module) => {
                let result = self.compile(&module);
                match result {
                    Ok(_) => {
                        return self.execute(palette);
                    }
                    Err(err) => println!("{}", err.to_string()),
                }
            }
            Err(err) => println!("{}", err.to_string()),
        }

        None
    }

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
        let mut script = VM::default();
        let result = script.execute_string("let a = 2; a + 2;".into(), &ThePalette::default());
        assert_eq!(result.unwrap().x, 4.0);
    }

    #[test]
    fn fib() {
        let mut script = VM::default();
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
        let result = script.execute_string(fib.into(), &ThePalette::default());
        assert_eq!(result.unwrap().x, 196418.0);
    }

    #[test]
    fn string_literal() {
        let mut script = VM::default();
        let result = script.execute_string(
            r#"let greeting = "hello"; greeting;"#,
            &ThePalette::default(),
        );
        assert_eq!(result.unwrap().as_string(), Some("hello"));
    }

    #[test]
    fn string_compare_literal() {
        let mut script = VM::default();
        let result = script.execute_string(
            r#"let name = "abc"; name == "abc";"#,
            &ThePalette::default(),
        );
        assert_eq!(result.unwrap().x, 1.0);
    }

    #[test]
    fn ternary_string() {
        let mut script = VM::default();
        let result = script.execute_string(
            r#"let flag = 1; flag ? "yes" : "no";"#,
            &ThePalette::default(),
        );
        assert_eq!(result.unwrap().as_string(), Some("yes"));
    }
}
