use crate::{
    Cell, CellItem, DebugModule, Grid, GridCtx, ModuleType, cell::CellRole, cellitem::CellItemForm,
};
use theframework::prelude::*;

fn default_scale() -> f32 {
    1.0
}

fn default_rotation() -> f32 {
    0.0
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Routine {
    pub id: Uuid,
    pub name: String,

    pub module_offset: u32,
    pub visible: bool,
    pub folded: bool,

    #[serde(default)]
    pub pixelization: i32,

    #[serde(default = "default_scale")]
    pub scale: f32,

    #[serde(default = "default_rotation")]
    pub rotation: f32,

    #[serde(default)]
    pub color_steps: i32,

    pub screen_width: u32,

    #[serde(skip)]
    pub buffer: TheRGBABuffer,

    pub grid: Grid,
}

impl Routine {
    pub fn new(name: &str) -> Self {
        let mut grid = Grid::new();
        grid.insert((0, 0), CellItem::new(Cell::Empty));
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            module_offset: 0,
            visible: false,
            folded: false,
            screen_width: 100,
            buffer: TheRGBABuffer::new(TheDim::sized(100, 100)),
            grid,
            pixelization: 0,
            color_steps: 0,
            scale: default_scale(),
            rotation: default_rotation(),
        }
    }

    pub fn draw(
        &mut self,
        ctx: &TheContext,
        grid_ctx: &GridCtx,
        id: u32,
        debug: Option<&DebugModule>,
    ) {
        // Size check
        let size = self.grid.size(
            ctx,
            grid_ctx,
            self.folded,
            self.screen_width,
            &self.name,
            id,
            debug,
        );

        let width = size.x.max(self.screen_width);
        let height = size.y;

        if self.buffer.dim().width != width as i32 || self.buffer.dim().height != height as i32 {
            self.buffer = TheRGBABuffer::new(TheDim::sized(width as i32, height as i32));
        }

        self.buffer.fill([116, 116, 116, 255]);

        let folded_corners = if !self.folded { 0.0 } else { 12.0 };
        let is_selected = Some(self.id) == grid_ctx.selected_routine;
        let normal_color = CellRole::Event.to_color();
        let text_color = [85, 81, 85, 255];
        let header_height = 35;

        self.buffer.draw_rounded_rect(
            &TheDim::rect(0, 0, self.screen_width as i32, header_height as i32),
            if is_selected {
                &[187, 122, 208, 255]
            } else {
                &normal_color
            },
            &(folded_corners, 12.0, folded_corners, 12.0),
            0.0,
            &WHITE,
        );

        let stride = self.buffer.dim().width as usize;
        let desc = self.get_description();

        if let Some(font) = &ctx.ui.font {
            ctx.draw.text_rect_blend(
                self.buffer.pixels_mut(),
                &(20, 0, self.screen_width as usize, header_height as usize),
                stride,
                font,
                15.0,
                &format!("{} ({})", self.name, self.grid.count()),
                &text_color,
                TheHorizontalAlign::Left,
                TheVerticalAlign::Center,
            );
            ctx.draw.text_rect_blend(
                self.buffer.pixels_mut(),
                &(
                    0,
                    0,
                    self.screen_width as usize - 10,
                    header_height as usize,
                ),
                stride,
                font,
                13.0,
                &desc,
                &text_color,
                TheHorizontalAlign::Right,
                TheVerticalAlign::Center,
            );
        }

        if !self.folded {
            for (coord, cell) in &mut self.grid.grid {
                if let Some(rect) = self.grid.grid_rects.get(coord) {
                    let is_selected = Some(self.id) == grid_ctx.selected_routine
                        && Some(coord.clone()) == grid_ctx.current_cell;
                    cell.draw(
                        &mut self.buffer,
                        &rect,
                        ctx,
                        grid_ctx,
                        is_selected,
                        coord,
                        &self.name,
                        id,
                        debug,
                    );
                }
            }
        }
    }

    /// Sets the screen width.
    pub fn set_screen_width(&mut self, width: u32, ctx: &TheContext, grid_ctx: &GridCtx) {
        self.screen_width = width;
        self.draw(ctx, grid_ctx, 0, None);
    }

    /// Returns the number of lines in the grid.
    pub fn lines(&self) -> u32 {
        let mut lines = 1;
        for (c, _) in &self.grid.grid {
            if c.1 > lines {
                lines = c.1;
            }
        }
        lines
    }

    /// Handle a click at the given position.
    pub fn drop_at(
        &mut self,
        loc: Vec2<u32>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        grid_ctx: &mut GridCtx,
        drop: &TheDrop,
        module_type: ModuleType,
        palette: &ThePalette,
    ) -> bool {
        let mut handled = false;
        let mut pos: Option<(u32, u32)> = None;
        let mut old_item: CellItem = CellItem::new(Cell::Empty);
        let header_height = 35;

        if loc.y > header_height && !self.folded {
            for (coord, item) in self.grid.grid.iter_mut() {
                if let Some(rect) = self.grid.grid_rects.get(coord) {
                    if rect.contains(Vec2::new(loc.x as i32, loc.y as i32)) {
                        if item.replaceable {
                            grid_ctx.selected_routine = Some(self.id);
                            grid_ctx.current_cell = Some(coord.clone());
                            pos = Some(coord.clone());
                            old_item = item.clone();
                        }
                        handled = true;
                        break;
                    }
                }
            }
        }

        if let Some(pos) = pos {
            if let Some(cell) = Cell::from_str(&drop.title) {
                let mut item = CellItem::new(cell);
                let mut insert = true;

                // Only accept Cells to be dropped on cells with the same role
                if old_item.cell.role() != item.cell.role() && old_item.cell != Cell::Empty {
                    insert = false;
                }

                // But allow functions on values
                if old_item.cell.role() == CellRole::Value && item.cell.role() == CellRole::Function
                {
                    insert = true;
                }

                // Arithmetic ops can be dropped on empty positions if the left is value | fn
                if matches!(item.cell, Cell::Arithmetic(_)) && old_item.cell == Cell::Empty {
                    if self.grid.is_role_at(pos, -1, CellRole::Value)
                        || self.grid.is_role_at(pos, -1, CellRole::Function)
                    {
                        insert = true;
                    } else {
                        insert = false;
                    }
                }

                // Values / fns can be dropped on an empty cell if an arithmetic op is on the left.
                if (item.cell.role() == CellRole::Value || item.cell.role() == CellRole::Function)
                    && old_item.cell == Cell::Empty
                {
                    if self.grid.is_role_at(pos, -1, CellRole::Operator) {
                        insert = true;
                    } else {
                        insert = false;
                    }
                }

                // Insert a function
                if item.cell.role() == CellRole::Function && pos.0 == 0 {
                    insert = true;
                }

                if item.cell.role() == CellRole::Function && !old_item.description.is_empty() {
                    insert = false;
                }

                if insert {
                    if matches!(item.cell, Cell::Arithmetic(_)) {
                        let right_pos = (pos.0 + 1, pos.1);
                        // For arithmetic make sure we insert a value to the right
                        if !self.grid.grid.contains_key(&(pos.0 + 1, pos.1)) {
                            if module_type.is_shader() {
                                let value = CellItem::new(Cell::Value("1".to_string()));
                                value.insert_at(right_pos, &mut self.grid);
                            } else {
                                let value = CellItem::new(Cell::Integer("1".to_string()));
                                value.insert_at(right_pos, &mut self.grid);
                            }
                        }
                    }

                    if item.cell.role() == CellRole::Value {
                        item.description = old_item.description.clone();
                        item.replaceable = old_item.replaceable.clone();
                        item.dependend_on = old_item.dependend_on.clone();
                        item.form = old_item.form.clone();
                        item.special_role = old_item.special_role.clone();
                    }

                    self.grid.remove_dependencies_for(old_item.id);
                    item.insert_at(pos, &mut self.grid);
                }
            }
        }

        if let Some(pos) = pos {
            if let Some(item) = self.grid.grid.get(&pos) {
                let nodeui: TheNodeUI = item.create_settings(palette, module_type);
                if let Some(layout) = ui.get_text_layout("Node Settings") {
                    nodeui.apply_to_text_layout(layout);
                    ctx.ui.relayout = true;

                    ctx.ui.send(TheEvent::Custom(
                        TheId::named("Show Node Settings"),
                        TheValue::Text(format!("{} Settings", item.cell.description())),
                    ));
                }
            }

            self.grid.insert_empty();
            self.draw(ctx, grid_ctx, 0, None);
        }

        handled
    }

    /// Handle a click at the given position.
    pub fn click_at(
        &mut self,
        loc: Vec2<u32>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        grid_ctx: &mut GridCtx,
        module_type: ModuleType,
        palette: &ThePalette,
    ) -> bool {
        let mut handled = false;
        let header_height = 35;

        if loc.y < header_height {
            self.folded = !self.folded;
            grid_ctx.selected_routine = Some(self.id);
            grid_ctx.current_cell = None;
            self.draw(ctx, grid_ctx, 0, None);
            handled = true;

            ctx.ui.send(TheEvent::Custom(
                TheId::named("ModuleChanged"),
                TheValue::Empty,
            ));
        } else if !self.folded {
            for (coord, cell) in &self.grid.grid {
                if let Some(rect) = self.grid.grid_rects.get(coord) {
                    if rect.contains(Vec2::new(loc.x as i32, loc.y as i32)) {
                        grid_ctx.selected_routine = Some(self.id);
                        if grid_ctx.current_cell != Some(coord.clone()) {
                            grid_ctx.current_cell = Some(coord.clone());

                            let nodeui: TheNodeUI = cell.create_settings(palette, module_type);
                            if let Some(layout) = ui.get_text_layout("Node Settings") {
                                nodeui.apply_to_text_layout(layout);
                                ctx.ui.relayout = true;

                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Show Node Settings"),
                                    TheValue::Text(format!("{}", cell.cell.description())),
                                ));
                            }

                            self.draw(ctx, grid_ctx, 0, None);
                        }
                        handled = true;
                        break;
                    }
                }
            }
        }

        handled
    }

    /// Handle menu context at the given position.
    pub fn context_at(
        &mut self,
        loc: Vec2<u32>,
        _ctx: &TheContext,
        grid_ctx: &mut GridCtx,
    ) -> Option<TheContextMenu> {
        for (coord, item) in &self.grid.grid {
            if let Some(rect) = self.grid.grid_rects.get(coord) {
                if rect.contains(Vec2::new(loc.x as i32, loc.y as i32)) {
                    grid_ctx.selected_routine = Some(self.id);
                    grid_ctx.current_cell = Some(coord.clone());
                    return Some(item.generate_context());
                }
            }
        }

        None
    }

    /// Build the routine into shader source
    pub fn build_shader(&self, out: &mut String, indent: usize) {
        let mut indent = indent;

        *out += "fn shade() {\n";
        indent += 4;

        if self.scale != 1.0 {
            *out += &format!("    uv /= {};\n", self.scale);
        }

        if self.rotation != 0.0 {
            *out += &format!("    uv = rotate2d(uv, {});\n", self.rotation);
        }

        if self.pixelization > 0 {
            *out += &format!(
                "    uv = floor(uv * {}) / {};\n",
                self.pixelization, self.pixelization
            );
        }

        let reserved_vars = vec![
            "color",
            "roughness",
            "metallic",
            "uv",
            "normal",
            "bump",
            "opacity",
        ];

        let rows = self.grid.grid_by_rows();

        let mut var_lookup: FxHashSet<String> = FxHashSet::default();

        for row in rows {
            let mut row_code = String::new();

            let mut is_if = false;
            let mut is_else = false;
            let mut ind = indent;

            for (index, (item, pos)) in row.iter().enumerate() {
                let item_code = item.code();
                if index == 0 {
                    // Add a let for a variable definition
                    if matches!(item.cell, Cell::Variable(_))
                        && !reserved_vars.contains(&item_code.as_str())
                        && !var_lookup.contains(&item_code)
                    {
                        row_code += "let ";
                        var_lookup.insert(item.code());
                    }

                    if matches!(item.cell, Cell::If) {
                        is_if = true;
                    }
                    if matches!(item.cell, Cell::Else) {
                        is_else = true;
                    }

                    if let Some(i) = self.grid.row_indents.get(&pos.1) {
                        ind += *i as usize * 4;
                    }
                }

                row_code += &item_code;
                if item.cell.role() == CellRole::Function && item.form == CellItemForm::Rounded {
                    row_code += ")";
                }

                if !item.description.is_empty() {
                    // Check if we need to insert a "," or ")"
                    if let Some(next) = self.grid.grid.get(&(pos.0 + 1, pos.1)) {
                        if !next.description.is_empty() {
                            row_code += ", ";
                        } else {
                            row_code += ") ";
                        }
                    } else {
                        row_code += ") ";
                    }
                }

                if index == row.len() - 1 {
                    if is_if || is_else {
                        row_code += "{";
                    } else {
                        row_code += " ";
                    }
                } else {
                    row_code += " ";
                }
            }

            row_code += ";";
            *out += &format!("{:ind$}{}\n", "", row_code);
        }

        if self.color_steps > 0 {
            *out += &format!(
                "    color = floor(color * {}) / {};\n",
                self.color_steps, self.color_steps
            );
        }

        *out += "}\n";
    }

    /// Build the routine into Python source
    pub fn build_python(&self, out: &mut String, indent: usize, debug: bool) {
        let mut indent = indent;

        if self.name != "instantiation" {
            *out += &format!("{:indent$}if event == \"{}\":\n", "", self.name);
            indent += 4;
        }

        if self.name == "take_damage" {
            *out += &format!("{:indent$}amount = value[\"amount\"]\n", "");
            *out += &format!("{:indent$}from_id = value[\"from\"]\n", "");
        } else if self.name == "intent" {
            *out += &format!("{:indent$}intent = value[\"intent\"]\n", "");
            *out += &format!("{:indent$}distance = value[\"distance\"]\n", "");
            *out += &format!(
                "{:indent$}item_id = value[\"item_id\"] if \"item_id\" in value else -1\n",
                ""
            );
            *out += &format!(
                "{:indent$}entity_id = value[\"entity_id\"] if \"entity_id\" in value else -1\n",
                ""
            );
            *out += &format!(
                "{:indent$}target_id = value[\"target_id\"] if \"target_id\" in value else value.get(\"item_id\")\n",
                ""
            );
        } else if self.name == "key_down" || self.name == "key_up" {
            *out += &format!("{:indent$}key = value\n", "");
        }

        let rows = self.grid.grid_by_rows();

        // If empty just add a "pass" statement
        if rows.len() <= 1 {
            *out += &format!("{:indent$}pass\n", "");
        }

        for row in rows {
            let mut row_code = String::new();

            let mut is_if = false;
            let mut is_else = false;
            let mut ind = indent;

            if debug {
                for (_, (item, pos)) in row.iter().enumerate() {
                    // Add debug code
                    if item.cell.role() == CellRole::Function {
                        row_code +=
                            &format!("set_debug_loc(\"{}\", {}, {}); ", self.name, pos.0, pos.1);
                    }
                }
            }

            for (index, (item, pos)) in row.iter().enumerate() {
                if index == 0 {
                    if matches!(item.cell, Cell::If) {
                        is_if = true;
                    }
                    if matches!(item.cell, Cell::Else) {
                        is_else = true;
                    }

                    if let Some(i) = self.grid.row_indents.get(&pos.1) {
                        ind += *i as usize * 4;
                    }
                }

                row_code += &item.code();
                if item.cell.role() == CellRole::Function && item.form == CellItemForm::Rounded {
                    row_code += ")";
                }

                if !item.description.is_empty() {
                    // Check if we need to insert a "," or ")"
                    if let Some(next) = self.grid.grid.get(&(pos.0 + 1, pos.1)) {
                        if !next.description.is_empty() {
                            row_code += ", ";
                        } else {
                            row_code += ") ";
                        }
                    } else {
                        row_code += ") ";
                    }
                }

                if index == row.len() - 1 {
                    if is_if || is_else {
                        row_code += ":";
                    } else {
                        row_code += " ";
                    }
                } else {
                    row_code += " ";
                }
            }

            *out += &format!("{:ind$}{}\n", "", row_code);
        }
    }

    /// Get the description of the event
    fn get_description(&self) -> String {
        match self.name.as_str() {
            "startup" => "send on startup, 'value' contains the ID".into(),
            "instantiation" => "".into(),
            "proximity_warning" => "'value' is a list of entity IDs in proximity".into(),
            "closed_in" => "`value` is the entity ID".into(),
            "take_damage" => "`amount` is the damage and `from_id` is the ID".into(),
            "death" => "send on death".into(),
            "kill" => "`value` is the killed entity's ID".into(),
            "arrived" => "`value` is the sector name".into(),
            "intent" => {
                "`intent` is the command, 'target_id', 'entity_id', 'item_id' the IDs".into()
            }
            "bumped_by_entity" => "`value` is the entity ID".into(),
            "bumped_into_entity" => "`value` is the entity ID".into(),
            "bumped_into_item" => "`value` is the item ID".into(),
            "active" => "`value` is the active state of the item".into(),
            "goodbye" => "`value` is the entity ID".into(),
            "entered" => "`value` is the sector name".into(),
            "left" => "`value` is the sector name".into(),
            "key_down" => "'key' contains the pressed key string".into(),
            "key_up" => "'key' contains the released key string".into(),
            _ => "custom event".into(),
        }
    }
}
