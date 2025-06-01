use ast::{ASTS, SExpId};
use lambda_lifting::LambdaPass;

pub mod ast;
pub mod builder;

pub mod lambda_lifting;
pub mod thunk_inserting;

pub mod types;

pub mod runtime;

pub fn process_ast(asts: &mut ASTS, mut root: SExpId) -> SExpId {
    root = LambdaPass::pass(asts, root);
    root
}
