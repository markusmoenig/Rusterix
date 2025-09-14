use rustc_hash::FxHashMap;

use crate::NodeOp;
use std::sync::Arc;

#[derive(Clone)]
pub struct Program {
    /// The program body
    pub body: Vec<NodeOp>,

    /// Code of all user defined functions.
    pub user_functions: Vec<Arc<[NodeOp]>>,

    /// Map of user function names to their indices.
    pub user_functions_name_map: FxHashMap<String, usize>,

    /// Index of the shape function
    pub shade_index: Option<usize>,
}

impl Program {
    pub fn new() -> Self {
        Self {
            body: Vec::new(),
            user_functions: vec![],
            user_functions_name_map: FxHashMap::default(),
            shade_index: None,
        }
    }
}
