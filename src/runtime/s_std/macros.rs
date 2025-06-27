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
        Spanned::new("fn", caller),
        Spanned::new(
            (
                Spanned::new(":self", caller),
                Spanned::new(":super", caller),
            ),
            caller,
        ),
        Spanned::new(
            |ast: &mut AST| {
                let mut items = args;
                items.insert(0, Spanned::new("obj/extend", caller).spanned(ast, caller));
                items.insert(1, Spanned::new("super", caller).spanned(ast, caller));

                items.assemble(ast)
            },
            caller,
        ),
    )
        .build_spanned(rt, caller))
}

pub fn let_star(
    rt: &mut ASTS,
    caller: Span,
    args: Vec<Spanned<SExpId>>,
) -> Result<Spanned<SExpId>, String> {
    match &args[..] {
        &[pattern, value] => Ok((
            Spanned::new("let-rec", caller),
            pattern,
            Spanned::new(
                (
                    Spanned::new("thunk", caller),
                    Spanned::new((), caller),
                    value,
                ),
                caller,
            ),
        )
            .build_spanned(rt, caller)),
        _ => Err("Expected two arguments".into()),
    }
}

pub fn obj_put_thunk(key: String, value: Spanned<impl ASTBuilder>, span: Span) -> impl ASTBuilder {
    let value = (Spanned::new("obj/construct-or", span), value);
    let value = (
        Spanned::new("thunk", span),
        Spanned::new((), span),
        Spanned::new(value, span),
    );
    (
        Spanned::new("obj/put", span),
        Spanned::new(format!(":{key}"), span),
        Spanned::new(value, span),
    )
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
            inner.push(obj_put_thunk(key.to_string(), value, caller).spanned(&mut ast, caller));
        } else {
            inner.push((Spanned::new("obj/eval", caller), arg_id).spanned(&mut ast, caller));
        }
    }

    inner.insert(0, "obj/condef".spanned(&mut ast, caller));
    let result = inner.spanned(&mut ast, caller);
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
        Spanned::new("obj/con", caller),
        Spanned::new(
            (
                Spanned::new("fn", caller),
                Spanned::new(
                    (
                        Spanned::new(":self", caller),
                        Spanned::new(":root", caller),
                        Spanned::new(":super", caller),
                        Spanned::new(":origin", caller),
                    ),
                    caller,
                ),
                Spanned::new(
                    move |ast: &mut AST| {
                        let mut items = args;
                        items.insert(0, Spanned::new("do".dep(ast, caller), caller));
                        items.push(Spanned::new("self".dep(ast, caller), caller));
                        (&items[..]).assemble(ast)
                    },
                    caller,
                ),
            ),
            caller,
        ),
    )
        .build_spanned(rt, caller))
}
pub fn objput(
    rt: &mut ASTS,
    caller: Span,
    args: Vec<Spanned<SExpId>>,
) -> Result<Spanned<SExpId>, String> {
    let Ok([key, value]) = TryInto::<[Spanned<SExpId>; 2]>::try_into(args) else {
        return Err("Expected two arguments".into());
    };

    Ok((
        Spanned::new("obj/insert", caller),
        Spanned::new("self", caller),
        key,
        value,
    )
        .build_spanned(rt, caller))
}

pub fn obj_add(
    rt: &mut ASTS,
    caller: Span,
    args: Vec<Spanned<SExpId>>,
) -> Result<Spanned<SExpId>, String> {
    match &args[..] {
        &[key, value] => Ok((
            Spanned::new("if", caller),
            Spanned::new(
                (
                    Spanned::new("has?", caller),
                    Spanned::new("super", caller),
                    key,
                ),
                caller,
            ),
            Spanned::new(
                (
                    Spanned::new("obj/put", caller),
                    key,
                    Spanned::new(
                        (
                            Spanned::new("+", caller),
                            Spanned::new((Spanned::new("super", caller), key), caller),
                            value,
                        ),
                        caller,
                    ),
                ),
                caller,
            ),
            Spanned::new((Spanned::new("obj/put", caller), key, value), caller),
        )
            .build_spanned(rt, caller)),
        arg => Err(format!("Expected two arguments. Found: {}", arg.len())),
    }
}
