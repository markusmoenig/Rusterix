use crate::{ShapeContext, Value, ValueContainer};
use std::fmt;
use std::str::FromStr;
use theframework::prelude::*;
use uuid::Uuid;
use vek::Vec4;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ShapeFXRole {
    Geometry,
    // Outline(ValueContainer),
    Gradient,
    // Glow(ValueContainer),
}

impl fmt::Display for ShapeFXRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ShapeFXRole::Geometry => "Geometry",
            ShapeFXRole::Gradient => "Gradient",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for ShapeFXRole {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Geometry" => Ok(ShapeFXRole::Geometry),
            "Gradient" => Ok(ShapeFXRole::Gradient),
            _ => Err(()),
        }
    }
}

impl ShapeFXRole {
    pub fn iterator() -> impl Iterator<Item = ShapeFXRole> {
        [ShapeFXRole::Geometry, ShapeFXRole::Gradient]
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
        let mut values = ValueContainer::default();

        match role {
            Gradient => {
                values.set("direction", Value::Float(0.0));
                values.set("from", Value::Int(0));
                values.set("to", Value::Int(1));
            }
            _ => {}
        }

        Self {
            id: Uuid::new_v4(),
            role,
            values,
            position: Vec2::new(20, 20),
        }
    }

    pub fn name(&self) -> String {
        match self.role {
            Geometry => "Geometry".into(),
            Gradient => "Gradient".into(),
        }
    }

    pub fn inputs(&self) -> Vec<TheNodeTerminal> {
        match self.role {
            Geometry => {
                vec![]
            }
            Gradient => {
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
            Gradient => {
                vec![TheNodeTerminal {
                    name: "out".into(),
                    color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                }]
            } // _ => vec![],
        }
    }

    pub fn evaluate(&self, ctx: &ShapeContext, palette: &ThePalette) -> Option<Vec4<f32>> {
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
            Gradient => {
                let alpha = 1.0 - ShapeFX::smoothstep(-2.0, 0.0, ctx.distance);
                if alpha > 0.0 {
                    let mut from = Vec4::zero();
                    let top_index = self.values.get_int_default("from", 0);
                    if let Some(Some(top_color)) = palette.colors.get(top_index as usize) {
                        from = top_color.to_vec4();
                    }
                    let mut to = Vec4::zero();
                    let bottom_index = self.values.get_int_default("to", 1);
                    if let Some(Some(bottom_color)) = palette.colors.get(bottom_index as usize) {
                        to = bottom_color.to_vec4();
                    }

                    let angle_rad =
                        (90.0 - self.values.get_float_default("direction", 0.0)).to_radians();
                    let dir = Vec2::new(angle_rad.cos(), angle_rad.sin());
                    let centered_uv = ctx.uv - Vec2::new(0.5, 0.5);
                    let projection = centered_uv.dot(dir);
                    let t =
                        (projection / std::f32::consts::FRAC_1_SQRT_2 * 0.5 + 0.5).clamp(0.0, 1.0);
                    let mut c = from * (1.0 - t) + to * t;
                    /*
                    c.w = 1.0;
                    if let Some(index) = palette.find_closest_color_index(&TheColor::from(c)) {
                        if let Some(Some(col)) = palette.colors.get(index) {
                            c = col.to_vec4();
                        }
                    }*/
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
