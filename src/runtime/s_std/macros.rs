use crate::{
    ast::{AST, SExpId},
    builder::ASTBuilder,
    runtime::Runtime,
};
pub fn let_star(rt: &mut Runtime, args: Vec<SExpId>) -> Result<SExpId, String> {
    match &args[..] {
        [pattern, value] => Ok(("let-rec", pattern, ("thunk", (), value)).build(&mut rt.asts)),
        _ => Err("Expected two arguments".into()),
    }
}

pub fn obj_put_thunk(key: String, value: impl ASTBuilder) -> impl ASTBuilder {
    let value = ("obj/construct-or", value);
    let value = ("thunk", ("self", "root", "super", "origin"), value);
    ("obj/put", format!(":{key}"), value)
}

pub fn obj_struct(rt: &mut Runtime, args: Vec<SExpId>) -> Result<SExpId, String> {
    let mut args = args.into_iter();
    let mut inner = Vec::new();
    let mut ast = rt.asts.new_ast();

    while let Some(arg_id) = args.next() {
        let arg = rt.asts.get(arg_id);
        if let Some(key) = arg.item.as_keyword() {
            let Some(value) = args.next() else {
                return Err("Expected value".into());
            };
            inner.push(obj_put_thunk(key.to_string(), value).assemble(&mut ast));
        } else {
            inner.push(("obj/eval", arg_id).assemble(&mut ast));
        }
    }

    inner.insert(0, "obj/condef".assemble(&mut ast));
    let result = inner.assemble(&mut ast);
    rt.asts.add_ast(ast);
    // tracing::debug!("obj/struct: {}", rt.asts.fmt(result));
    Ok(result)
}

pub fn condef(rt: &mut Runtime, args: Vec<SExpId>) -> Result<SExpId, String> {
    Ok((
        "obj/con",
        (
            "fn",
            (":self", ":root", ":super", ":origin"),
            move |ast: &mut AST| {
                let mut items = args;
                items.insert(0, "do".assemble(ast));
                items.push("self".assemble(ast));
                items.assemble(ast)
            },
        ),
    )
        .build(&mut rt.asts))
}
pub fn objput(rt: &mut Runtime, args: Vec<SExpId>) -> Result<SExpId, String> {
    let Ok([key, value]) = TryInto::<[SExpId; 2]>::try_into(args) else {
        return Err("Expected two arguments".into());
    };

    Ok(("obj/insert", "self", key, value).build(&mut rt.asts))
}

pub fn obj_add(rt: &mut Runtime, args: Vec<SExpId>) -> Result<SExpId, String> {
    match &args[..] {
        [key, value] => Ok((
            "if",
            ("has?", "super", key),
            ("obj/put", key, ("+", ("super", key), value)),
            ("obj/put", key, value),
        )
            .build(&mut rt.asts)),
        arg => Err(format!("Expected two arguments. Found: {}", arg.len())),
    }
}
