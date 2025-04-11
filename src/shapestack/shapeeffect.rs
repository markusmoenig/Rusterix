use crate::{ShapeContext, ValueContainer};
use vek::Vec4;

#[derive(Debug, Clone)]
pub enum ShapeEffect {
    // Outline(ValueContainer),
    VerticalGradient(ValueContainer),
    // Glow(ValueContainer),
}

impl ShapeEffect {
    pub fn evaluate(&self, ctx: &ShapeContext) -> Option<Vec4<f32>> {
        match self {
            // ShapeEffect::Outline(props) => {
            //     let color = props.get_color("color").unwrap_or(Vec4::one());
            //     let thickness = props.get_float("thickness").unwrap_or(1.5);
            //     if ctx.distance < 0.0 && ctx.distance >= -thickness * ctx.px {
            //         color
            //     } else {
            //         Vec4::zero()
            //     }
            // }
            ShapeEffect::VerticalGradient(props) => {
                let alpha = 1.0 - ShapeEffect::smoothstep(-2.0, 0.0, ctx.distance);
                if alpha > 0.0 {
                    let top = Vec4::new(1.0, 1.0, 1.0, 1.0);
                    let bottom = Vec4::new(0.0, 0.0, 0.0, 1.0);
                    let t = ctx.uv.y.clamp(0.0, 1.0);
                    let mut c = top * (1.0 - t) + bottom * t;
                    c.w = alpha;
                    Some(c)
                } else {
                    None
                }
            } // ShapeEffect::Glow(props) => {
              //     let glow_color = props
              //         .get_color("color")
              //         .unwrap_or(Vec4::new(1.0, 1.0, 0.5, 1.0));
              //     let radius = props.get_float("radius").unwrap_or(4.0);
              //     let glow = 1.0 - smoothstep(0.0, radius * ctx.px, ctx.distance.max(0.0));
              //     glow_color * glow
              // }
        }
    }

    pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
        let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }
}
