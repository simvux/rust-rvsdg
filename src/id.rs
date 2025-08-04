use std::fmt;
use std::marker::PhantomData;

use cranelift_entity::entity_impl;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnyNode(u32);
entity_impl!(AnyNode, "node");

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Region(u32);
entity_impl!(Region, "region");

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Argument(u32);
entity_impl!(Argument, "a");

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Result(u32);
entity_impl!(Result, "r");

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Input(u32);
entity_impl!(Input, "i");

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Output(u32);
entity_impl!(Output, "o");

#[derive(Debug)]
pub struct Node<K> {
    pub id: AnyNode,
    _kind: PhantomData<K>,
}

impl<K> Clone for Node<K> {
    fn clone(&self) -> Self {
        Self::new(self.id)
    }
}
impl<K> Copy for Node<K> {}

impl<K> Node<K> {
    pub(super) fn new(id: AnyNode) -> Self {
        Self {
            id,
            _kind: PhantomData,
        }
    }
}

impl From<AnyNode> for Node<AnyNode> {
    fn from(id: AnyNode) -> Self {
        Node::new(id)
    }
}

impl<K> fmt::Display for Node<K> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.id.fmt(f)
    }
}
