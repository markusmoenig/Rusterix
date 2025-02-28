use rand::*;
use rustpython::vm::*;
use vek::Vec2;

/// Generate an i32 or f32 random number within the given range.
pub fn random_in_range(
    from: PyObjectRef,
    to: PyObjectRef,
    vm: &VirtualMachine,
) -> PyResult<PyObjectRef> {
    if from.class().is(vm.ctx.types.int_type) && to.class().is(vm.ctx.types.int_type) {
        // Extract integers
        let start: i32 = from.try_into_value(vm)?;
        let end: i32 = to.try_into_value(vm)?;

        // Generate a random i32 within the range
        let mut rng = rand::thread_rng();
        let result = rng.gen_range(start..=end);

        Ok(vm.ctx.new_int(result).into())
    } else if from.class().is(vm.ctx.types.float_type) && to.class().is(vm.ctx.types.float_type) {
        // Extract floats
        let start: f64 = from.try_into_value(vm)?;
        let end: f64 = to.try_into_value(vm)?;

        // Generate a random f64 within the range
        let mut rng = rand::thread_rng();
        let result = rng.gen_range(start..=end);

        Ok(vm.ctx.new_float(result).into())
    } else {
        // If the inputs are not valid numbers, raise a TypeError
        Err(vm.new_type_error("Both from and to must be integers or floats".to_string()))
    }
}

/// Find a random poition max_distance away from pos.
pub fn find_random_position(pos: Vec2<f32>, max_distance: f32) -> Vec2<f32> {
    let mut rng = rand::thread_rng();
    let angle = rng.gen_range(0.0..std::f32::consts::TAU);
    let dx = max_distance * angle.cos();
    let dy = max_distance * angle.sin();
    Vec2::new(pos.x + dx, pos.y + dy)
}

/// Get an i32 value from an Python object with a default fallback.
pub fn get_i32(value: PyObjectRef, default: i32, vm: &VirtualMachine) -> i32 {
    if value.class().is(vm.ctx.types.int_type) {
        value.try_into_value::<i32>(vm).unwrap_or(default)
    } else if value.class().is(vm.ctx.types.float_type) {
        value
            .try_into_value::<f32>(vm)
            .map(|v| v as i32)
            .unwrap_or(default) // Convert f32 to i32
    } else {
        default
    }
}

/// Get an f32 value from an Python object with a default fallback.
pub fn get_f32(value: PyObjectRef, default: f32, vm: &VirtualMachine) -> f32 {
    if value.class().is(vm.ctx.types.int_type) {
        value
            .try_into_value::<i32>(vm)
            .map(|v| v as f32)
            .unwrap_or(default)
    } else {
        value.try_into_value::<f32>(vm).unwrap_or(default)
    }
}
