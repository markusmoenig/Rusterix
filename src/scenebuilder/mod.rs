pub mod d2builder;
pub mod d2material;
pub mod d2preview;
pub mod d3builder;

use crate::{
    CompiledLight, Map, Material, MaterialRole, PixelSource, ShapeFXRole, Value, ValueContainer,
};
use vek::Vec3;

/// Gets a material from a geometry graph
pub fn get_material_from_geo_graph(properties: &ValueContainer, map: &Map) -> Option<Material> {
    if let Some(Value::Source(PixelSource::ShapeFXGraphId(graph_id))) =
        properties.get("region_graph")
    {
        if let Some(graph) = map.shapefx_graphs.get(graph_id) {
            let nodes = graph.collect_nodes_from(0, 1);
            for node in nodes {
                if graph.nodes[node as usize].role == ShapeFXRole::Material {
                    let material_type = graph.nodes[node as usize]
                        .values
                        .get_int_default("material_type", 0);
                    let material_value = graph.nodes[node as usize]
                        .values
                        .get_float_default("material_value", 1.0);
                    if let Some(material_type) = MaterialRole::from_u8(material_type as u8) {
                        return Some(Material::new(material_type, material_value));
                    }
                }
            }
        }
    }

    None
}

/// Gets a light from a geometry graph
pub fn get_light_from_geo_graph(
    properties: &ValueContainer,
    terminal: usize,
    map: &Map,
) -> Option<CompiledLight> {
    if let Some(Value::Source(PixelSource::ShapeFXGraphId(graph_id))) =
        properties.get("region_graph")
    {
        if let Some(graph) = map.shapefx_graphs.get(graph_id) {
            let nodes = graph.collect_nodes_from(0, terminal);
            for node in nodes {
                if graph.nodes[node as usize].role == ShapeFXRole::PointLight {
                    return graph.nodes[node as usize].compile_light(Vec3::zero());
                }
            }
        }
    }

    None
}
