use crate::{ShapeContext, ValueContainer};
use std::fmt;
use std::str::FromStr;
use theframework::prelude::*;
use uuid::Uuid;
use vek::Vec4;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ShapeFXRole {
    Geometry,
    // Outline(ValueContainer),
    VerticalGradient,
    // Glow(ValueContainer),
}

impl fmt::Display for ShapeFXRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ShapeFXRole::Geometry => "Geometry",
            ShapeFXRole::VerticalGradient => "Gradient",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for ShapeFXRole {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Geometry" => Ok(ShapeFXRole::Geometry),
            "Gradient" => Ok(ShapeFXRole::VerticalGradient),
            _ => Err(()),
        }
    }
}

impl ShapeFXRole {
    pub fn iterator() -> impl Iterator<Item = ShapeFXRole> {
        [ShapeFXRole::Geometry, ShapeFXRole::VerticalGradient]
            .iter()
            .copied()
    }
}

use ShapeFXRole::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeFX {
    pub id: Uuid,
    pub role: ShapeFXRole,
    pub values: ValueContainer,

    pub position: Vec2<i32>,
}

impl ShapeFX {
    pub fn new(role: ShapeFXRole) -> Self {
        Self {
            id: Uuid::new_v4(),
            role,
            values: ValueContainer::default(),
            position: Vec2::new(20, 20),
        }
    }

    pub fn name(&self) -> String {
        match self.role {
            Geometry => "Geometry".into(),
            VerticalGradient => "Gradient".into(),
        }
    }

    pub fn inputs(&self) -> Vec<TheNodeTerminal> {
        match self.role {
            Geometry => {
                vec![]
            }
            VerticalGradient => {
                vec![TheNodeTerminal {
                    name: "in".into(),
                    color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                }]
            }
        }
    }

    pub fn outputs(&self) -> Vec<TheNodeTerminal> {
        match self.role {
            Geometry => {
                vec![TheNodeTerminal {
                    name: "out".into(),
                    color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                }]
            }
            VerticalGradient => {
                vec![TheNodeTerminal {
                    name: "out".into(),
                    color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                }]
            } // _ => vec![],
        }
    }

    pub fn evaluate(&self, ctx: &ShapeContext) -> Option<Vec4<f32>> {
        match self.role {
            Geometry => None,
            // ShapeEffect::Outline(props) => {
            //     let color = props.get_color("color").unwrap_or(Vec4::one());
            //     let thickness = props.get_float("thickness").unwrap_or(1.5);
            //     if ctx.distance < 0.0 && ctx.distance >= -thickness * ctx.px {
            //         color
            //     } else {
            //         Vec4::zero()
            //     }
            // }
            VerticalGradient => {
                let alpha = 1.0 - ShapeFX::smoothstep(-2.0, 0.0, ctx.distance);
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
