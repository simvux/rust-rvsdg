use crate::NodeKind;

impl NodeKind for Number {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn node_type(&self) -> &str {
        "simple"
    }
}

#[derive(Debug)]
pub struct Number(pub i128);
