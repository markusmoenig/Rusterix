use crate::Stmt;
use rustc_hash::FxHashMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct PreModule {
    pub name: String,
    pub source: String,
    pub path: PathBuf,

    pub globals: FxHashMap<String, u32>,
    pub stmts: Vec<Box<Stmt>>,
}

impl PreModule {
    pub fn new(
        name: String,
        source: String,
        path: PathBuf,
        stmts: Vec<Box<Stmt>>,
        globals: FxHashMap<String, u32>,
    ) -> Self {
        Self {
            name,
            source,
            path,
            stmts,
            globals,
        }
    }
}
