use std::collections::BTreeSet;

use crate::{
    ast::{AST, ASTS, SExp, SExpId},
    source::{Span, Spanned},
};

pub trait ASTBuilder {
    fn assemble(self, ast: &mut AST) -> SExp;
    fn dep(self, ast: &mut AST, span: Span) -> SExpId
    where
        Self: Sized,
    {
        let assembled = self.assemble(ast);
        ast.add_node(assembled, span)
    }
    fn spanned(self, ast: &mut AST, span: Span) -> Spanned<SExpId>
    where
        Self: Sized,
    {
        let id = self.dep(ast, span);
        Spanned::new(id, span)
    }

    fn build_spanned(self, asts: &mut ASTS, span: Span) -> Spanned<SExpId>
    where
        Self: Sized,
    {
        let built = self.build(asts, span);
        Spanned::new(built, span)
    }

    fn build(self, asts: &mut ASTS, span: Span) -> SExpId
    where
        Self: Sized,
    {
        let mut ast = asts.new_ast();
        let result = self.assemble(&mut ast);
        let id = ast.add_node(result, span);
        ast.set_root(id);
        asts.add_ast(ast);
        id
    }

    // fn assemble(self, ast: &mut AST) -> SExpId;
    // fn build(self, asts: &mut ASTS) -> SExpId
    // where
    //     Self: Sized,
    // {
    //     let mut ast = asts.new_ast();
    //     let result = self.assemble(&mut ast);
    //     asts.add_ast(ast);
    //     result
    // }
}

impl<F> ASTBuilder for F
where
    F: FnOnce(&mut AST) -> SExp,
{
    fn assemble(self, ast: &mut AST) -> SExp {
        self(ast)
    }
}

pub fn symbol(name: &str) -> impl ASTBuilder {
    |_ast: &mut AST| {
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
    move |_ast: &mut AST| SExp::String(s)
}

pub fn rest(name: &str, span: Span, mut rest: Vec<Spanned<SExpId>>) -> impl ASTBuilder {
    move |ast: &mut AST| {
        let first = name.dep(ast, span);
        let first = Spanned::new(first, span);
        rest.insert(0, first);
        rest.assemble(ast)
    }
}

pub fn error() -> impl ASTBuilder {
    |_ast: &mut AST| SExp::Error
}

impl ASTBuilder for &str {
    fn assemble(self, ast: &mut AST) -> SExp {
        symbol(self).assemble(ast)
    }
}

impl ASTBuilder for String {
    fn assemble(self, ast: &mut AST) -> SExp {
        symbol(&self).assemble(ast)
    }
}

pub fn quote(span: Span, exp: Spanned<impl ASTBuilder>) -> impl ASTBuilder {
    (Spanned::new(symbol("quote"), span), exp)
}

impl<T: ASTBuilder + Copy> ASTBuilder for &[Spanned<T>] {
    fn assemble(self, ast: &mut AST) -> SExp {
        let mut items = vec![];
        for item in self {
            items.push(item.dep(ast));
        }
        SExp::List(items)
    }
}
impl<T: ASTBuilder> ASTBuilder for BTreeSet<Spanned<T>> {
    fn assemble(self, ast: &mut AST) -> SExp {
        let mut items = vec![];
        for item in self {
            let item = item.dep(ast);
            items.push(item);
        }
        SExp::List(items)
    }
}
impl<T: ASTBuilder> ASTBuilder for Vec<Spanned<T>> {
    fn assemble(self, ast: &mut AST) -> SExp {
        let mut items = vec![];
        for item in self {
            let item = item.dep(ast);
            items.push(item);
        }
        SExp::List(items)
    }
}

pub fn list() -> impl ASTBuilder {
    |_ast: &mut AST| SExp::List(vec![])
}

impl ASTBuilder for SExpId {
    fn assemble(self, _ast: &mut AST) -> SExp {
        unreachable!()
    }
    fn dep(self, _ast: &mut AST, _span: Span) -> SExpId {
        self
    }
}

impl ASTBuilder for &SExpId {
    fn assemble(self, _ast: &mut AST) -> SExp {
        unreachable!()
    }
}

impl ASTBuilder for () {
    fn assemble(self, _ast: &mut AST) -> SExp {
        SExp::List(vec![])
    }
}

pub trait SpannedASTBuilder {
    fn dep(self, ast: &mut AST) -> SExpId;
}

impl<T: ASTBuilder> SpannedASTBuilder for Spanned<T> {
    fn dep(self, ast: &mut AST) -> SExpId {
        let span = self.span;
        self.inner().dep(ast, span)
    }
}

macro_rules! impl_list {
    ($($t:tt),*) => {
        #[allow(non_snake_case)]
        impl<$($t: ASTBuilder),*> ASTBuilder for ($(Spanned<$t>,)*)
        {
            fn assemble(self, ast: &mut AST) -> SExp {
                let ($($t,)*) = self;
                $( let $t = $t.dep(ast); )*

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
