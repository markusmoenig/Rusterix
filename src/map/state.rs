use crate::Vertex;
use serde::{Deserialize, Serialize};
use vek::Vec2;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InterpolationType {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    Step,
}

impl InterpolationType {
    /// Adjusts progress based on the interpolation type
    pub fn adjust_progress(&self, progress: f32) -> f32 {
        match self {
            InterpolationType::Linear => progress,
            InterpolationType::EaseIn => progress * progress,
            InterpolationType::EaseOut => progress * (2.0 - progress),
            InterpolationType::EaseInOut => {
                if progress < 0.5 {
                    2.0 * progress * progress
                } else {
                    -1.0 + (4.0 - 2.0 * progress) * progress
                }
            }
            InterpolationType::Step => {
                if progress >= 1.0 {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VertexState {
    pub id: u32,
    pub position: Vec2<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimationVertexState {
    pub state_name: String,
    pub vertices: Vec<VertexState>,
    pub interpolation: InterpolationType,
}

impl AnimationVertexState {
    /// Updates the position of a vertex if it exists, or adds it if it doesn't
    pub fn update_or_add(&mut self, vertex_id: u32, new_position: Vec2<f32>) {
        if let Some(existing_vertex) = self.vertices.iter_mut().find(|v| v.id == vertex_id) {
            // Update the position of the existing vertex
            existing_vertex.position = new_position;
        } else {
            // Add a new vertex to the state
            self.vertices.push(VertexState {
                id: vertex_id,
                position: new_position,
            });
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VertexAnimationSystem {
    pub states: Vec<AnimationVertexState>, // All animation states
    pub current_state: Option<usize>,      // Index of the current state
    pub next_state: Option<usize>,         // Index of the next state for transitioning
    pub transition_duration: f32,          // Duration of transitions (in seconds)
    pub transition_progress: f32,          // Progress of the current transition (0.0 to 1.0)
    pub loop_states: Vec<usize>,           // Indices of states to loop between
    pub loop_duration: f32,                // Time for a full loop cycle
    pub loop_elapsed_time: f32,            // Elapsed time within the loop
    pub current_loop_index: usize,         // Current index in the loop
}

impl Default for VertexAnimationSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl VertexAnimationSystem {
    pub fn new() -> Self {
        Self {
            states: vec![],
            current_state: None,
            next_state: None,
            transition_duration: 0.0,
            transition_progress: 0.0,
            loop_states: vec![],
            loop_duration: 1.0,
            loop_elapsed_time: 0.0,
            current_loop_index: 0,
        }
    }

    /// Add an animation state
    /// Add an animation state with a specific interpolation type
    pub fn add_state(
        &mut self,
        state_name: &str,
        vertices: Vec<VertexState>,
        interpolation: InterpolationType,
    ) -> usize {
        self.states.push(AnimationVertexState {
            state_name: state_name.to_string(),
            vertices,
            interpolation,
        });
        self.states.len() - 1
    }

    /// Updates or adds a vertex to the specified animation state
    /// Does nothing if the state doesn't exist.
    pub fn update_or_add_to_state(
        &mut self,
        state_name: &str,
        vertex_id: u32,
        new_position: Vec2<f32>,
    ) {
        // Find the target state and call its update_or_add method
        if let Some(state) = self.states.iter_mut().find(|s| s.state_name == state_name) {
            state.update_or_add(vertex_id, new_position);
        }
    }

    /// Set a loop between animation states
    pub fn set_loop(&mut self, state_names: &[&str], duration: f32) {
        self.loop_states = state_names
            .iter()
            .filter_map(|name| self.states.iter().position(|s| &s.state_name == name))
            .collect();
        self.loop_duration = duration;
        self.loop_elapsed_time = 0.0;
        self.current_loop_index = 0;

        if let Some(first_index) = self.loop_states.first() {
            self.current_state = Some(*first_index);
        }
    }

    /// Transition to a specific animation state
    pub fn transition_to_state(&mut self, state_name: &str, duration: f32) {
        if let Some(index) = self.states.iter().position(|s| s.state_name == state_name) {
            self.next_state = Some(index);
            self.transition_duration = duration;
            self.transition_progress = 0.0;
        }
    }

    /// Updates the animation system and applies changes to the base vertices
    pub fn update(&mut self, delta_time: f32, base_vertices: &mut [Vertex]) {
        // Handle transitions
        if let Some(next_index) = self.next_state {
            self.transition_progress += delta_time / self.transition_duration;

            if self.transition_progress >= 1.0 {
                // Transition complete
                self.current_state = Some(next_index);
                self.next_state = None;
                self.transition_progress = 1.0;
            }

            // If transitioning from the base map to a state
            if self.current_state.is_none() {
                let blended_state = self.interpolate_with_base(
                    &self.states[next_index],
                    self.transition_progress,
                    base_vertices,
                );
                self.apply_state_to_base(&blended_state, base_vertices);
                return;
            }

            // If transitioning between two animation states
            if let Some(current_index) = self.current_state {
                let blended_state = self.interpolate_states(
                    &self.states[current_index],
                    &self.states[next_index],
                    self.transition_progress,
                );
                self.apply_state_to_base(&blended_state, base_vertices);
                return;
            }
        }

        // Handle looping states
        if !self.loop_states.is_empty() {
            self.loop_elapsed_time += delta_time;
            if self.loop_elapsed_time >= self.loop_duration / self.loop_states.len() as f32 {
                self.loop_elapsed_time = 0.0;
                self.current_loop_index = (self.current_loop_index + 1) % self.loop_states.len();
                self.current_state = Some(self.loop_states[self.current_loop_index]);
            }
        }

        // Apply the current state if no transition is happening
        if let Some(current_index) = self.current_state {
            if current_index < self.states.len() {
                self.apply_state_to_base(&self.states[current_index], base_vertices);
            }
        }
    }

    /// Interpolates between two states
    fn interpolate_states(
        &self,
        from: &AnimationVertexState,
        to: &AnimationVertexState,
        progress: f32,
    ) -> AnimationVertexState {
        let mut blended_vertices = Vec::new();

        let adjusted_progress = match to.interpolation {
            InterpolationType::Linear => progress,
            InterpolationType::EaseIn => progress * progress,
            InterpolationType::EaseOut => progress * (2.0 - progress),
            InterpolationType::EaseInOut => {
                if progress < 0.5 {
                    2.0 * progress * progress
                } else {
                    -1.0 + (4.0 - 2.0 * progress) * progress
                }
            }
            InterpolationType::Step => {
                if progress >= 1.0 {
                    1.0
                } else {
                    0.0
                }
            }
        };

        for (from_vertex, to_vertex) in from.vertices.iter().filter_map(|fv| {
            to.vertices
                .iter()
                .find(|tv| tv.id == fv.id)
                .map(|tv| (fv, tv))
        }) {
            blended_vertices.push(VertexState {
                id: from_vertex.id,
                position: Vec2::lerp(from_vertex.position, to_vertex.position, adjusted_progress),
            });
        }

        AnimationVertexState {
            state_name: format!("Blended: {} -> {}", from.state_name, to.state_name),
            vertices: blended_vertices,
            interpolation: to.interpolation.clone(), // Preserve the interpolation type
        }
    }

    /// Interpolates between the base map and a target animation state
    fn interpolate_with_base(
        &self,
        to: &AnimationVertexState,
        progress: f32,
        base_vertices: &[Vertex],
    ) -> AnimationVertexState {
        let mut blended_vertices = Vec::new();

        for to_vertex in &to.vertices {
            // Find the corresponding vertex in the base map
            if let Some(base_vertex) = base_vertices.iter().find(|v| v.id == to_vertex.id) {
                let base_position = Vec2::new(base_vertex.x, base_vertex.y);

                blended_vertices.push(VertexState {
                    id: to_vertex.id,
                    position: Vec2::lerp(base_position, to_vertex.position, progress),
                });
            } else {
                // If no matching vertex in the base map, use the animation state's position
                blended_vertices.push(to_vertex.clone());
            }
        }

        AnimationVertexState {
            state_name: format!("Blended: Base -> {}", to.state_name),
            vertices: blended_vertices,
            interpolation: to.interpolation.clone(),
        }
    }

    /// Applies an animation state to the base vertices
    fn apply_state_to_base(&self, state: &AnimationVertexState, base_vertices: &mut [Vertex]) {
        for vertex_state in &state.vertices {
            if let Some(base_vertex) = base_vertices.iter_mut().find(|v| v.id == vertex_state.id) {
                base_vertex.x = vertex_state.position.x;
                base_vertex.y = vertex_state.position.y;
            }
        }
    }
}
