#![allow(dead_code)]
use std::marker::PhantomData;

struct NodeId<Node>(usize, PhantomData<Node>);
impl<T> Clone for NodeId<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for NodeId<T> {}

trait ToRef: Sized {
    type Ref<'a>
    where
        Self: 'a;

    fn to_ref<'a>(&self, graph: &'a Graph<Self>) -> Self::Ref<'a>;
}

struct Graph<Node> {
    nodes: Vec<Node>,
}

struct Ref<'a, Node> {
    graph: &'a Graph<Node>,
    id: NodeId<Node>,
}

impl<'a, Node> Ref<'a, Node> {
    fn get(&self) -> &'a Node {
        self.graph.get(self.id)
    }

    fn get_ref(&self) -> Node::Ref<'a>
    where
        Node: ToRef,
    {
        self.get().to_ref(self.graph)
    }
}

impl<Node> Graph<Node> {
    fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    fn add(&mut self, node: Node) -> NodeId<Node> {
        let id = NodeId(self.nodes.len(), PhantomData);
        self.nodes.push(node);
        id
    }

    fn get(&self, id: NodeId<Node>) -> &Node {
        &self.nodes[id.0]
    }

    fn get_ref(&self, id: NodeId<Node>) -> Ref<'_, Node>
    where
        Node: ToRef,
    {
        Ref { graph: self, id }
    }

    fn get_mut(&mut self, id: NodeId<Node>) -> &mut Node {
        &mut self.nodes[id.0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    enum TestNode {
        Leaf,
        Branch(Vec<NodeId<Self>>),
    }

    impl ToRef for TestNode {
        type Ref<'a> = TestNodeRef<'a>;

        fn to_ref<'a>(&self, graph: &'a Graph<Self>) -> Self::Ref<'a> {
            match self {
                TestNode::Leaf => TestNodeRef::Leaf,
                TestNode::Branch(children) => {
                    TestNodeRef::Branch(children.iter().map(|id| Ref { id: *id, graph }).collect())
                }
            }
        }
    }

    enum TestNodeRef<'a> {
        Leaf,
        Branch(Vec<Ref<'a, TestNode>>),
    }

    fn process_ref(node: TestNodeRef<'_>) {
        match node {
            TestNodeRef::Leaf => todo!(),
            TestNodeRef::Branch(items) => {
                for item in items {
                    process_ref(item.get_ref());
                }
            }
        }
    }

    #[test]
    fn test_name() {
        let mut graph = Graph::new();
        let leaf_id = graph.add(TestNode::Leaf);
        let branch_id = graph.add(TestNode::Branch(vec![leaf_id]));

        let branch = graph.get_ref(branch_id).get_ref();
        process_ref(branch);
    }
}
