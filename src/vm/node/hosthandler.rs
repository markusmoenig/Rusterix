use crate::vm::{NodeOp, VMValue};

/// Host handler invoked for VM ops that need to touch external context.
pub trait HostHandler {
    fn on_action(&mut self, _v: &VMValue) {}
    fn on_intent(&mut self, _v: &VMValue) {}
    fn on_message(&mut self, _text: &VMValue, _category: &VMValue) {}

    /// Dispatch a NodeOp that targets the host layer. Returns true if handled.
    fn handle_host_op(&mut self, op: &NodeOp, stack: &mut Vec<VMValue>) -> bool {
        match op {
            NodeOp::Action => {
                if let Some(v) = stack.pop() {
                    self.on_action(&v);
                }
                true
            }
            NodeOp::Intent => {
                if let Some(v) = stack.pop() {
                    self.on_intent(&v);
                }
                true
            }
            NodeOp::Message => {
                let category = stack.pop().unwrap_or_else(VMValue::zero);
                let text = stack.pop().unwrap_or_else(VMValue::zero);
                self.on_message(&text, &category);
                true
            }
            _ => false,
        }
    }
}
