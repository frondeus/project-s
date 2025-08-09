#![allow(dead_code)]

use crate::{
    ast::{AST, ASTS, SExpId},
    builder::ASTBuilder,
    source::{Span, Spanned},
};

pub fn obj_record(
    rt: &mut ASTS,
    caller: Span,
    args: Vec<Spanned<SExpId>>,
) -> Result<Spanned<SExpId>, String> {
    Ok((
        "fn",
        (":self", ":root", ":super"),
        |ast: &mut AST, caller: Span| {
            let mut items = args;
            items.insert(0, "obj/extend".assemble_id_with_span(ast, caller));
            items.insert(1, "super".assemble_id_with_span(ast, caller));

            items.assemble(ast, caller)
        },
    )
        .build_ast(rt, caller))
}

pub fn obj_put_thunk(key: String, value: impl ASTBuilder) -> impl ASTBuilder {
    let value = ("obj/construct-or", value);
    let value = ("thunk", (), value);
    ("obj/put", format!(":{key}"), value)
}

pub fn obj_struct(
    rt: &mut ASTS,
    caller: Span,
    args: Vec<Spanned<SExpId>>,
) -> Result<Spanned<SExpId>, String> {
    let mut args = args.into_iter();
    let mut inner = Vec::new();
    let mut ast = rt.new_ast();

    while let Some(arg_id) = args.next() {
        let arg = rt.get(arg_id.inner());
        if let Some(key) = arg.as_keyword() {
            let Some(value) = args.next() else {
                return Err("Expected value".into());
            };
            inner.push(obj_put_thunk(key.to_string(), value).assemble_id(&mut ast, caller));
        } else {
            inner.push(("obj/eval", arg_id).assemble_id(&mut ast, caller));
        }
    }

    inner.insert(0, "obj/condef".assemble_id(&mut ast, caller));
    let result = inner.assemble_id_with_span(&mut ast, caller);
    ast.set_root(result.inner());
    rt.add_ast(ast);
    // tracing::debug!("obj/struct: {}", rt.asts.fmt(result));
    Ok(result)
}

pub fn condef(
    rt: &mut ASTS,
    caller: Span,
    args: Vec<Spanned<SExpId>>,
) -> Result<Spanned<SExpId>, String> {
    Ok((
        "obj/con",
        (
            "fn",
            (":self", ":root", ":super", ":origin"),
            move |ast: &mut AST, caller: Span| {
                let mut items = args;
                items.insert(0, "do".assemble_id_with_span(ast, caller));
                items.push("self".assemble_id_with_span(ast, caller));
                (&items[..]).assemble(ast, caller)
            },
        ),
    )
        .build_ast(rt, caller))
}
pub fn objput(
    rt: &mut ASTS,
    caller: Span,
    args: Vec<Spanned<SExpId>>,
) -> Result<Spanned<SExpId>, String> {
    let Ok([key, value]) = TryInto::<[Spanned<SExpId>; 2]>::try_into(args) else {
        return Err("Expected two arguments".into());
    };

    Ok(("obj/insert", "self", key, value).build_ast(rt, caller))
}

pub fn obj_add(
    rt: &mut ASTS,
    caller: Span,
    args: Vec<Spanned<SExpId>>,
) -> Result<Spanned<SExpId>, String> {
    match &args[..] {
        &[key, value] => Ok((
            "if",
            ("has?", "super", key),
            ("obj/put", key, ("+", ("super", key), value)),
        )
            .build_ast(rt, caller)),
        arg => Err(format!("Expected two arguments. Found: {}", arg.len())),
    }
}

pub fn extend_macro(
    asts: &mut ASTS,
    caller: Span,
    mut args: Vec<Spanned<SExpId>>,
) -> Result<Spanned<SExpId>, String> {
    let last = args
        .pop()
        .ok_or("extend!: Expected at least one argument")?;
    let mut args = args.into_iter();
    let mut previous = args
        .next()
        .ok_or("extend!: Expected at least two arguments")?;
    let ast = asts.new_ast_mut();
    for arg in args {
        previous = ("extend-fn", previous, arg).assemble_id_with_span(ast, caller);
    }
    previous = ("extend", previous, last).assemble_id_with_span(ast, caller);
    Ok(previous)
}
