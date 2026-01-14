use crate::vm::*;
use crate::{EntityAction, PlayerCamera, RegionCtx, Value};

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

    /// Set the player camera mode which maps abstract movement input ("left" etc) to movement.
    fn on_set_player_camera(&mut self, mode: &VMValue) {
        if let Some(entity) = self.ctx.get_current_entity_mut() {
            if let Some(camera) = &mode.string {
                let player_camera = match camera.as_str() {
                    "iso" => PlayerCamera::D3Iso,
                    "firstp" => PlayerCamera::D3FirstP,
                    _ => PlayerCamera::D2,
                };
                entity.set_attribute("player_camera", Value::PlayerCamera(player_camera));
            }
        }
    }
}

// Usage when executing a compiled function:
pub fn run_server_fn(
    exec: &mut Execution,
    args: &[VMValue],
    program: &crate::vm::Program,
    region_ctx: &mut RegionCtx,
) {
    if let Some(index) = program.user_functions_name_map.get("event").copied() {
        exec.reset(program.globals);
        let mut host = RegionHost { ctx: region_ctx };
        let _ret = exec.execute_function_host(args, index, program, &mut host);
    }
}
