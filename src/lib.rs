use cranelift_entity::{EntityList, ListPool, PrimaryMap, SecondaryMap};
use std::collections::HashMap;
use std::io::Write;
use tracing::{info, trace};

mod edge;
pub use edge::{Argument, Edge, Input, Origin, Output, Result, User};
pub mod id;
pub mod nodes;
pub use nodes::NodeKind;
use nodes::*;
#[cfg(test)]
mod tests;
mod xml;
pub use xml::{new_xml, open_viewer};

#[macro_export]
macro_rules! node_kind_impl {
    ($ty:ty, $kind:literal) => {
        impl NodeKind for $ty {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }

            fn node_type(&self) -> &str {
                $kind
            }
        }
    };
}

/// The context for a whole translation unit
#[derive(Debug)]
pub struct TranslationUnitContext {
    nodes: PrimaryMap<id::AnyNode, Node>,
    regions: PrimaryMap<id::Region, Region>,

    symbols: SecondaryMap<id::AnyNode, String>,

    node_id_pool: ListPool<id::AnyNode>,
    region_id_pool: ListPool<id::Region>,

    pub region: id::Region,
}

#[derive(Debug)]
pub struct Node {
    // Self-referential node id
    id: id::AnyNode,
    region: id::Region,

    inputs: u32,
    outputs: u32,

    regions: EntityList<id::Region>,

    kind: Box<dyn NodeKind + Send + Sync>,
}

#[derive(Debug)]
struct Region {
    container_node: Option<id::AnyNode>,
    arguments: u32,
    results: u32,

    edges: Vec<Edge>,

    nodes: EntityList<id::AnyNode>,
}

impl TranslationUnitContext {
    pub fn new() -> Self {
        let mut omega = TranslationUnitContext {
            nodes: PrimaryMap::new(),
            regions: PrimaryMap::new(),
            symbols: SecondaryMap::new(),
            node_id_pool: ListPool::new(),
            region_id_pool: ListPool::new(),
            region: id::Region::from_u32(0),
        };

        omega.add_region(0, 0);

        omega
    }

    pub fn get<K: NodeKind>(&self, id: id::Node<K>) -> &K {
        get_kind(&self.nodes, id)
    }

    pub fn get_mut<K: NodeKind>(&mut self, id: id::Node<K>) -> &mut K {
        get_kind_mut(&mut self.nodes, id)
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

        trace!("adding {node_id} in {}", self.region);

        let mut node = Node {
            kind: Box::new(kind),
            region: self.region,
            inputs: 0,
            outputs: 0,
            regions: EntityList::new(),
            id: any_node_id,
        };

        for region in regions {
            self.regions[region].container_node = Some(any_node_id);
        }
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

    pub fn add_symbol(&mut self, node: id::AnyNode, sym: impl Into<String>) {
        self.symbols[node] = sym.into();
    }

    fn add_region(&mut self, arguments: u32, results: u32) -> id::Region {
        self.regions.push(Region {
            container_node: None,
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

    fn debug_node(&self, node: id::AnyNode) -> String {
        let sym = &self.symbols[node];
        if sym == "" {
            format!("{node}")
        } else {
            format!("{node}·{sym}")
        }
    }

    pub fn add_input<K>(&mut self, node: id::Node<K>) -> Input<K> {
        let inputs = &mut self.nodes[node.id].inputs;
        let input = id::Input::from_u32(*inputs);
        *inputs += 1;

        trace!("added input {input} for {}", self.debug_node(node.id));

        // Forward this input as an argument to each contained region.
        for region in self.nodes[node.id].regions.as_slice(&self.region_id_pool) {
            let arguments = &mut self.regions[*region].arguments;
            *arguments += 1;
            assert!(
                dbg!(*arguments) >= dbg!(input.as_u32() + 1),
                "region has fewer arguments than node has inputs"
            );
        }

        Input { id: input, node }
    }

    pub fn input_as_argument<K>(&self, input: Input<K>) -> Argument {
        let region = self.region(input.node.id);
        let args = self.regions[region].arguments;
        let inputs = self.nodes[input.node.id].inputs;

        let node_custom_args = args - inputs;

        let forwarded_arg = node_custom_args + input.id.as_u32();
        let id = id::Argument::from_u32(forwarded_arg);
        Argument { id, region }
    }

    pub fn argument_as_input(
        &self,
        region: id::Region,
        argument: id::Argument,
    ) -> Option<Input<id::AnyNode>> {
        let args = self.regions[region].arguments;
        let node = self.regions[region].container_node.unwrap();
        let inputs = self.nodes[node].inputs;

        let node_custom_args = args - inputs;

        // λ x y 2 3 4
        // check 2 >= 2
        // yield 2 - 2
        if argument.as_u32() >= node_custom_args {
            let id = id::Input::from_u32(argument.as_u32() - node_custom_args);
            Some(Input {
                node: id::Node::new(node),
                id,
            })
        } else {
            None
        }
    }

    pub fn add_output<K>(&mut self, node: id::Node<K>) -> Output<K> {
        let outputs = &mut self.nodes[node.id].outputs;
        let output = id::Output::from_u32(*outputs);
        *outputs += 1;
        Output { id: output, node }
    }

    pub fn add_argument(&mut self) -> Argument {
        let arguments = &mut self.regions[self.region].arguments;
        let arg = id::Argument::from_u32(*arguments);
        *arguments += 1;

        trace!("added argument {arg} for {}", self.region);

        Argument {
            id: arg,
            region: self.region,
        }
    }

    pub fn add_result(&mut self) -> Result {
        let results = &mut self.regions[self.region].results;
        let result = id::Result::from_u32(*results);
        *results += 1;

        trace!("added result {result} for {}", self.region);

        Result {
            id: result,
            region: self.region,
        }
    }

    pub fn connect(&mut self, origin: impl Into<Origin>, user: impl Into<User>) {
        let origin = origin.into();
        let user = user.into();
        let ok = self.try_connect(origin, user);
        if !ok {
            panic!("no available path to connect {origin:?} → {user:?}");
        }
    }

    /// Try to find a path to make the connection. Returns false if unable.
    pub fn try_connect(&mut self, origin: impl Into<Origin>, user: impl Into<User>) -> bool {
        const CONNECTED: bool = true;

        // TODO: We could resolve recenv lambda nodes here and edge-case them to 'fix' connections?
        // That way we could effectively 'hide' recenv's from user API.

        let origin = origin.into();
        let user = user.into();

        trace!("trying to connect {origin:?} -> {user:?}");

        if self.connection_exists(origin, user) {
            return CONNECTED;
        }

        // TODO: we need to be able to detect cycles and reformat things into recenvs.

        match origin {
            Origin::Output(node_id, output) => {
                let Some(origin) = self.find_and_connect_output(node_id, output) else {
                    return !CONNECTED;
                };

                self.raw_connect_asserted(origin, user);
                return CONNECTED;
            }
            Origin::Argument(region, arg) => {
                let arg = Argument { region, id: arg };
                let Some(origin) = self.find_and_connect_argument(arg) else {
                    return !CONNECTED;
                };

                self.raw_connect_asserted(origin, user);
                return CONNECTED;
            }
        }
    }

    // We can just traverse regions upwards until we find the one that has the node we need

    fn find_and_connect_output(
        &mut self,
        output_node: id::AnyNode,
        output: id::Output,
    ) -> Option<Origin> {
        let region = &self.regions[self.region];

        if region
            .nodes
            .as_slice(&self.node_id_pool)
            .contains(&output_node)
        {
            Some(Origin::Output(output_node, output))
        } else {
            let Some(in_node) = region.container_node else {
                // We've reached omega. There's no path to node
                return None;
            };

            let parent_region = self.nodes[in_node].region;

            // The output we're trying to connect is not from a node in this region.
            // So; try the parent region. If it succeeded then forward that new connection into the
            // current region.
            self.in_region(parent_region, |this| {
                this.find_and_connect_output(output_node, output)
                    .map(|origin| {
                        let input = this.add_input::<id::AnyNode>(id::Node::new(in_node));
                        let arg = this.input_as_argument(input);
                        this.raw_connect_asserted(origin, input);
                        Origin::Argument(arg.region, arg.id)
                    })
            })
        }
    }

    fn find_and_connect_argument(&mut self, arg: Argument) -> Option<Origin> {
        trace!("attempting to find path from {arg:?} to {}", self.region);

        if self.region == arg.region {
            Some(Origin::from(arg))
        } else {
            let Some(parent_node) = self.regions[self.region].container_node else {
                trace!("can not connect from {}", self.region);
                return None;
            };

            trace!("not in current region, checking parent {parent_node:?}");

            let parent_region = self.nodes[parent_node].region;
            self.in_region(parent_region, |this| {
                this.find_and_connect_argument(arg).map(|origin| {
                    dbg!(&origin);
                    let input = this.add_input::<id::AnyNode>(id::Node::new(parent_node));
                    let arg = this.input_as_argument(input);
                    this.raw_connect_asserted(origin, input);
                    Origin::Argument(arg.region, arg.id)
                })
            })
        }
    }

    // Raw-connect an origin to a node and return the created argument for the contained region
    fn forward_origin_as_argument(&mut self, origin: Origin) -> Argument {
        todo!();
    }

    /// Connect without any implicit automatic connections but still assert against incorrect connections
    pub fn raw_connect_asserted(&mut self, origin: impl Into<Origin>, user: impl Into<User>) {
        let origin = origin.into();
        let user = user.into();

        info!("connecting {origin:?} → {user:?}");

        if let Origin::Output(node_id, _) = origin {
            assert!(
                self.current_nodes().contains(&node_id),
                "origin {origin:?} is from node not in current {}",
                self.region
            );
        }

        if let User::Input(node_id, _) = user {
            assert!(
                self.current_nodes().contains(&node_id),
                "user {user:?} is for node not in current {}",
                self.region
            );
        }

        unsafe { self.raw_connect(origin, user) }
    }

    /// NOTE: While this function is memory safe, it's marked as unsafe since it allows you to
    /// break the rules of what defines an RVSDG and can break any assumptions made by code analysis.
    unsafe fn raw_connect(&mut self, origin: impl Into<Origin>, user: impl Into<User>) {
        let origin = origin.into();
        let user = user.into();
        self.regions[self.region].edges.push(Edge { origin, user });
    }

    fn current_nodes(&self) -> &[id::AnyNode] {
        self.regions[self.region].nodes.as_slice(&self.node_id_pool)
    }

    fn connection_exists(&self, origin: Origin, user: User) -> bool {
        self.regions[self.region]
            .edges
            .iter()
            .find(|edge| (edge.user == user) && self.edge_leads_to_origin(&edge, origin))
            .is_some()
    }

    fn edge_leads_to_origin(&self, edge: &Edge, origin: Origin) -> bool {
        edge.origin == origin
            || match edge.origin {
                Origin::Output(..) => false,
                Origin::Argument(_, argument) => {
                    let Some(input) = self.argument_as_input(self.region, argument) else {
                        return false;
                    };

                    self.connection_exists(origin, input.into())
                }
            }
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

    pub fn open_rvsdg_viewer(&mut self) {
        let xml = self.to_xml();
        xml::open_viewer(xml)
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
