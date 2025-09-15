use vek::Vec3;

use crate::{NodeOp, Program, Value};

#[derive(Clone)]
pub struct Execution {
    /// Global variables. The parser keeps count of all global variables and we allocate the array on creation.
    pub globals: Vec<Value>,

    /// Local variables used inside function bodies.
    locals: Vec<Value>,

    /// The locals state for recursive functions
    locals_stack: Vec<Vec<Value>>,

    /// The execution stack.
    pub stack: Vec<Value>,

    /// Function return value.
    return_value: Option<Value>,

    /// UV
    pub uv: Value,

    /// Input color
    pub input: Value,

    /// Normal
    pub normal: Value,

    /// Hitpoint
    pub hitpoint: Value,

    /// Time
    pub time: Value,
}

impl Execution {
    pub fn new(var_size: usize) -> Self {
        Self {
            globals: vec![Value::zero(); var_size],
            locals: vec![],
            locals_stack: vec![],
            stack: Vec::with_capacity(8),
            return_value: None,
            uv: Vec3::zero(),
            input: Vec3::zero(),
            normal: Vec3::zero(),
            hitpoint: Vec3::zero(),
            time: Vec3::zero(),
        }
    }

    pub fn new_from_var(execution: &Execution) -> Self {
        Self {
            globals: execution.globals.clone(),
            locals: vec![],
            locals_stack: vec![],
            stack: Vec::with_capacity(8),
            return_value: None,
            uv: Vec3::zero(),
            input: Vec3::zero(),
            normal: Vec3::zero(),
            hitpoint: Vec3::zero(),
            time: Vec3::zero(),
        }
    }

    /// When switching between programs we need to resize the count of global variables.
    #[inline]
    pub fn reset(&mut self, var_size: usize) {
        if var_size != self.globals.len() {
            self.globals.resize(var_size, Value::zero());
        }
    }

    pub fn execute(&mut self, code: &[NodeOp], program: &Program) {
        for op in code {
            // Unwind if return is set
            if self.return_value.is_some() {
                break;
            }
            match op {
                NodeOp::LoadGlobal(index) => {
                    self.stack.push(self.globals[*index]);
                }
                NodeOp::StoreGlobal(index) => {
                    self.globals[*index] = self.stack.pop().unwrap();
                }
                NodeOp::LoadLocal(index) => {
                    self.stack.push(self.locals[*index]);
                }
                NodeOp::StoreLocal(index) => {
                    self.locals[*index] = self.stack.pop().unwrap();
                }
                NodeOp::Swap => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(b);
                    self.stack.push(a);
                }
                NodeOp::GetComponents(swizzle) => {
                    let v = self.stack.pop().unwrap();
                    let mut result = vec![];

                    for &index in swizzle {
                        let f = match index {
                            0 => v.x,
                            1 => v.y,
                            2 => v.z,
                            _ => continue,
                        };
                        result.push(f);
                    }

                    // Push as single Value: scalar or vector depending on result length
                    let pushed = match result.as_slice() {
                        [x] => Value::broadcast(*x),
                        [x, y] => Value::new(*x, *y, 0.0),
                        [x, y, z] => Value::new(*x, *y, *z),
                        _ => Value::broadcast(0.0),
                    };

                    self.stack.push(pushed);
                }
                NodeOp::SetComponents(swizzle) => {
                    let value = self.stack.pop().unwrap();
                    let mut target = self.stack.pop().unwrap();

                    let components = match swizzle.len() {
                        1 => vec![value.x],
                        2 => vec![value.x, value.y],
                        3 => vec![value.x, value.y, value.z],
                        _ => vec![],
                    };

                    for (i, &idx) in swizzle.iter().enumerate() {
                        if i >= components.len() {
                            break;
                        }
                        match idx {
                            0 => target.x = components[i],
                            1 => target.y = components[i],
                            2 => target.z = components[i],
                            _ => {}
                        }
                    }

                    self.stack.push(target);
                }
                NodeOp::Push(v) => self.stack.push(*v),
                NodeOp::Clear => _ = self.stack.pop(),
                NodeOp::FunctionCall(arity, total_locals, index) => {
                    self.push_locals_state();
                    self.locals = vec![Value::zero(); *total_locals as usize];

                    // Arguments are on stack in call order
                    for index in (0..*arity as usize).rev() {
                        if let Some(arg) = self.stack.pop() {
                            self.locals[index] = arg;
                        }
                    }

                    // Save the stack position
                    let stack_base = self.stack.len();

                    // Execute the function body
                    let body = program.user_functions[*index].clone(); // Arc clone
                    self.execute(&body, program);

                    // Retrieve the return value. A function always returns exactly one value.
                    let ret = if self.return_value.is_some() {
                        self.return_value.take().unwrap_or(Value::zero())
                    } else if self.stack.len() > stack_base {
                        self.stack.pop().unwrap()
                    } else {
                        Value::zero()
                    };

                    // Clean up temporaries
                    while self.stack.len() > stack_base {
                        _ = self.stack.pop();
                    }

                    self.pop_locals_state();

                    // Push the return value
                    self.stack.push(ret);
                }
                NodeOp::Return => {
                    // Prefer the top of stack as the explicit return expression.
                    // If nothing was pushed (e.g., miscompiled branch), fall back to an
                    // existing return_value (from a deeper recursive call), else zero.
                    let v = if let Some(top) = self.stack.pop() {
                        top
                    } else if let Some(prev) = self.return_value.take() {
                        prev
                    } else {
                        Value::zero()
                    };
                    self.return_value = Some(v);
                    break;
                }
                NodeOp::Pack2 => {
                    let y = self.stack.pop().unwrap();
                    let x = self.stack.pop().unwrap();
                    self.stack.push(Value::new(x.x, y.x, 0.0));
                }
                NodeOp::Pack3 => {
                    let z = self.stack.pop().unwrap();
                    let y = self.stack.pop().unwrap();
                    let x = self.stack.pop().unwrap();
                    self.stack.push(Value::new(x.x, y.x, z.x));
                }
                NodeOp::Dup => {
                    if let Some(top) = self.stack.last() {
                        self.stack.push(*top);
                    }
                }
                NodeOp::If(then_code, else_code) => {
                    let value = self.stack.pop().unwrap().x != 0.0;
                    if value {
                        self.execute(then_code, program);
                    } else if let Some(else_code) = else_code {
                        self.execute(else_code, program);
                    }
                }
                // Math
                NodeOp::Add => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a + b);
                }
                NodeOp::Sub => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a - b);
                }
                NodeOp::Mul => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a * b);
                }
                NodeOp::Div => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a / b);
                }
                NodeOp::Length => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(Value::broadcast(a.magnitude()));
                }
                NodeOp::Abs => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.map(|x| x.abs()));
                }
                NodeOp::Sin => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.map(|x| x.sin()));
                }
                NodeOp::Cos => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.map(|x| x.cos()));
                }
                NodeOp::Normalize => {
                    let a = self.stack.pop().unwrap();
                    let len = a.magnitude();
                    self.stack.push(if len > 0.0 {
                        a / Value::broadcast(len)
                    } else {
                        a
                    });
                }
                NodeOp::Tan => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.map(|x| x.tan()));
                }
                NodeOp::Atan => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.map(|x| x.atan()));
                }
                NodeOp::Atan2 => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::new(a.x.atan2(b.x), a.y.atan2(b.y), a.z.atan2(b.z)));
                }
                NodeOp::Dot => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(Value::broadcast(a.dot(b)));
                }
                NodeOp::Cross => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.cross(b));
                }
                NodeOp::Floor => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.map(|x| x.floor()));
                }
                NodeOp::Ceil => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.map(|x| x.ceil()));
                }
                NodeOp::Fract => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.map(|x| x.fract()));
                }
                NodeOp::Mod => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(Value::new(a.x % b.x, a.y % b.y, a.z % b.z));
                }
                NodeOp::Radians => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.map(|x| x.to_radians()));
                }
                NodeOp::Degrees => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.map(|x| x.to_degrees()));
                }
                NodeOp::Min => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z)));
                }
                NodeOp::Max => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z)));
                }
                NodeOp::Mix => {
                    let c: Value = self.stack.pop().unwrap(); // t
                    let b: Value = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    // mix(a,b,t) = a*(1-t) + b*t, all component-wise
                    self.stack.push(a + (b - a) * c);
                }
                NodeOp::Smoothstep => {
                    let c: Value = self.stack.pop().unwrap(); // x
                    let b: Value = self.stack.pop().unwrap(); // edge1
                    let a = self.stack.pop().unwrap(); // edge0
                    let t = ((c - a) / (b - a)).map(|x| x.clamp(0.0, 1.0));
                    self.stack
                        .push(t * t * (Value::broadcast(3.0) - Value::broadcast(2.0) * t));
                }
                NodeOp::Step => {
                    // step(edge, x): returns 0.0 if x < edge else 1.0 (per component)
                    let b: Value = self.stack.pop().unwrap(); // x
                    let a = self.stack.pop().unwrap(); // edge
                    self.stack.push(Value::new(
                        if b.x >= a.x { 1.0 } else { 0.0 },
                        if b.y >= a.y { 1.0 } else { 0.0 },
                        if b.z >= a.z { 1.0 } else { 0.0 },
                    ));
                }
                NodeOp::Clamp => {
                    let c: Value = self.stack.pop().unwrap(); // hi
                    let b: Value = self.stack.pop().unwrap(); // lo
                    let a = self.stack.pop().unwrap(); // x
                    self.stack.push(Value::new(
                        a.x.clamp(b.x, c.x),
                        a.y.clamp(b.y, c.y),
                        a.z.clamp(b.z, c.z),
                    ));
                }
                NodeOp::Sqrt => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.map(|x| x.sqrt()));
                }
                NodeOp::Log => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(a.map(|x| x.ln()));
                }
                NodeOp::Pow => {
                    let b: Value = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::new(a.x.powf(b.x), a.y.powf(b.y), a.z.powf(b.z)));
                }
                // Comparison (booleans encoded as splat(1.0) / splat(0.0), using .x lane)
                NodeOp::Eq => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::broadcast(if a.x == b.x { 1.0 } else { 0.0 }));
                }
                NodeOp::Ne => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::broadcast(if a.x != b.x { 1.0 } else { 0.0 }));
                }
                NodeOp::Lt => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::broadcast(if a.x < b.x { 1.0 } else { 0.0 }));
                }
                NodeOp::Le => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::broadcast(if a.x <= b.x { 1.0 } else { 0.0 }));
                }
                NodeOp::Gt => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::broadcast(if a.x > b.x { 1.0 } else { 0.0 }));
                }
                NodeOp::Ge => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::broadcast(if a.x >= b.x { 1.0 } else { 0.0 }));
                }
                // Logical (use .x lane)
                NodeOp::And => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(Value::broadcast(
                        ((a.x != 0.0) & (b.x != 0.0)) as i32 as f32,
                    ));
                }
                NodeOp::Or => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(Value::broadcast(
                        ((a.x != 0.0) | (b.x != 0.0)) as i32 as f32,
                    ));
                }
                // Unary
                NodeOp::Not => {
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::broadcast((a.x == 0.0) as i32 as f32));
                }
                NodeOp::Neg => {
                    let a = self.stack.pop().unwrap();
                    self.stack.push(-a);
                }
                NodeOp::Print => {
                    let a = self.stack.pop().unwrap();
                    println!("print: {:?}", a);
                }
                NodeOp::UV => {
                    self.stack.push(self.uv);
                }
                NodeOp::Input => {
                    self.stack.push(self.input);
                }
                NodeOp::Normal => {
                    self.stack.push(self.normal);
                }
                NodeOp::Hitpoint => {
                    self.stack.push(self.hitpoint);
                }
                NodeOp::Time => {
                    self.stack.push(self.time);
                }
            }
        }
    }

    // Push the current locals state when we enter a function.
    fn push_locals_state(&mut self) {
        self.locals_stack.push(self.locals.clone());
    }

    // Pop the last locals state when we exit a function.
    fn pop_locals_state(&mut self) {
        if let Some(state) = self.locals_stack.pop() {
            self.locals = state;
        }
    }

    /// Call a function with no arguments
    #[inline]
    pub fn execute_function_no_args(&mut self, index: usize, program: &Program) -> Value {
        // Reset state for this call
        self.stack.truncate(0);
        self.return_value = None;

        self.locals.resize(10, Value::zero());

        self.execute(&program.user_functions[index], program);

        // Prefer an explicit return value; else top of stack; else zero
        if let Some(ret) = self.return_value.take() {
            return ret;
        }
        if let Some(rc) = self.stack.pop() {
            rc
        } else {
            Value::zero()
        }
    }

    /// Call a function with arguments provided as a slice.
    #[inline]
    pub fn execute_function(&mut self, args: &[Value], index: usize, program: &Program) -> Value {
        // Reset state for this call
        self.stack.truncate(0);
        self.return_value = None;

        // Prepare locals without reallocating each time
        let argc = args.len();
        if self.locals.len() < argc {
            self.locals.resize(argc, Value::zero());
        }
        // Copy args into locals in order (0..argc)
        self.locals[..argc].clone_from_slice(args);

        self.execute(&program.user_functions[index], program);

        // Prefer an explicit return value; else top of stack; else zero
        if let Some(ret) = self.return_value.take() {
            return ret;
        }
        if let Some(rc) = self.stack.pop() {
            rc
        } else {
            Value::zero()
        }
    }
}
