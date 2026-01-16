use crate::server::message::RegionMessage;
use crate::server::region::add_debug_value;
use crate::vm::*;
use crate::{EntityAction, Item, PixelSource, PlayerCamera, RegionCtx, Value};
use rand::Rng;
use theframework::prelude::TheValue;
use vek::Vec2;

struct RegionHost<'a> {
    ctx: &'a mut RegionCtx,
}

impl<'a> HostHandler for RegionHost<'a> {
    fn on_host_call(&mut self, name: &str, args: &[VMValue]) -> Option<VMValue> {
        match name {
            "action" => {
                if let Some(s) = args.get(0).and_then(|v| v.as_string()) {
                    if let Ok(action) = s.parse::<EntityAction>() {
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
            "intent" => {
                if let Some(s) = args.get(0).and_then(|v| v.as_string()) {
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
            "message" => {
                if let (Some(receiver), Some(msg)) =
                    (args.get(0), args.get(1).and_then(|v| v.as_string()))
                {
                    let category = args
                        .get(2)
                        .and_then(|v| v.as_string())
                        .unwrap_or("")
                        .to_string();

                    let mut entity_id = Some(self.ctx.curr_entity_id);
                    let item_id = self.ctx.curr_item_id;
                    if item_id.is_some() {
                        entity_id = None;
                    }

                    let msg = RegionMessage::Message(
                        self.ctx.region_id,
                        entity_id,
                        item_id,
                        receiver.x as u32,
                        msg.to_string(),
                        category,
                    );
                    if let Some(sender) = self.ctx.from_sender.get() {
                        let _ = sender.send(msg);
                    }

                    if self.ctx.debug_mode {
                        add_debug_value(&mut self.ctx, TheValue::Text("Ok".into()), false);
                    }
                }
            }
            "set_player_camera" => {
                if let Some(entity) = self.ctx.get_current_entity_mut() {
                    if let Some(camera) = args.get(0).and_then(|v| v.as_string()) {
                        let player_camera = match camera {
                            "iso" => PlayerCamera::D3Iso,
                            "firstp" => PlayerCamera::D3FirstP,
                            _ => PlayerCamera::D2,
                        };
                        entity.set_attribute("player_camera", Value::PlayerCamera(player_camera));
                    }
                }
            }
            "set_debug_loc" => {
                if let (Some(event), Some(x), Some(y)) = (
                    args.get(0).and_then(|v| v.as_string()),
                    args.get(1),
                    args.get(2),
                ) {
                    let x = x.x as u32;
                    let y = y.x as u32;
                    self.ctx.curr_debug_loc = Some((event.to_string(), x, y));
                }
            }
            "set_tile" => {
                if let (Some(mode), Some(item_id)) = (
                    args.get(0).and_then(|v| v.as_string()),
                    self.ctx.curr_item_id,
                ) {
                    if let Ok(uuid) = theframework::prelude::Uuid::try_parse(mode) {
                        if let Some(item) = self.ctx.get_item_mut(item_id) {
                            item.set_attribute("source", Value::Source(PixelSource::TileId(uuid)));
                        }
                    }
                }
            }
            "set_emit_light" => {
                let active = args.get(0).map(|v| v.is_truthy()).unwrap_or(false);
                if let Some(item_id) = self.ctx.curr_item_id {
                    if let Some(item) = self.ctx.get_item_mut(item_id) {
                        if let Some(Value::Light(light)) = item.attributes.get_mut("light") {
                            light.active = active;
                            item.mark_dirty_attribute("light");
                        }
                    }
                } else if let Some(entity) = self.ctx.get_current_entity_mut() {
                    if let Some(Value::Light(light)) = entity.attributes.get_mut("light") {
                        light.active = active;
                        entity.mark_dirty_attribute("light");
                    }
                }
            }
            "set_attr" => {
                if let (Some(key), Some(val)) =
                    (args.get(0).and_then(|v| v.as_string()), args.get(1))
                {
                    if let Some(item_id) = self.ctx.curr_item_id {
                        if let Some(item) = self.ctx.get_item_mut(item_id) {
                            item.set_attribute(key, vmvalue_to_value(val));
                        }
                    } else if let Some(entity) = self.ctx.get_current_entity_mut() {
                        entity.set_attribute(key, vmvalue_to_value(val));
                    }
                }
            }
            "toggle_attr" => {
                if let Some(key) = args.get(0).and_then(|v| v.as_string()) {
                    if let Some(item_id) = self.ctx.curr_item_id {
                        if let Some(item) = self.ctx.get_item_mut(item_id) {
                            item.attributes.toggle(key);
                        }
                    } else if let Some(entity) = self.ctx.get_current_entity_mut() {
                        entity.attributes.toggle(key);
                    }
                }
            }
            "id" => {
                return Some(VMValue::broadcast(self.ctx.curr_entity_id as f32));
            }
            "get_attr_of" => {
                if let (Some(id_val), Some(key)) =
                    (args.get(0), args.get(1).and_then(|v| v.as_string()))
                {
                    let id = id_val.x as u32;
                    if let Some(entity) = self.ctx.get_entity_mut(id) {
                        if let Some(v) = entity.attributes.get(key).cloned() {
                            return Some(VMValue::from_value(&v));
                        }
                    } else if let Some(item) = self.ctx.get_item_mut(id) {
                        if let Some(v) = item.attributes.get(key).cloned() {
                            return Some(VMValue::from_value(&v));
                        }
                    }
                }
            }
            "get_attr" => {
                if let Some(key) = args.get(0).and_then(|v| v.as_string()) {
                    if let Some(item_id) = self.ctx.curr_item_id {
                        if let Some(item) = self.ctx.get_item_mut(item_id) {
                            if let Some(v) = item.attributes.get(key).cloned() {
                                return Some(VMValue::from_value(&v));
                            }
                        }
                    } else if let Some(entity) = self.ctx.get_current_entity_mut() {
                        if let Some(v) = entity.attributes.get(key).cloned() {
                            return Some(VMValue::from_value(&v));
                        }
                    }
                }
            }
            "random" => {
                // random(min, max) inclusive; fallback to 0..1 if missing args
                if let (Some(a), Some(b)) = (args.get(0), args.get(1)) {
                    let mut lo = a.x as i32;
                    let mut hi = b.x as i32;
                    if lo > hi {
                        std::mem::swap(&mut lo, &mut hi);
                    }
                    let mut rng = rand::rng();
                    let r: i32 = rng.random_range(lo..=hi);
                    return Some(VMValue::broadcast(r as f32));
                } else {
                    let r: f32 = rand::random();
                    return Some(VMValue::broadcast(r));
                }
            }
            "notify_in" => {
                if let (Some(mins), Some(notification)) =
                    (args.get(0), args.get(1).and_then(|v| v.as_string()))
                {
                    let minutes = mins.x as i32;
                    let target_tick =
                        self.ctx.ticks + (self.ctx.ticks_per_minute as i32 * minutes) as i64;
                    if let Some(item_id) = self.ctx.curr_item_id {
                        self.ctx.notifications_items.push((
                            item_id,
                            target_tick,
                            notification.to_string(),
                        ));
                    } else {
                        self.ctx.notifications_entities.push((
                            self.ctx.curr_entity_id,
                            target_tick,
                            notification.to_string(),
                        ));
                    }
                }
            }
            "random_walk" => {
                // distance, speed, max_sleep
                let distance = args.get(0).map(|v| v.x).unwrap_or(1.0);
                let speed = args.get(1).map(|v| v.x).unwrap_or(1.0);
                let max_sleep = args.get(2).map(|v| v.x as i32).unwrap_or(0);
                if let Some(entity) = self.ctx.get_current_entity_mut() {
                    entity.action =
                        EntityAction::RandomWalk(distance, speed, max_sleep, 0, Vec2::zero());
                }
            }
            "random_walk_in_sector" => {
                let distance = args.get(0).map(|v| v.x).unwrap_or(1.0);
                let speed = args.get(1).map(|v| v.x).unwrap_or(1.0);
                let max_sleep = args.get(2).map(|v| v.x as i32).unwrap_or(0);
                if let Some(entity) = self.ctx.get_current_entity_mut() {
                    entity.action = EntityAction::RandomWalkInSector(
                        distance,
                        speed,
                        max_sleep,
                        0,
                        Vec2::zero(),
                    );
                }
            }
            "set_proximity_tracking" => {
                let turn_on = args.get(0).map(|v| v.is_truthy()).unwrap_or(false);
                let distance = args.get(1).map(|v| v.x).unwrap_or(5.0);
                if let Some(item_id) = self.ctx.curr_item_id {
                    if turn_on {
                        self.ctx.item_proximity_alerts.insert(item_id, distance);
                    } else {
                        self.ctx.item_proximity_alerts.remove(&item_id);
                    }
                } else {
                    let entity_id = self.ctx.curr_entity_id;
                    if turn_on {
                        self.ctx.entity_proximity_alerts.insert(entity_id, distance);
                    } else {
                        self.ctx.entity_proximity_alerts.remove(&entity_id);
                    }
                }
            }
            "set_rig_sequence" => {
                // Not yet modeled; ignore.
            }
            "take" => {
                if let Some(item_id) = args.get(0).map(|v| v.x as u32) {
                    if let Some(pos) = self.ctx.map.items.iter().position(|i| i.id == item_id) {
                        let item = self.ctx.map.items.remove(pos);
                        if let Some(entity) = self.ctx.get_current_entity_mut() {
                            let _ = entity.add_item(item); // ignore errors (full inventory)
                        }
                    }
                }
            }
            "equip" => {
                if let Some(item_id) = args.get(0).map(|v| v.x as u32) {
                    if let Some(slot) = self
                        .ctx
                        .get_current_entity_mut()
                        .and_then(|e| e.get_item(item_id))
                        .and_then(|it| it.attributes.get_str("slot").map(|s| s.to_string()))
                    {
                        if let Some(entity) = self.ctx.get_current_entity_mut() {
                            let _ = entity.equip_item(item_id, &slot);
                        }
                    }
                }
            }
            "inventory_items" => {
                if let Some(entity) = self.ctx.get_current_entity_mut() {
                    let filter = args
                        .get(0)
                        .and_then(|v| v.as_string())
                        .unwrap_or("")
                        .to_string();
                    let ids: Vec<u32> = entity
                        .iter_inventory()
                        .filter(|(_, it)| {
                            filter.is_empty()
                                || it
                                    .attributes
                                    .get_str("name")
                                    .map(|n| n.contains(&filter))
                                    .unwrap_or(false)
                                || it
                                    .attributes
                                    .get_str("class_name")
                                    .map(|c| c.contains(&filter))
                                    .unwrap_or(false)
                        })
                        .map(|(_, i)| i.id)
                        .collect();
                    let ids_str: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
                    let mut v = VMValue::zero();
                    if let Some(id0) = ids.get(0) {
                        v.x = *id0 as f32;
                    }
                    if let Some(id1) = ids.get(1) {
                        v.y = *id1 as f32;
                    }
                    v.z = ids.len() as f32;
                    v.string = Some(ids_str.join(","));
                    return Some(v);
                }
            }
            "inventory_items_of" => {
                if let Some(entity_id) = args.get(0).map(|v| v.x as u32) {
                    if let Some(entity) = self.ctx.get_entity_mut(entity_id) {
                        let filter = args
                            .get(1)
                            .and_then(|v| v.as_string())
                            .unwrap_or("")
                            .to_string();
                        let ids: Vec<u32> = entity
                            .iter_inventory()
                            .filter(|(_, it)| {
                                filter.is_empty()
                                    || it
                                        .attributes
                                        .get_str("name")
                                        .map(|n| n.contains(&filter))
                                        .unwrap_or(false)
                                    || it
                                        .attributes
                                        .get_str("class_name")
                                        .map(|c| c.contains(&filter))
                                        .unwrap_or(false)
                            })
                            .map(|(_, i)| i.id)
                            .collect();
                        let ids_str: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
                        let mut v = VMValue::zero();
                        if let Some(id0) = ids.get(0) {
                            v.x = *id0 as f32;
                        }
                        if let Some(id1) = ids.get(1) {
                            v.y = *id1 as f32;
                        }
                        v.z = ids.len() as f32;
                        v.string = Some(ids_str.join(","));
                        return Some(v);
                    }
                }
            }
            "entities_in_radius" => {
                let mut ids: Vec<u32> = Vec::new();
                let pos = if let Some(item_id) = self.ctx.curr_item_id {
                    self.ctx.get_item_mut(item_id).map(|i| i.get_pos_xz())
                } else {
                    self.ctx.get_current_entity_mut().map(|e| e.get_pos_xz())
                };
                if let Some(pos) = pos {
                    for e in &self.ctx.map.entities {
                        if e.id != self.ctx.curr_entity_id {
                            if pos.distance(e.get_pos_xz()) < 1.0 {
                                ids.push(e.id);
                            }
                        }
                    }
                }
                let ids_str: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
                let mut v = VMValue::zero();
                if let Some(id0) = ids.get(0) {
                    v.x = *id0 as f32;
                }
                if let Some(id1) = ids.get(1) {
                    v.y = *id1 as f32;
                }
                v.z = ids.len() as f32;
                v.string = Some(ids_str.join(","));
                return Some(v);
            }
            "list_get" => {
                // list is arg0 (comma-separated string), index is arg1
                let idx = args.get(1).map(|v| v.x as i32).unwrap_or(0);
                if let Some(list_str) = args.get(0).and_then(|v| v.as_string()) {
                    let parts: Vec<&str> = list_str.split(',').filter(|s| !s.is_empty()).collect();
                    if parts.is_empty() {
                        return Some(VMValue::zero());
                    }
                    let clamped = if idx < 0 {
                        0
                    } else if (idx as usize) >= parts.len() {
                        parts.len() - 1
                    } else {
                        idx as usize
                    };
                    if let Ok(val) = parts[clamped].parse::<f32>() {
                        return Some(VMValue::broadcast(val));
                    }
                    return Some(VMValue::zero());
                }
            }
            "deal_damage" => {
                if let (Some(target), Some(amount)) = (args.get(0), args.get(1)) {
                    let id = target.x as u32;
                    let dmg = amount.x as i32;
                    let attr = self.ctx.health_attr.clone();
                    if let Some(entity) = self.ctx.get_entity_mut(id) {
                        if let Some(mut health) = entity.attributes.get_int(&attr) {
                            health = (health - dmg).max(0);
                            entity.set_attribute(&attr, Value::Int(health));
                            if health == 0 {
                                entity.set_attribute("mode", Value::Str("dead".into()));
                                entity.action = EntityAction::Off;
                            }
                        }
                    }
                }
            }
            "took_damage" => {
                // already applied to current entity; nothing to return
            }
            "block_events" => {
                if let (Some(minutes), Some(event)) =
                    (args.get(0), args.get(1).and_then(|v| v.as_string()))
                {
                    let target_tick =
                        self.ctx.ticks + (self.ctx.ticks_per_minute as f32 * minutes.x) as i64;
                    if let Some(item_id) = self.ctx.curr_item_id {
                        if let Some(state) = self.ctx.item_state_data.get_mut(&item_id) {
                            state.set(event, Value::Int64(target_tick));
                        }
                    } else {
                        let eid = self.ctx.curr_entity_id;
                        if let Some(state) = self.ctx.entity_state_data.get_mut(&eid) {
                            state.set(event, Value::Int64(target_tick));
                        }
                    }
                }
            }
            "add_item" => {
                if let Some(class_name) = args.get(0).and_then(|v| v.as_string()) {
                    // Minimal: create blank item with class_name and push into inventory
                    if let Some(entity) = self.ctx.get_current_entity_mut() {
                        let mut item = Item {
                            id: crate::server::region::get_global_id(),
                            ..Default::default()
                        };
                        item.set_attribute("class_name", Value::Str(class_name.to_string()));
                        item.set_attribute("name", Value::Str(class_name.to_string()));
                        let _ = entity.add_item(item);
                    }
                }
            }
            "drop_items" => {
                if let Some(filter) = args.get(0).and_then(|v| v.as_string()) {
                    if let Some(entity) = self.ctx.get_current_entity_mut() {
                        let ids: Vec<u32> = entity
                            .iter_inventory()
                            .filter(|(_, it)| {
                                let name = it.attributes.get_str("name").unwrap_or_default();
                                let class_name =
                                    it.attributes.get_str("class_name").unwrap_or_default();
                                filter.is_empty()
                                    || name.contains(filter)
                                    || class_name.contains(filter)
                            })
                            .map(|(_, it)| it.id)
                            .collect();
                        let mut removed_items = Vec::new();
                        for id in ids {
                            if let Some(pos) = entity
                                .inventory
                                .iter()
                                .position(|opt| opt.as_ref().map(|i| i.id) == Some(id))
                            {
                                if let Some(item) = entity.remove_item_from_slot(pos) {
                                    removed_items.push(item);
                                }
                            }
                        }
                        let _ = entity;
                        self.ctx.map.items.extend(removed_items);
                    }
                }
            }
            "offer_inventory" => {
                // Not modeled; ignore.
            }
            "drop" => {
                if let Some(item_id) = args.get(0).map(|v| v.x as u32) {
                    if let Some(entity) = self.ctx.get_current_entity_mut() {
                        if let Some(pos) = entity
                            .inventory
                            .iter()
                            .position(|opt| opt.as_ref().map(|i| i.id) == Some(item_id))
                        {
                            if let Some(mut item) = entity.remove_item_from_slot(pos) {
                                item.position = entity.position;
                                item.mark_all_dirty();
                                self.ctx.map.items.push(item);
                            }
                        }
                    }
                }
            }
            "teleport" => {
                // destination, region (ignored)
                if let Some(dest) = args.get(0).and_then(|v| v.as_string()) {
                    let center = {
                        let map = &self.ctx.map;
                        map.sectors
                            .iter()
                            .find(|s| s.name == dest)
                            .and_then(|s| s.center(map))
                    };
                    if let (Some(center), Some(entity)) =
                        (center, self.ctx.get_current_entity_mut())
                    {
                        entity.set_pos_xz(center);
                    }
                }
            }
            "goto" => {
                if let Some(_dest) = args.get(0).and_then(|v| v.as_string()) {
                    let speed = args.get(1).map(|v| v.x).unwrap_or(1.0);
                    if let Some(entity) = self.ctx.get_current_entity_mut() {
                        entity.action = EntityAction::Goto(Vec2::new(0.0, 0.0), speed);
                        // destination resolution omitted
                    }
                }
            }
            "close_in" => {
                if let (Some(target), Some(radius), Some(speed)) =
                    (args.get(0), args.get(1), args.get(2))
                {
                    if let Some(entity) = self.ctx.get_current_entity_mut() {
                        entity.action = EntityAction::CloseIn(target.x as u32, radius.x, speed.x);
                    }
                }
            }
            "debug" => {
                // No-op for now.
            }
            _ => {}
        }
        None
    }
}

fn vmvalue_to_value(v: &VMValue) -> Value {
    v.to_value()
}

// Run an event
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

// Run a user_event
pub fn run_client_fn(
    exec: &mut Execution,
    args: &[VMValue],
    program: &crate::vm::Program,
    region_ctx: &mut RegionCtx,
) {
    if let Some(index) = program.user_functions_name_map.get("user_event").copied() {
        exec.reset(program.globals);
        let mut host = RegionHost { ctx: region_ctx };
        let _ret = exec.execute_function_host(args, index, program, &mut host);
    }
}
