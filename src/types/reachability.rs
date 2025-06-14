#![allow(unused_variables)]
use std::collections::HashSet;

use super::core::ID;

#[derive(Default, Debug)]
pub struct Reachability {
    upsets: Vec<OrderedSet<ID>>,
    downsets: Vec<OrderedSet<ID>>,
}

impl Reachability {
    pub fn add_node(&mut self) -> ID {
        let i = self.upsets.len();
        self.upsets.push(Default::default());
        self.downsets.push(Default::default());
        i
    }
    pub fn add_edge(&mut self, lhs: ID, rhs: ID, out: &mut Vec<(ID, ID)>) {
        let mut work = vec![(lhs, rhs)];

        while let Some((lhs, rhs)) = work.pop() {
            // Insert returns false if the edge is already present
            if !self.downsets[lhs].insert(rhs) {
                continue;
            }
            self.upsets[rhs].insert(lhs);
            // Inform the caller that a new edge was added
            out.push((lhs, rhs));

            for &lhs2 in self.upsets[lhs].iter() {
                work.push((lhs2, rhs));
            }
            for &rhs2 in self.downsets[rhs].iter() {
                work.push((lhs, rhs2));
            }
        }
    }

    pub fn predecessors(&self, id: ID) -> impl Iterator<Item = ID> {
        self.upsets[id].iter().cloned()
    }

    pub fn successors(&self, id: ID) -> impl Iterator<Item = ID> {
        self.downsets[id].iter().cloned()
    }

    pub fn all_linked(&self, id: ID) -> impl Iterator<Item = ID> {
        self.predecessors(id).chain(self.successors(id))
    }
}

struct OrderedSet<T> {
    v: Vec<T>,
    s: HashSet<T>,
}

impl<T: std::fmt::Debug> std::fmt::Debug for OrderedSet<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrderedSet").field("v", &self.v).finish()
    }
}
impl<T> Default for OrderedSet<T> {
    fn default() -> Self {
        Self {
            v: Vec::new(),
            s: HashSet::new(),
        }
    }
}
impl<T: Eq + std::hash::Hash + Clone> OrderedSet<T> {
    fn insert(&mut self, value: T) -> bool {
        if self.s.insert(value.clone()) {
            self.v.push(value);
            true
        } else {
            false
        }
    }

    fn iter(&self) -> std::slice::Iter<T> {
        self.v.iter()
    }
}
