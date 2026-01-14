use crate::vm::node::nodeop::NodeOp;
use rustc_hash::FxHashMap;

/// Simple registry of built-in functions (name -> (arity, op)).
#[derive(Clone)]
pub struct Builtins {
    map: FxHashMap<String, (u8, NodeOp)>,
}

impl Builtins {
    pub fn new() -> Self {
        Self {
            map: FxHashMap::default(),
        }
    }

    pub fn insert<S: Into<String>>(&mut self, name: S, arity: u8, op: NodeOp) {
        self.map.insert(name.into(), (arity, op));
    }

    pub fn get(&self, name: &str) -> Option<(u8, NodeOp)> {
        self.map.get(name).cloned()
    }

    pub fn entries(&self) -> impl Iterator<Item = (&String, &(u8, NodeOp))> {
        self.map.iter()
    }
}

impl Default for Builtins {
    fn default() -> Self {
        let mut b = Builtins::new();
        b.insert("length", 1, NodeOp::Length);
        b.insert("length2", 1, NodeOp::Length2);
        b.insert("length3", 1, NodeOp::Length3);
        b.insert("abs", 1, NodeOp::Abs);
        b.insert("sin", 1, NodeOp::Sin);
        b.insert("sin1", 1, NodeOp::Sin1);
        b.insert("sin2", 1, NodeOp::Sin2);
        b.insert("cos", 1, NodeOp::Cos);
        b.insert("cos1", 1, NodeOp::Cos1);
        b.insert("cos2", 1, NodeOp::Cos2);
        b.insert("normalize", 1, NodeOp::Normalize);
        b.insert("tan", 1, NodeOp::Tan);
        b.insert("atan", 1, NodeOp::Atan);
        b.insert("atan2", 2, NodeOp::Atan2);
        b.insert("rotate2d", 2, NodeOp::Rotate2D);
        b.insert("dot", 2, NodeOp::Dot);
        b.insert("dot2", 2, NodeOp::Dot2);
        b.insert("dot3", 2, NodeOp::Dot3);
        b.insert("cross", 2, NodeOp::Cross);
        b.insert("floor", 1, NodeOp::Floor);
        b.insert("ceil", 1, NodeOp::Ceil);
        b.insert("round", 1, NodeOp::Round);
        b.insert("fract", 1, NodeOp::Fract);
        b.insert("mod", 2, NodeOp::Mod);
        b.insert("degrees", 1, NodeOp::Degrees);
        b.insert("radians", 1, NodeOp::Radians);
        b.insert("min", 2, NodeOp::Min);
        b.insert("max", 2, NodeOp::Max);
        b.insert("mix", 3, NodeOp::Mix);
        b.insert("smoothstep", 3, NodeOp::Smoothstep);
        b.insert("step", 2, NodeOp::Step);
        b.insert("clamp", 3, NodeOp::Clamp);
        b.insert("sqrt", 1, NodeOp::Sqrt);
        b.insert("log", 1, NodeOp::Log);
        b.insert("pow", 2, NodeOp::Pow);
        // print is variadic; arity handled in compiler
        b.insert("print", 0, NodeOp::Print(0));
        b.insert("set_debug_loc", 3, NodeOp::SetDebugLoc);
        b.insert("set_player_camera", 1, NodeOp::SetPlayerCamera);
        b.insert("action", 1, NodeOp::Action);
        b.insert("intent", 1, NodeOp::Intent);
        b.insert("message", 2, NodeOp::Message);
        // format is variadic; arity handled specially in compiler.
        b.insert("format", 0, NodeOp::Format(0));
        b
    }
}
