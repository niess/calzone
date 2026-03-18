#![allow(unused)]

use std::collections::{HashMap, HashSet};
use std::hash::Hash;


// ===============================================================================================
//
// Graph traits.
//
// ===============================================================================================

pub trait DiGraph<N> {
    fn edges<'a>(
        &'a self,
        node: &'a N
    ) -> Output<impl IntoIterator<Item=&'a N>>;
}

pub enum Output<T> {
    Edges(T),
    Empty,
    Missing,
}

pub trait NodeAction<'a, N> {
    fn node_action(&mut self, node: &'a N) -> Result<(), GraphError<'a, N>>;
}

pub enum GraphError<'a, N> {
    Cycle(Vec<&'a N>),
    Missing(&'a N),
}


// ===============================================================================================
//
// Graph visitor.
//
// ===============================================================================================

pub struct Visitor<'a, G, N>
where
    G: DiGraph<N>,
{
    graph: &'a G,
    visited: HashSet<&'a N>,
}

struct Branch<'a, N> {
    current: HashSet<&'a N>,
    previous: HashMap<&'a N, &'a N>,
}

impl<'a, G, N> Visitor<'a, G, N>
where
    G: DiGraph<N>,
{
    pub fn new(graph: &'a G) -> Self {
        Self {
            graph,
            visited: HashSet::new(),
        }
    }

    pub fn visit<'b: 'a, A>(
        &mut self,
        node: &'b N,
        action: &mut A,
    ) -> Result<(), GraphError<'a, N>>
    where
        N: Eq + Hash,
        A: NodeAction<'a, N>,
    {
        let mut branch = Branch::<N> { current: HashSet::new(), previous: HashMap::new() };
        self.visit1(node, action, &mut branch)
    }

    fn visit1<'b: 'a, A>(
        &mut self,
        node: &'b N,
        action: &mut A,
        branch: &mut Branch<'a, N>
    ) -> Result<(), GraphError<'a, N>>
    where
        N: Eq + Hash,
        A: NodeAction<'a, N>,
    {
        branch.current.insert(node);
        self.visited.insert(node);

        match self.graph.edges(node) {
            Output::Edges(edges) => for w in edges {
                if !self.visited.contains(w) {
                    branch.previous.insert(w, node);
                    self.visit1(w, action, branch)?;
                } else if branch.current.contains(w) {
                    let mut nodes = vec![w, node];
                    loop {
                        match branch.previous.get(nodes.last().unwrap()) {
                            Some(previous) => if previous == &w {
                                break
                            } else {
                                nodes.push(previous);
                            },
                            None => break,
                        }
                    }
                    nodes.reverse();
                    return Err(GraphError::Cycle(nodes))
                }
            },
            Output::Empty => (),
            Output::Missing => return Err(GraphError::Missing(node)),
        }

        branch.current.remove(node);
        action.node_action(node)?;

        Ok(())
    }
}


// ===============================================================================================
//
// Graph actions.
//
// ===============================================================================================

pub struct Collector<'a, N> (pub Vec<&'a N>);

impl<'a, N> Collector<'a, N> {
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

impl<'a, N> NodeAction<'a, N> for Collector<'a, N> {
    #[inline]
    fn node_action(&mut self, node: &'a N) -> Result<(), GraphError<'a, N>> {
        self.0.push(node);
        Ok(())
    }
}

pub struct Nothing;

impl<'a, N> NodeAction<'a, N> for Nothing {
    #[inline]
    fn node_action(&mut self, _node: &'a N) -> Result<(), GraphError<'a, N>> {
        Ok(())
    }
}


// ===============================================================================================
//
// Basic graph implementation using a HashMap.
//
// ===============================================================================================

pub struct HashGraph<T: Eq + Hash>(pub HashMap<T, Vec<T>>);

impl<T> DiGraph<T> for HashGraph<T>
where
    T: Eq + Hash,
{
    fn edges<'a>(&'a self, node: &'a T) -> Output<impl IntoIterator<Item=&'a T>> {
        match self.0.get(node) {
            Some(edges) => if edges.is_empty() {
                Output::Empty
            } else {
                Output::Edges(edges)
            },
            None => Output::Missing,
        }
    }
}


// ===============================================================================================
//
// Unit tests.
//
// ===============================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cycle() {
        let graph = HashGraph::<usize>(HashMap::from([
            (0, vec![1]),
            (1, vec![0]),
        ]));
        let mut visitor = Visitor::new(&graph);
        if let GraphError::Cycle(nodes) = visitor.visit(&0, &mut Nothing).unwrap_err() {
            assert_eq!(nodes[0], &1);
            assert_eq!(nodes[1], &0);
        } else { assert!(false) };

        let graph = HashGraph::<usize>(HashMap::from([
            (0, vec![1, 2]),
            (1, vec![]),
            (2, vec![1, 3]),
            (3, vec![0]),
        ]));
        let mut visitor = Visitor::new(&graph);
        if let GraphError::Cycle(nodes) = visitor.visit(&0, &mut Nothing).unwrap_err() {
            assert_eq!(nodes[0], &2);
            assert_eq!(nodes[1], &3);
            assert_eq!(nodes[2], &0);
        } else { assert!(false) };
    }

    #[test]
    fn test_missing() {
        let graph = HashGraph::<usize>(HashMap::from([
            (0, vec![1]),
        ]));
        let mut visitor = Visitor::new(&graph);
        if let GraphError::Missing(node) = visitor.visit(&0, &mut Nothing).unwrap_err() {
            assert_eq!(node, &1);
        } else { assert!(false) };
    }

    #[test]
    fn test_valid() {
        let graph = HashGraph::<usize>(HashMap::from([
            (0, vec![1, 2]),
            (1, vec![]),
            (2, vec![1, 3]),
            (3, vec![]),
        ]));
        let mut collector = Collector::new();
        let mut visitor = Visitor::new(&graph);
        assert!(visitor.visit(&0, &mut collector).is_ok());
        assert_eq!(
            collector.0,
            vec![&1, &3, &2, &0],
        )
    }
}
