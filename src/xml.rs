use super::*;
use std::fmt;
use xmlwriter::{Options, XmlWriter};

pub struct XmlCtx<'ctx> {
    stack: Vec<StackEntry<'ctx>>,
    ctx: &'ctx TranslationUnitContext,
    xml: XmlWriter,
}

enum StackEntry<'ctx> {
    Node(id::AnyNode),
    Region(&'ctx str),
}

impl<'ctx> XmlCtx<'ctx> {
    pub fn write_node(&mut self, id: id::AnyNode) {
        let node = &self.ctx.nodes[id];

        self.xml.start_element("node");
        self.stack.push(StackEntry::Node(id));

        self.xml.write_attribute("id", &id);
        if let Some(name) = self.ctx.symbols.get(id) {
            self.xml.write_attribute("name", &name);
        }
        self.xml.write_attribute("type", node.kind.node_type());

        for i in self.ctx.inputs(id) {
            self.xml.start_element("input");
            self.xml.write_attribute("id", &self.prefixed(i));
            self.xml.end_element();
        }

        for o in self.ctx.outputs(id) {
            self.xml.start_element("output");
            self.xml.write_attribute("id", &self.prefixed(o));
            // self.xml.write_attribute("id", &format_args!("{id}.{o}"));
            self.xml.end_element();
        }

        for region in node.regions.as_slice(&self.ctx.region_id_pool) {
            self.write_region("", region);
        }

        self.stack.pop();
        self.xml.end_element();
    }

    pub fn write_region(&mut self, id: &'ctx str, region: &'ctx id::Region) {
        self.xml.start_element("region");
        self.xml.write_attribute("id", &id);
        self.stack.push(StackEntry::Region(id));

        for a in self.ctx.arguments(*region) {
            self.xml.start_element("argument");
            self.xml.write_attribute("id", &self.prefixed(a));
            // self.xml.write_attribute("name", &m.name);
            self.xml.end_element();
        }

        for r in self.ctx.results(*region) {
            self.xml.start_element("result");
            self.xml.write_attribute("id", &self.prefixed(r));
            // self.xml.write_attribute("name", &m.name);
            self.xml.end_element();
        }

        for node_id in self.ctx.nodes(*region) {
            self.write_node(node_id);
        }

        for edge in &self.ctx.regions[*region].edges {
            self.xml.start_element("edge");
            self.xml.write_attribute(
                "source",
                &match edge.origin {
                    Origin::Output(node, output) => self.prefixed(format!("{node}.{output}")),
                    Origin::Argument(argument) => self.prefixed(argument),
                },
            );
            self.xml.write_attribute(
                "target",
                &match edge.user {
                    User::Input(node, input) => self.prefixed(format!("{node}.{input}")),
                    User::Result(result) => self.prefixed(result),
                },
            );
            self.xml.end_element();
        }

        self.stack.pop();
        self.xml.end_element();
    }

    fn prefixed(&self, v: impl fmt::Display) -> String {
        let mut buf = String::new();
        for entry in &self.stack {
            match entry {
                StackEntry::Node(node_id) => buf.push_str(&node_id.to_string()),
                StackEntry::Region(name) => buf.push_str(&format!("region[{name}]")),
            }
            buf.push('.');
        }
        // if buf.ends_with('.') {
        //     buf.pop();
        // }
        buf.push_str(&v.to_string());
        buf
    }
}

impl TranslationUnitContext {
    pub fn to_xml(&self) -> String {
        let opt = Options::default();
        let mut xml = XmlWriter::new(opt);

        xml.start_element("rvsdg");

        let mut ctx = XmlCtx {
            xml,
            stack: vec![],
            ctx: self,
        };
        ctx.write_node(id::AnyNode::from_u32(0));

        ctx.xml.end_element();

        ctx.xml.end_document()
    }
}
