use rand::*;
use rustpython::vm::*;

/// Generate an i32 or f32 random number within the given range.
pub fn random(from: PyObjectRef, to: PyObjectRef, vm: &VirtualMachine) -> PyResult<PyObjectRef> {
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
