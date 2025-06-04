use ast::{ASTS, SExpId};
use lambda_lifting::LambdaPass;
use thunk_inserting::ThunkPass;

pub mod ast;
pub mod builder;

pub mod visitor;

pub mod lambda_lifting;
pub mod thunk_inserting;

pub mod types;

pub mod runtime;

pub fn process_ast(asts: &mut ASTS, mut root: SExpId) -> SExpId {
    root = LambdaPass::pass(asts, root);
    root = ThunkPass::pass(asts, root);
    root
}
