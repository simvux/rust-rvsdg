use crate::NodeKind;

impl NodeKind for GlobalV {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn node_type(&self) -> &str {
        "delta"
    }
}

#[derive(Debug)]
pub struct GlobalV {}
