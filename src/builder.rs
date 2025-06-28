use std::collections::BTreeSet;

use crate::{
    ast::{AST, ASTS, SExp, SExpId},
    source::{Span, Spanned},
};

pub trait ASTBuilder: Sized {
    fn assemble(self, ast: &mut AST, caller: Span) -> SExp;
    fn assemble_with_span(self, ast: &mut AST, caller: Span) -> Spanned<SExp> {
        Spanned::new(self.assemble(ast, caller), caller)
    }
    fn assemble_id(self, ast: &mut AST, caller: Span) -> SExpId {
        let sexp = self.assemble_with_span(ast, caller);
        let span = sexp.span;
        ast.add_node(sexp.inner(), span)
    }
    fn assemble_id_with_span(self, ast: &mut AST, caller: Span) -> Spanned<SExpId> {
        let sexp = self.assemble_with_span(ast, caller);
        let span = sexp.span;
        sexp.map(|sexp| ast.add_node(sexp, span))
    }

    fn build_ast(self, asts: &mut ASTS, caller: Span) -> Spanned<SExpId> {
        let mut ast = asts.new_ast();
        let root = self.assemble_id_with_span(&mut ast, caller);
        ast.set_root(root.inner());
        asts.add_ast(ast);
        root
    }
}

impl<F> ASTBuilder for F
where
    F: FnOnce(&mut AST, Span) -> SExp,
{
    fn assemble(self, ast: &mut AST, caller: Span) -> SExp {
        self(ast, caller)
    }
}

impl ASTBuilder for Spanned<SExpId> {
    fn assemble_with_span(self, _ast: &mut AST, _caller: Span) -> Spanned<SExp> {
        unreachable!()
    }
    fn assemble(self, _ast: &mut AST, _caller: Span) -> SExp {
        unreachable!()
    }
    fn assemble_id(self, _ast: &mut AST, _caller: Span) -> SExpId {
        self.inner()
    }
    fn assemble_id_with_span(self, _ast: &mut AST, _caller: Span) -> Spanned<SExpId> {
        self
    }
}

pub fn symbol(name: &str) -> impl ASTBuilder {
    |_ast: &mut AST, _caller: Span| {
        if name.starts_with(":") {
            let name = name.trim_start_matches(':');
            SExp::Keyword(name.to_string())
        } else {
            SExp::Symbol(name.to_string())
        }
    }
}

pub fn string(s: impl ToString) -> impl ASTBuilder {
    let s = s.to_string();
    move |_ast: &mut AST, _caller: Span| SExp::String(s)
}

pub fn rest(name: &str, rest: Vec<impl ASTBuilder>) -> impl ASTBuilder {
    move |ast: &mut AST, caller: Span| {
        let first = name.assemble_id(ast, caller);
        let mut rest = rest
            .into_iter()
            .map(|item| item.assemble_id(ast, caller))
            .collect::<Vec<_>>();
        rest.insert(0, first);
        rest.assemble(ast, caller)
    }
}

pub fn error() -> impl ASTBuilder {
    |_ast: &mut AST, _caller: Span| SExp::Error
}

impl ASTBuilder for &str {
    fn assemble(self, ast: &mut AST, caller: Span) -> SExp {
        symbol(self).assemble(ast, caller)
    }
}

impl ASTBuilder for String {
    fn assemble(self, ast: &mut AST, caller: Span) -> SExp {
        symbol(&self).assemble(ast, caller)
    }
}

pub fn quote(exp: impl ASTBuilder) -> impl ASTBuilder {
    ("quote", exp)
}

impl<T: ASTBuilder + Copy> ASTBuilder for &[T] {
    fn assemble(self, ast: &mut AST, caller: Span) -> SExp {
        let mut items = vec![];
        for item in self {
            items.push(item.assemble_id(ast, caller));
        }
        SExp::List(items)
    }
}
impl<T: ASTBuilder> ASTBuilder for BTreeSet<T> {
    fn assemble(self, ast: &mut AST, caller: Span) -> SExp {
        let mut items = vec![];
        for item in self {
            let item = item.assemble_id(ast, caller);
            items.push(item);
        }
        SExp::List(items)
    }
}
impl<T: ASTBuilder> ASTBuilder for Vec<T> {
    fn assemble(self, ast: &mut AST, caller: Span) -> SExp {
        let mut items = vec![];
        for item in self {
            let item = item.assemble_id(ast, caller);
            items.push(item);
        }
        SExp::List(items)
    }
}

pub fn list() -> impl ASTBuilder {
    |_ast: &mut AST, _caller: Span| SExp::List(vec![])
}

impl ASTBuilder for SExpId {
    fn assemble(self, _ast: &mut AST, _caller: Span) -> SExp {
        unreachable!()
    }
    fn assemble_id(self, _ast: &mut AST, _caller: Span) -> SExpId {
        self
    }
}

// impl<T: ASTBuilder + Copy> ASTBuilder for &T {
//     fn assemble(self, ast: &mut AST, caller: Span) -> SExp {
//         (*self).assemble(ast, caller)
//     }
// }

impl ASTBuilder for &SExpId {
    fn assemble(self, _ast: &mut AST, _caller: Span) -> SExp {
        unreachable!()
    }
    fn assemble_id(self, _ast: &mut AST, _caller: Span) -> SExpId {
        *self
    }
}

impl ASTBuilder for () {
    fn assemble(self, _ast: &mut AST, _caller: Span) -> SExp {
        SExp::List(vec![])
    }
}

macro_rules! impl_list {
    ($($t:tt),*) => {
        #[allow(non_snake_case)]
        impl<$($t: ASTBuilder),*> ASTBuilder for ($($t,)*)
        {
            fn assemble(self, ast: &mut AST, caller: Span) -> SExp {
                let ($($t,)*) = self;
                $( let $t = $t.assemble_id(ast, caller); )*

                SExp::List(vec![$($t),*])
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
