use super::*;

#[derive(Clone, Debug)]
pub struct Editor {
    pub objects: Vec<Object>,
    pub held: Option<Held>,
    /// Socket indicators to keep visible on a carried outlet. The native
    /// editor disconnects its devices on pickup, but the carried sprite keeps
    /// showing the plugs until it is placed.
    pub(super) held_outlet_plugs: Option<(usize, u16)>,
    pub(super) drag_constraints: Vec<DragConstraint>,
    pub resizing: Option<Resize>,
    /// First visible bin group (cycle_object_selection_forward/backward step by 5).
    pub bin_scroll: usize,
    pub connection_hints: bool,
    pub plug_preview: bool,
    /// The highlighted part (selected_object while no tool is active).
    pub hovered: Option<usize>,
    /// Freeform mode: the bin holds one template per part, and taking one
    /// copies it (pick_up_object_under_cursor's create_object_from_template).
    pub templates: Option<Vec<usize>>,
    /// Safe-triangle apex: the last cursor position near the highlighted part.
    pub(super) hover_anchor: Option<(i16, i16)>,
    /// Frames the cursor may rest in the safe triangle before it lapses.
    pub(super) hover_grace: u8,
    pub(super) last_cursor: Option<(i16, i16)>,
}

impl Editor {
    pub fn new(objects: Vec<Object>) -> Self {
        Self {
            objects,
            held: None,
            held_outlet_plugs: None,
            drag_constraints: Vec::new(),
            resizing: None,
            bin_scroll: 0,
            connection_hints: true,
            plug_preview: true,
            hovered: None,
            templates: None,
            hover_anchor: None,
            hover_grace: 0,
            last_cursor: None,
        }
    }

    /// Freeform editing: `objects` placed parts plus bin templates.
    pub fn freeform(mut objects: Vec<Object>, templates: Vec<Object>) -> Self {
        let first = objects.len();
        objects.extend(templates);
        let templates = (first..objects.len()).collect();
        Self {
            templates: Some(templates),
            ..Self::new(objects)
        }
    }

    /// The layout the machine starts from: placed parts with derived state.
    pub fn machine(&self) -> Result<Vec<Object>, String> {
        let mut objects = self.objects.clone();
        if let Some(held) = &self.held {
            objects[held.index].list = ObjectList::Inventory;
        }
        crate::machine::finalize_objects(&mut objects)?;
        Ok(objects)
    }
}
