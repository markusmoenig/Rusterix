# Terrain System - Quick Reference

## Map Properties

### Enable Terrain
```rust
map.properties.set("terrain_enabled", Value::Bool(true));
```

### Default Terrain Tile
```rust
map.properties.set("default_terrain_tile", Value::Source(pixel_source));
```
If not set, uses fallback tile `27826750-a9e7-4346-994b-fb318b238452`.

### Tile Overrides (Per 1x1 Tile)
```rust
map.properties.set("terrain_tiles", Value::TileOverrides(hashmap));
```
Same pattern as surface `tiles` property - sets texture per 1x1 UV cell.

---

## Vertex Properties

### Mark as Terrain Control Point
```rust
vertex.properties.set("terrain_control", Value::Bool(true));
```
Only vertices with this property affect terrain height.

### Height
```rust
vertex.z = 10.0;  // Sets terrain height at this point
```
The vertex Z coordinate becomes world Y (vertical height).

### Smoothness (Optional)
```rust
vertex.properties.set("smoothness", Value::Float(2.0));
```
- Higher value = wider, gentler influence area (more gradual slopes)
- Lower value = sharper, steeper peaks/valleys
- Default: `1.0` (from global config)

---

## Sector Properties

### Terrain Mode
```rust
sector.properties.set("terrain_mode", Value::Int(mode));
```

Controls how the sector interacts with terrain:
- **`0`** (default) - No special terrain interaction
- **`1`** - Exclude from terrain (cuts a hole for buildings, dungeons, interiors)
- **`2`** - Ridge mode (creates elevated ridges or valleys along sector boundaries)

#### Mode 1: Terrain Exclusion
```rust
sector.properties.set("terrain_mode", Value::Int(1));
```
Cuts a hole in terrain where all 3 triangle vertices are inside the sector.

**Note:** Triangles are excluded on an all-or-nothing basis. For smooth transitions, use terrain control vertices at sector boundaries with height=0.

#### Mode 2: Terrain Ridges
```rust
sector.properties.set("terrain_mode", Value::Int(2));
sector.properties.set("ridge_height", Value::Float(5.0));
sector.properties.set("ridge_plateau_width", Value::Float(2.0));
sector.properties.set("ridge_falloff_distance", Value::Float(8.0));
sector.properties.set("ridge_falloff_steepness", Value::Float(2.0));
```

Creates an elevated ridge (or valley if negative) following the sector's boundary edges with a flat plateau on top.

#### Ridge Parameters

- **`ridge_height`** (float, default: `1.0`)
  - Maximum height of the ridge at the plateau
  - Measured in world units (becomes Y coordinate in 3D)
  - Can be **positive** (ridges, walls) or **negative** (valleys, trenches, lakes)
  - Example: `5.0` creates a 5-unit tall ridge
  - Example: `-3.0` creates a 3-unit deep depression

- **`ridge_plateau_width`** (float, default: `0.0`)
  - Width of the flat top of the ridge, measured from sector edges
  - `0.0` = sharp peak with no plateau
  - `2.0` = 2-unit wide flat area on top
  - Useful for creating walkable paths or defensive walls

- **`ridge_falloff_distance`** (float, default: `5.0`)
  - Distance over which the ridge height falls from plateau to ground level
  - Controls how wide/narrow the ridge slopes are
  - Measured outward from the plateau edge
  - Example: `8.0` creates gentle, wide slopes

- **`ridge_falloff_steepness`** (float, default: `2.0`)
  - Controls the shape of the falloff curve
  - `1.0` = linear falloff (constant slope)
  - `2.0` = smooth quadratic falloff (natural looking)
  - `3.0+` = sharper falloff (cliff-like edges)
  - Higher values create steeper slopes near the top

#### Ridge Behavior

- Ridges are **additive** with vertex-based terrain heights
- Multiple ridge sectors can overlap and combine
- Ridge height is calculated based on **distance to nearest sector edge**
- Inside the sector polygon = full plateau height
- Outside the sector = smooth falloff based on distance

#### Example: City Wall

```rust
// Create a sector outlining the city wall path
wall_sector.properties.set("terrain_mode", Value::Int(2));               // Ridge mode
wall_sector.properties.set("ridge_height", Value::Float(4.0));          // 4 units tall
wall_sector.properties.set("ridge_plateau_width", Value::Float(1.5));   // 1.5 units wide top
wall_sector.properties.set("ridge_falloff_distance", Value::Float(3.0)); // Steep 3-unit slopes
wall_sector.properties.set("ridge_falloff_steepness", Value::Float(3.0)); // Cliff-like sides
```

#### Example: Mountain Range

```rust
// Create an elongated sector following the mountain ridge line
mountain_sector.properties.set("terrain_mode", Value::Int(2));           // Ridge mode
mountain_sector.properties.set("ridge_height", Value::Float(15.0));      // Tall mountain
mountain_sector.properties.set("ridge_plateau_width", Value::Float(0.0)); // Sharp peak
mountain_sector.properties.set("ridge_falloff_distance", Value::Float(20.0)); // Wide slopes
mountain_sector.properties.set("ridge_falloff_steepness", Value::Float(1.5));  // Natural slopes
```

#### Example: River or Lake

```rust
// Create a sector outlining a river bed or lake
river_sector.properties.set("terrain_mode", Value::Int(2));              // Ridge mode
river_sector.properties.set("ridge_height", Value::Float(-2.0));         // 2 units below ground
river_sector.properties.set("ridge_plateau_width", Value::Float(3.0));   // 3-unit wide flat bottom
river_sector.properties.set("ridge_falloff_distance", Value::Float(5.0)); // Gentle banks
river_sector.properties.set("ridge_falloff_steepness", Value::Float(1.5)); // Natural slope to water
```

**Note:** Negative heights work for creating depressions, valleys, trenches, rivers, and lakes. The falloff works the same way - it transitions smoothly from the low point back to ground level.

---

## Linedef Properties

### Terrain Smoothing (Roads, Paths)
```rust
linedef.properties.set("terrain_smooth", Value::Bool(true));
linedef.properties.set("terrain_width", Value::Float(3.0));
linedef.properties.set("terrain_falloff_distance", Value::Float(4.0));
linedef.properties.set("terrain_falloff_steepness", Value::Float(2.0));

// Set vertex heights to define the road elevation
start_vertex.z = 0.0;  // Road height at start
end_vertex.z = 2.0;    // Road height at end (slopes upward)
```

Linedefs with `terrain_smooth` enabled create smooth corridors of terrain along their path, perfect for roads, paths, and rivers that need to cut through hilly terrain. The height along the linedef is **interpolated from the start and end vertex Z coordinates**, allowing roads to smoothly slope up or down.

#### Linedef Terrain Parameters

- **`terrain_smooth`** (bool, default: `false`)
  - Enable terrain smoothing for this linedef
  - When `true`, the linedef creates a smooth corridor along its path
  - **Height is interpolated from start and end vertex Z coordinates**
  - Start vertex Z = road height at start point
  - End vertex Z = road height at end point
  - Height varies linearly along the linedef (smooth slopes)
  - Example: Both vertices at Z=0 → flat ground-level road
  - Example: Start Z=0, End Z=5 → upward sloping road
  - Example: Start Z=2, End Z=-1 → downward into a riverbed

- **`terrain_width`** (float, default: `2.0`)
  - Half-width of the flat terrain corridor (distance from linedef center)
  - `2.0` = 2 units on each side = 4 units total width
  - `3.0` = 6 units total width (wider road)
  - The corridor is perfectly flat at `terrain_target_height`

- **`terrain_falloff_distance`** (float, default: `3.0`)
  - Distance over which terrain transitions from corridor to natural height
  - Measured outward from the corridor edge
  - Controls how wide the transition zone is
  - Example: `4.0` creates gentle transitions to surrounding terrain

- **`terrain_falloff_steepness`** (float, default: `2.0`)
  - Controls the shape of the falloff curve
  - `1.0` = linear falloff (constant slope)
  - `2.0` = smooth quadratic falloff (natural looking, **default**)
  - `3.0+` = sharper falloff (road appears more "cut" into terrain)

#### Linedef Smoothing Behavior

- Linedefs smooth terrain **toward the target height** within their corridor
- Multiple linedefs can affect the same point and blend naturally
- Linedef smoothing is **applied after** vertex hills and sector ridges
- Works seamlessly across chunk boundaries
- Perfect for creating roads that traverse hilly terrain

#### Example: Ground-Level Road

```rust
// Set vertex heights for a flat ground-level road
start_vertex.z = 0.0;
end_vertex.z = 0.0;

// Create a linedef marking the road path
road_linedef.properties.set("terrain_smooth", Value::Bool(true));       // Enable smoothing
road_linedef.properties.set("terrain_width", Value::Float(2.5));        // 5 units wide road
road_linedef.properties.set("terrain_falloff_distance", Value::Float(3.0)); // 3-unit transition
road_linedef.properties.set("terrain_falloff_steepness", Value::Float(2.0)); // Natural slopes
```

#### Example: Elevated Causeway

```rust
// Set vertex heights for a raised road across a valley
start_vertex.z = 3.0;
end_vertex.z = 3.0;

// Create a raised road
causeway.properties.set("terrain_smooth", Value::Bool(true));
causeway.properties.set("terrain_width", Value::Float(2.0));           // 4 units wide
causeway.properties.set("terrain_falloff_distance", Value::Float(5.0)); // Wide transition
causeway.properties.set("terrain_falloff_steepness", Value::Float(1.5)); // Gentle slopes
```

#### Example: Riverbed

```rust
// Set vertex heights for a sunken river path
start_vertex.z = -1.5;
end_vertex.z = -1.5;

// Create a sunken river path
river_linedef.properties.set("terrain_smooth", Value::Bool(true));
river_linedef.properties.set("terrain_width", Value::Float(3.0));          // 6 units wide river
river_linedef.properties.set("terrain_falloff_distance", Value::Float(4.0)); // Gentle banks
river_linedef.properties.set("terrain_falloff_steepness", Value::Float(2.0)); // Natural slopes
```

#### Example: Mountain Pass

```rust
// Set vertex heights for a pass through mountains with a specific elevation
start_vertex.z = 8.0;
end_vertex.z = 8.0;

// Create a pass through mountains
pass_linedef.properties.set("terrain_smooth", Value::Bool(true));
pass_linedef.properties.set("terrain_width", Value::Float(1.5));           // Narrow pass
pass_linedef.properties.set("terrain_falloff_distance", Value::Float(2.0)); // Quick transition
pass_linedef.properties.set("terrain_falloff_steepness", Value::Float(3.0)); // Steep sides
```

#### Example: Sloping Road

```rust
// Set vertex heights for a road that slopes upward
start_vertex.z = 0.0;  // Start at ground level
end_vertex.z = 5.0;    // End 5 units higher

// Create an upward sloping road
road_linedef.properties.set("terrain_smooth", Value::Bool(true));
road_linedef.properties.set("terrain_width", Value::Float(2.0));           // 4 units wide
road_linedef.properties.set("terrain_falloff_distance", Value::Float(3.0)); // 3-unit transition
road_linedef.properties.set("terrain_falloff_steepness", Value::Float(2.0)); // Natural slopes
```

---

## How It Works

### Height Calculation Order

Terrain height at each point is calculated in this order:

1. **Vertex control points**: Create hills and valleys using IDW interpolation
2. **Ridge sectors**: Add height contributions from ridge sectors (additive)
3. **Linedef smoothing**: Smooth terrain toward target height in corridors (blending)
4. **Map edge falloff**: Terrain smoothly transitions to **height 0 at map boundaries**

### Height Interpolation (IDW - Inverse Distance Weighting)

1. **Smooth "sugar cone" hills**: Control points create natural, rounded hills with smooth falloff
2. **Ridge additive**: Ridge sector heights are added to base terrain height
3. **Linedef smoothing**: Roads blend terrain toward target height within their corridor
4. **Map edge falloff**: Terrain smoothly transitions to **height 0 at map boundaries** (not chunk boundaries)
5. **Seamless chunks**: Terrain is continuous across all chunk boundaries - no visible seams
6. **Multiple features**: Vertices, ridges, and linedefs blend naturally when they overlap

### Terrain Shape

The terrain creates smooth, natural-looking hills around control vertices:
- **At control point**: Full height (vertex.z value)
- **Moving away**: Height decreases smoothly based on distance and smoothness
- **At map edges**: Always height 0 (within `falloff_distance` of map boundary)
- **Between chunks**: Perfectly continuous, no discontinuities

### Exclusion Behavior

Sectors marked with `terrain_mode="exclude"`:
- Removes terrain triangles where **all 3 vertices** are inside the sector
- Boundary triangles remain (creates a sharp edge)
- For smooth transitions: Add terrain control vertices at sector perimeter with `z=0`

---

## Complete Example

```rust
// 1. Enable terrain for map
map.properties.set("terrain_enabled", Value::Bool(true));

// 2. Set default terrain tile
map.properties.set("default_terrain_tile", Value::Source(grass_tile));

// 3. (Optional) Set tile overrides
let mut tile_overrides = FxHashMap::default();
tile_overrides.insert((5, 5), dirt_tile);  // Dirt at world position (5, 5)
map.properties.set("terrain_tiles", Value::TileOverrides(tile_overrides));

// 4. Create terrain control points
let hill = map.create_vertex(50.0, 50.0, 8.0);
hill.properties.set("terrain_control", Value::Bool(true));
hill.properties.set("smoothness", Value::Float(2.0));  // Wide, gentle slopes

let peak = map.create_vertex(30.0, 30.0, 12.0);
peak.properties.set("terrain_control", Value::Bool(true));
peak.properties.set("smoothness", Value::Float(0.5));  // Sharp, steep peak

// 5. Exclude building from terrain (creates a hole)
building_sector.properties.set("terrain_mode", Value::Str("exclude".to_string()));

// 6. (Optional) Add control vertices at building entrance for smooth transition
for vertex in building_entrance_vertices {
    vertex.z = 0.0;  // Floor level
    vertex.properties.set("terrain_control", Value::Bool(true));
    vertex.properties.set("smoothness", Value::Float(1.5));
}
```

---

## Advanced Configuration (Code-level)

```rust
use rusterix::chunkbuilder::terrain_generator::TerrainConfig;

let config = TerrainConfig {
    subdivisions: 1,              // Grid subdivisions per world tile (1, 2, 4, etc.)
    idw_power: 2.0,               // Interpolation smoothness (1.0-4.0)
    max_influence_distance: 50.0, // Vertex influence range
    smoothness: 1.0,              // Default smoothness if vertex doesn't specify
};
```

### `subdivisions`
- `1` = 1 quad per world tile (default, fast)
- `2` = 4 quads per world tile (more detail)
- `4` = 16 quads per world tile (high detail, smoother hills)

### `idw_power`
- `1.0` = Linear falloff (very gentle, wide slopes)
- `2.0` = Quadratic falloff (natural "sugar cone" shape, **default**)
- `4.0` = Sharp falloff (steep, pronounced peaks)

### `max_influence_distance`
- Distance beyond which control vertices have no effect
- Larger = smoother transitions between distant points, more computation
- Smaller = localized features, sharper terrain changes, faster generation

### `smoothness` (per-vertex or global default)
- Scales the effective distance for influence calculation
- Higher values = wider influence area (gradual, rolling hills)
- Lower values = tighter influence area (sharp peaks/valleys)

---

## Tips & Best Practices

### Creating Natural Terrain

1. **Use varied smoothness**: Mix wide gentle hills (smoothness=2.0-3.0) with sharp peaks (smoothness=0.5-1.0)
2. **Layer control points**: Combine multiple heights for realistic terrain
3. **Map boundaries**: Terrain automatically fades to 0 at map edges within 10 world units

### Smooth Building Transitions

Instead of just excluding sectors (which creates hard edges):

```rust
// 1. Mark sector for exclusion
building_sector.properties.set("terrain_mode", Value::Str("exclude".to_string()));

// 2. Add control vertices around the building perimeter at floor height
for entrance_vertex in perimeter_vertices {
    entrance_vertex.z = 0.0;  // Match building floor
    entrance_vertex.properties.set("terrain_control", Value::Bool(true));
    entrance_vertex.properties.set("smoothness", Value::Float(1.5));
}
```

This creates a smooth ramp from the terrain down to the building floor instead of a vertical gap.

### Performance

- Use `subdivisions: 1` for most terrain (fast, looks good)
- Increase subdivisions only where you need fine detail
- Keep `max_influence_distance` reasonable (50.0 is a good default)
- More control points = more computation, but smoother blending

---

## Troubleshooting

### Flat terrain everywhere
- Ensure vertices have `terrain_control=true` property set
- Check that vertex Z values are non-zero
- Verify map has `terrain_enabled=true`

### Terrain has plateaus in each chunk
- This should not happen anymore - terrain is continuous across chunks
- If you see this, check that map bounding box is correct

### Gaps at sector boundaries
- Add terrain control vertices at sector perimeter with appropriate heights
- Set their smoothness to control transition steepness

### Hills too sharp/too gentle
- Adjust per-vertex `smoothness` property
- Lower smoothness = sharper peaks
- Higher smoothness = gentler, wider hills
