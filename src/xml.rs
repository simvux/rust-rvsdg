use super::*;
use std::fmt;
use xmlwriter::{Options, XmlWriter};

pub struct XmlCtx<'ctx> {
    stack: Vec<StackEntry>,
    ctx: &'ctx TranslationUnitContext,
    xml: XmlWriter,
}

enum StackEntry {
    Unit(String),
    Node(id::AnyNode),
    Region(id::Region),
}

impl<'ctx> XmlCtx<'ctx> {
    pub fn write_node(&mut self, id: id::AnyNode) {
        let node = &self.ctx.nodes[id];

        self.xml.start_element("node");
        self.stack.push(StackEntry::Node(id));

        self.xml.write_attribute("id", &self.prefixed(""));
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
            self.write_region(*region);
        }

        self.stack.pop();
        self.xml.end_element();
    }

    pub fn write_region(&mut self, region: id::Region) {
        self.xml.start_element("region");
        self.stack.push(StackEntry::Region(region));

        // self.xml.write_attribute("id", &self.prefixed(""));

        for a in self.ctx.arguments(region) {
            self.xml.start_element("argument");
            self.xml.write_attribute("id", &self.prefixed(a));
            self.xml.end_element();
        }

        for r in self.ctx.results(region) {
            self.xml.start_element("result");
            self.xml.write_attribute("id", &self.prefixed(r));
            self.xml.end_element();
        }

        for node_id in self.ctx.nodes(region) {
            self.write_node(node_id);
        }

        for edge in &self.ctx.regions[region].edges {
            self.xml.start_element("edge");
            self.xml.write_attribute(
                "source",
                &match edge.origin {
                    Origin::Output(node, output) => {
                        self.stack.push(StackEntry::Node(node));
                        let str = self.prefixed(&output);
                        self.stack.pop().unwrap();
                        str
                    }
                    Origin::Argument(_, argument) => self.prefixed(&argument),
                },
            );
            self.xml.write_attribute(
                "target",
                &match edge.user {
                    User::Input(node, input) => {
                        self.stack.push(StackEntry::Node(node));
                        let str = self.prefixed(&input);
                        self.stack.pop().unwrap();
                        str
                    }
                    User::Result(_, result) => self.prefixed(&result),
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
                StackEntry::Node(id) => match self.ctx.symbols.get(*id) {
                    Some(sym) => buf.push_str(sym),
                    None => buf.push_str(&format!("n{}", id.as_u32())),
                },
                StackEntry::Region(id) => buf.push_str(&format!("r{}", id.as_u32())),
                StackEntry::Unit(name) => buf.push_str(&format!("{name}")),
            }
            buf.push('.');
        }
        buf.push_str(&v.to_string());
        buf
    }
}

pub fn new_xml() -> XmlWriter {
    let opt = Options::default();
    let mut xml = XmlWriter::new(opt);
    xml.start_element("rvsdg");
    xml
}

pub fn open_viewer(xml: String) {
    let mut path = std::env::temp_dir();
    path.push("rvsdg.xml");
    let mut f = std::fs::File::create(&path).unwrap();
    write!(f, "{}", xml).unwrap();
    println!(" wrote to {}", path.display());

    std::process::Command::new("rvsdg-viewer")
        .arg(path)
        .spawn()
        .unwrap();
}

impl TranslationUnitContext {
    pub fn add_to_xml(&self, name: String, xml: XmlWriter) -> XmlWriter {
        let mut ctx = XmlCtx {
            xml,
            stack: vec![StackEntry::Unit(name)],
            ctx: self,
        };
        ctx.write_node(id::AnyNode::from_u32(0));
        ctx.xml
    }

    pub fn to_xml(&self) -> String {
        let xml = new_xml();

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
