use crate::{NodeKind, id, xml::XmlCtx};

impl NodeKind for TranslationUnit {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn node_type(&self) -> &str {
        "omega"
    }
}

#[derive(Debug)]
pub struct TranslationUnit {
    pub region: id::Region,
}

impl TranslationUnit {}
