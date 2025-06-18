use cranelift_entity::{EntityList, ListPool, PrimaryMap, SecondaryMap};
use std::{any::Any, collections::HashMap};

mod edge;
pub use edge::{Edge, Input, Origin, Output, User};
pub mod id;
mod nodes;
use nodes::*;
#[cfg(test)]
mod tests;
mod xml;

pub trait NodeKind: std::any::Any + std::fmt::Debug {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn node_type(&self) -> &str;
}

/// The context for a whole translation unit
pub struct TranslationUnitContext {
    nodes: PrimaryMap<id::AnyNode, Node>,
    regions: PrimaryMap<id::Region, Region>,

    symbols: SecondaryMap<id::AnyNode, String>,

    node_id_pool: ListPool<id::AnyNode>,
    region_id_pool: ListPool<id::Region>,

    region: id::Region,
}

struct Node {
    // Self-referential node id
    id: id::AnyNode,

    inputs: u32,
    outputs: u32,

    regions: EntityList<id::Region>,

    kind: Box<dyn NodeKind>,
}

struct Region {
    arguments: u32,
    results: u32,

    edges: Vec<Edge>,

    nodes: EntityList<id::AnyNode>,
}

impl TranslationUnitContext {
    pub fn new() -> (Self, id::Node<TranslationUnit>) {
        let mut omega = TranslationUnitContext {
            nodes: PrimaryMap::new(),
            regions: PrimaryMap::new(),
            symbols: SecondaryMap::new(),
            node_id_pool: ListPool::new(),
            region_id_pool: ListPool::new(),
            region: id::Region::from_u32(0),
        };

        let tunit = omega.add_node(|omega, _| {
            let region = omega.add_region(0, 0);
            (TranslationUnit { region }, [region])
        });

        (omega, tunit)
    }

    pub fn in_region<T>(&mut self, region: id::Region, f: impl FnOnce(&mut Self) -> T) -> T {
        let previous = self.region;
        self.switch_region(region);
        let v = f(self);
        self.region = previous;
        v
    }

    pub fn switch_region(&mut self, region: id::Region) {
        self.region = region;
    }

    pub fn inputs(&self, node: id::AnyNode) -> impl Iterator<Item = id::Input> + 'static {
        (0..self.nodes[node].inputs)
            .into_iter()
            .map(|i| id::Input::from_u32(i))
    }
    pub fn outputs(&self, node: id::AnyNode) -> impl Iterator<Item = id::Output> + 'static {
        (0..self.nodes[node].outputs)
            .into_iter()
            .map(|i| id::Output::from_u32(i))
    }
    pub fn arguments(&self, region: id::Region) -> impl Iterator<Item = id::Argument> + 'static {
        (0..self.regions[region].arguments)
            .into_iter()
            .map(|i| id::Argument::from_u32(i))
    }
    pub fn results(&self, region: id::Region) -> impl Iterator<Item = id::Result> + 'static {
        (0..self.regions[region].results)
            .into_iter()
            .map(|i| id::Result::from_u32(i))
    }
    pub fn nodes(&self, region: id::Region) -> impl Iterator<Item = id::AnyNode> {
        self.regions[region]
            .nodes
            .as_slice(&self.node_id_pool)
            .iter()
            .copied()
    }

    /// Create a new empty node of any kind and manually initialize it with `init`
    pub fn add_node<const N: usize, F, K: NodeKind>(&mut self, init: F) -> id::Node<K>
    where
        F: FnOnce(&mut Self, id::Node<K>) -> (K, [id::Region; N]),
    {
        let any_node_id = self.nodes.next_key();
        let node_id = id::Node::<K>::new(any_node_id);

        let (kind, regions) = init(self, node_id);

        let mut node = Node {
            kind: Box::new(kind),
            inputs: 0,
            outputs: 0,
            regions: EntityList::new(),
            id: any_node_id,
        };

        node.regions.extend(regions, &mut self.region_id_pool);

        assert_eq!(
            self.nodes.push(node),
            any_node_id,
            "A node initializer is not allowed to create additional nodes"
        );

        self.regions[self.region]
            .nodes
            .push(node_id.id, &mut self.node_id_pool);

        node_id
    }

    pub fn add_symbol(&mut self, node: id::AnyNode, sym: String) {
        self.symbols[node] = sym;
    }

    fn add_region(&mut self, arguments: u32, results: u32) -> id::Region {
        self.regions.push(Region {
            arguments,
            results,
            edges: vec![],
            nodes: EntityList::new(),
        })
    }

    // TODO: we should assert against region belonging to an recenv?
    // pub fn add_node_to_region(&mut self, region: id::Region, node: id::AnyNode) {
    //     self.regions[region]
    //         .nodes
    //         .push(node, &mut self.node_id_pool);
    // }

    /// Get the only singular region. Panics if there's not exactly one region
    pub fn region(&self, node: id::AnyNode) -> id::Region {
        match self.nodes[node].regions.as_slice(&self.region_id_pool) {
            [only] => *only,
            regions => panic!(
                "`region` can not be called for node with {} regions",
                regions.len()
            ),
        }
    }

    pub fn regions(&self, node: id::AnyNode) -> &[id::Region] {
        self.nodes[node].regions.as_slice(&self.region_id_pool)
    }

    pub fn lambda_output(&self, _node: id::Node<Lambda>) -> id::Output {
        id::Output::from_u32(0)
    }

    /// Create a lambda node.
    ///
    /// Lambda nodes have a singular region.
    /// Lambda nodes have a singular output, representing itself.
    pub fn add_lambda_node(&mut self) -> Output<Lambda> {
        let node_id = self.add_node(|ctx, _| {
            let region = ctx.add_region(0, 0);
            (Lambda {}, [region])
        });

        self.add_output(node_id)
    }

    // Create a globalv (delta) node.
    //
    // GlobalV nodes have a singular region representing the initialization of a value.
    // GlobalV nodes regions have singular results, representing the initialized values.
    // GlobalV nodes have a singualr output, representing the initialized value.
    pub fn add_globalv_node(&mut self) -> (id::Result, Output<GlobalV>) {
        let node_id = self.add_node(|ctx, _| {
            let initializer = ctx.add_region(0, 1);
            (GlobalV {}, [initializer])
        });

        let output = self.add_output(node_id);

        (id::Result::from_u32(0), output)
    }

    // Create a RecEnv (phi) node.
    //
    // RecEnv nodes have a singular region, containing lambdas that can be mutually recursive.
    // RecEnv nodes have an output for each contained lambda.
    pub fn add_recenv_node(&mut self) -> id::Node<RecEnv> {
        let node_id = self.add_node(|ctx, _| {
            let region = ctx.add_region(0, 0);
            let lambdas = HashMap::new();
            (RecEnv { lambdas }, [region])
        });

        node_id
    }

    // Create a number node.
    //
    // Number nodes have no regions and have one output representing the numeric value.
    pub fn add_number_node(&mut self, n: i128) -> Output<Number> {
        let node_id = self.add_node(|_, _| (Number(n), []));
        self.add_output(node_id)
    }

    // Create an apply node.
    //
    // Apply nodes take a lambda as first input. The rest of the inputs will be mapped to the
    // argument for the lambda's region.
    pub fn add_apply_node(&mut self) -> Input<Apply> {
        let node_id = self.add_node(|_, _| (Apply {}, []));
        self.add_input(node_id)
    }

    // Create a placeholder node.
    //
    // Placeholder nodes have no regions and start with one output.
    //
    // They're meant to act as a "todo" node.
    pub fn add_placeholder_node(&mut self, name: &'static str) -> Output<Placeholder> {
        let node_id = self.add_node(|_, _| (Placeholder(name), []));
        self.add_output(node_id)
    }

    pub fn add_input<K: NodeKind>(&mut self, node: id::Node<K>) -> Input<K> {
        let inputs = &mut self.nodes[node.id].inputs;
        let input = id::Input::from_u32(*inputs);
        *inputs += 1;

        // Forward this input as an argument to each contained region.
        for region in self.nodes[node.id].regions.as_slice(&self.region_id_pool) {
            self.regions[*region].arguments += 1;
        }

        Input { id: input, node }
    }

    pub fn input_as_argument<K>(&self, input: Input<K>) -> id::Argument {
        let region = self.region(input.node.id);
        let args = self.regions[region].arguments;
        let inputs = self.nodes[input.node.id].inputs;

        let node_custom_args = args - inputs;

        let forwarded_arg = node_custom_args + input.id.as_u32();
        id::Argument::from_u32(forwarded_arg)
    }

    pub fn add_output<K>(&mut self, node: id::Node<K>) -> Output<K> {
        let outputs = &mut self.nodes[node.id].inputs;
        let output = id::Output::from_u32(*outputs);
        *outputs += 1;
        Output { id: output, node }
    }

    pub fn add_argument(&mut self) -> id::Argument {
        let arguments = &mut self.regions[self.region].arguments;
        let arg = id::Argument::from_u32(*arguments);
        *arguments += 1;
        arg
    }

    pub fn add_result(&mut self) -> id::Result {
        let results = &mut self.regions[self.region].results;
        let result = id::Result::from_u32(*results);
        *results += 1;
        result
    }

    pub fn connect(&mut self, origin: impl Into<Origin>, user: impl Into<User>) {
        let node_list = self.regions[self.region].nodes.as_slice(&self.node_id_pool);

        // TODO: We could resolve recenv lambda nodes here and edge-case them to 'fix' connections?
        // That way we could effectively 'hide' recenv's from user API.
        let origin = origin.into();
        let user = user.into();

        if let User::Input(node_id, _) = user {
            assert!(
                node_list.contains(&node_id),
                "connection user is an input to a node in a different region"
            );
        }

        if let Origin::Output(node_id, _) = origin {
            assert!(
                node_list.contains(&node_id),
                "connection origin is an output of a node from a different region"
            );
        }

        self.regions[self.region].edges.push(Edge { origin, user });
    }

    pub fn move_node(&mut self, node: id::AnyNode, to: id::Region) {
        let rnodes = &mut self.regions[self.region].nodes;
        let i = rnodes
            .as_slice(&self.node_id_pool)
            .iter()
            .position(|n| *n == node)
            .expect("node is not in current region");
        rnodes.remove(i, &mut self.node_id_pool);
        self.regions[to].nodes.push(node, &mut self.node_id_pool);
    }
}

fn get_kind<K: NodeKind>(nodes: &PrimaryMap<id::AnyNode, Node>, id: id::Node<K>) -> &K {
    nodes[id.id]
        .kind
        .as_any()
        .downcast_ref()
        .expect("node is not of expected kind")
}

fn get_kind_mut<K: NodeKind>(nodes: &mut PrimaryMap<id::AnyNode, Node>, id: id::Node<K>) -> &mut K {
    nodes[id.id]
        .kind
        .as_any_mut()
        .downcast_mut()
        .expect("node is not of expected kind")
}

impl Node {
    // fn new(name: String) -> Node {
    //     Node {}
    // }
}
