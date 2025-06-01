use std::collections::BTreeSet;

use crate::ast::{AST, ASTS, SExp, SExpId};

pub trait ASTBuilder {
    fn assemble(self, ast: &mut AST) -> SExpId;
    fn build(self, asts: &mut ASTS) -> SExpId
    where
        Self: Sized,
    {
        let mut ast = asts.new_ast();
        let result = self.assemble(&mut ast);
        asts.add_ast(ast);
        result
    }
}

impl<F> ASTBuilder for F
where
    F: FnOnce(&mut AST) -> SExpId,
{
    fn assemble(self, ast: &mut AST) -> SExpId {
        self(ast)
    }
}

pub fn symbol(name: &str) -> impl ASTBuilder {
    |ast: &mut AST| ast.add_node(SExp::Symbol(name.to_string()))
}

pub fn error() -> impl ASTBuilder {
    |ast: &mut AST| ast.add_node(SExp::Error)
}

impl ASTBuilder for &str {
    fn assemble(self, ast: &mut AST) -> SExpId {
        symbol(self).assemble(ast)
    }
}

impl ASTBuilder for String {
    fn assemble(self, ast: &mut AST) -> SExpId {
        symbol(&self).assemble(ast)
    }
}

pub fn quote(exp: impl ASTBuilder) -> impl ASTBuilder {
    (symbol("quote"), exp)
}

impl<T: ASTBuilder> ASTBuilder for BTreeSet<T> {
    fn assemble(self, ast: &mut AST) -> SExpId {
        let list = ast.reserve();
        let mut items = vec![];
        for item in self {
            items.push(item.assemble(ast));
        }
        ast.set(list, SExp::List(items));
        list
    }
}

pub fn list() -> impl ASTBuilder {
    |ast: &mut AST| {
        let list = ast.reserve();
        ast.add_node(SExp::List(vec![]));
        list
    }
}

impl ASTBuilder for SExpId {
    fn assemble(self, _ast: &mut AST) -> SExpId {
        self
    }
}

impl ASTBuilder for &SExpId {
    fn assemble(self, _ast: &mut AST) -> SExpId {
        *self
    }
}

impl ASTBuilder for () {
    fn assemble(self, ast: &mut AST) -> SExpId {
        ast.add_node(SExp::List(vec![]))
    }
}

macro_rules! impl_list {
    ($($t:tt),*) => {
        #[allow(non_snake_case)]
        impl<$($t: ASTBuilder),*> ASTBuilder for ($($t,)*)
        {
            fn assemble(self, ast: &mut AST) -> SExpId {
                let list = ast.reserve();
                let ($($t,)*) = self;
                $( let $t = $t.assemble(ast); )*

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
impl_list!(T1, T2, T3, T4, T5, T6);
impl_list!(T1, T2, T3, T4, T5, T6, T7);
impl_list!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_list!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_list!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_list!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_list!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_list!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_list!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
impl_list!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15
);
impl_list!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
);
