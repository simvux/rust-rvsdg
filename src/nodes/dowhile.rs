use crate::NodeKind;

impl NodeKind for DoWhile {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn node_type(&self) -> &str {
        "theta"
    }
}

#[derive(Debug)]
pub struct DoWhile {}
