#![allow(dead_code)]

pub enum Type {
    Number,
    String,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeId(usize);

#[derive(Default)]
pub struct TypeEnv {
    types: Vec<Type>,
}
