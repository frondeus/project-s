use crate::ast::{AST, SExp, SExpId};

pub trait ASTBuilder {
    fn build(self, ast: &mut AST) -> SExpId;
}

impl<F> ASTBuilder for F
where
    F: FnOnce(&mut AST) -> SExpId,
{
    fn build(self, ast: &mut AST) -> SExpId {
        self(ast)
    }
}

pub fn symbol(name: &str) -> impl ASTBuilder {
    |ast: &mut AST| ast.add_node(SExp::Symbol(name.to_string()))
}

impl ASTBuilder for SExpId {
    fn build(self, _ast: &mut AST) -> SExpId {
        self
    }
}

impl ASTBuilder for () {
    fn build(self, ast: &mut AST) -> SExpId {
        ast.add_node(SExp::List(vec![]))
    }
}

macro_rules! impl_list {
    ($($t: ty),*) => {
        impl<
        $($t),*
        > ASTBuilder for ($($t,)*)
        where
            $($t: ASTBuilder),*
        {
            fn build(self, ast: &mut AST) -> SExpId {
                let ($($t),*) = self;
                let list = ast.reserve();
                // let $($t) = $($t.build(ast)),*;
                $(
                    let $t = $t.build(ast);
                )*

                ast.set(list, SExp::List(vec![$($t),*]));
                list
            }
        }
    }
}

impl_list!(T1);
impl_list!(T1, T2);
impl_list!(T1, T2, T3);
impl_list!(T1, T2, T3, T4);
impl_list!(T1, T2, T3, T4, T5);
