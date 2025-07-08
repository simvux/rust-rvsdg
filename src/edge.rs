use super::id;

#[derive(Debug)]
pub struct Edge {
    pub origin: Origin,
    pub user: User,
}

#[derive(Clone, PartialEq, Eq, Debug, Copy)]
pub enum User {
    Input(id::AnyNode, id::Input),
    Result(id::Region, id::Result),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Origin {
    Output(id::AnyNode, id::Output),
    Argument(id::Region, id::Argument),
}

#[derive(Debug)]
pub struct Input<K> {
    pub node: id::Node<K>,
    pub id: id::Input,
}

#[derive(Debug)]
pub struct Output<K> {
    pub node: id::Node<K>,
    pub id: id::Output,
}

#[derive(Debug)]
pub struct Argument {
    pub region: id::Region,
    pub id: id::Argument,
}

#[derive(Debug)]
pub struct Result {
    pub region: id::Region,
    pub id: id::Result,
}

impl<K> Output<K> {
    pub fn downgrade(self) -> Output<id::AnyNode> {
        Output {
            node: id::Node::new(self.node.id),
            id: self.id,
        }
    }
}

impl<K> Input<K> {
    pub fn downgrade(self) -> Input<id::AnyNode> {
        Input {
            node: id::Node::new(self.node.id),
            id: self.id,
        }
    }
}

impl<K> Clone for Input<K> {
    fn clone(&self) -> Self {
        Input {
            node: self.node,
            id: self.id,
        }
    }
}
impl<K> Clone for Output<K> {
    fn clone(&self) -> Self {
        Output {
            node: self.node,
            id: self.id,
        }
    }
}

impl<K> Copy for Input<K> {}
impl<K> Copy for Output<K> {}

impl<K> From<Input<K>> for User {
    fn from(input: Input<K>) -> Self {
        User::Input(input.node.id, input.id)
    }
}

impl From<Result> for User {
    fn from(result: Result) -> Self {
        User::Result(result.region, result.id)
    }
}

impl<K> From<Output<K>> for Origin {
    fn from(output: Output<K>) -> Self {
        Origin::Output(output.node.id, output.id)
    }
}

impl From<Argument> for Origin {
    fn from(argument: Argument) -> Self {
        Origin::Argument(argument.region, argument.id)
    }
}
