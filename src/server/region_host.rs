use crate::vm::node::hosthandler::HostHandler;
use crate::vm::{Execution, VMValue};
use crate::{EntityAction, RegionCtx, Value};

struct RegionHost<'a> {
    ctx: &'a mut RegionCtx,
}

impl<'a> HostHandler for RegionHost<'a> {
    fn on_action(&mut self, v: &VMValue) {
        if let Some(s) = v.as_string() {
            if let Ok(action) = s.parse::<EntityAction>() {
                // Example: set current entity’s action
                if let Some(ent) = self
                    .ctx
                    .map
                    .entities
                    .iter_mut()
                    .find(|e| e.id == self.ctx.curr_entity_id)
                {
                    ent.action = action;
                }
            }
        }
    }

    fn on_intent(&mut self, v: &VMValue) {
        if let Some(s) = v.as_string() {
            if let Some(ent) = self
                .ctx
                .map
                .entities
                .iter_mut()
                .find(|e| e.id == self.ctx.curr_entity_id)
            {
                ent.set_attribute("intent", Value::Str(s.to_string()));
            }
        }
    }

    fn on_message(&mut self, text: &VMValue, category: &VMValue) {
        if let (Some(msg), Some(cat)) = (text.as_string(), category.as_string()) {
            // Route to your RegionMessage channel or log as needed
            println!("Message to {} [{}]: {}", self.ctx.curr_entity_id, cat, msg);
        }
    }
}

// Usage when executing a compiled function:
fn run_server_fn(
    exec: &mut Execution,
    func_index: usize,
    args: &[VMValue],
    program: &crate::vm::Program,
    region_ctx: &mut RegionCtx,
) {
    let mut host = RegionHost { ctx: region_ctx };
    let _ret = exec.execute_function_host(args, func_index, program, &mut host);
}
