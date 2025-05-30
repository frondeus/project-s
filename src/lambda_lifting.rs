use std::collections::HashSet;

use crate::ast::{AST, ASTS, SExp, SExpId};

pub fn lift_lambdas(asts: &mut ASTS, root: SExpId) -> Option<SExpId> {
    if let SExp::List(sexp_ids) = asts.get(root) {
        let first_id = sexp_ids[0];
        let first = asts.get(first_id).as_symbol().unwrap();
        if first == "fn" {
            let signature_id = sexp_ids[1];
            let signature = asts.get(signature_id).as_list().unwrap().to_vec();
            let mut body = sexp_ids[2];

            let mut edited = false;
            if let Some(new_body) = lift_lambdas(asts, body) {
                body = new_body;
                edited = true;
            }

            if let Some((new_body, free_vars)) = process_fn_decl(asts, signature.clone(), body) {
                let mut ast = AST::default();
                let id = ast.reserve();
                let closure_symbol = ast.add_node(SExp::Symbol("cl".to_string()));
                let captured_id = ast.reserve();

                let free_vars = free_vars
                    .into_iter()
                    .map(|v| ast.add_node(SExp::Symbol(v)))
                    .collect();

                ast.set(captured_id, SExp::List(free_vars));

                ast.set(
                    id,
                    SExp::List(vec![closure_symbol, signature_id, captured_id, new_body]),
                );
                asts.add_ast(ast);
                Some(id)
            } else if edited {
                let mut ast = AST::default();
                let id = ast.reserve();
                ast.set(id, SExp::List(vec![first_id, signature_id, body]));
                asts.add_ast(ast);
                Some(id)
            } else {
                None
            }
        } else {
            let mut new_sexp_ids = sexp_ids.clone();
            let mut edited = false;
            for id in &mut new_sexp_ids {
                if let Some(new_id) = lift_lambdas(asts, *id) {
                    *id = new_id;
                    edited = true;
                }
            }
            if edited {
                let mut ast = AST::default();
                let id = ast.reserve();
                ast.set(id, SExp::List(new_sexp_ids));
                asts.add_ast(ast);
                Some(id)
            } else {
                None
            }
        }
    } else {
        None
    }
}

fn process_fn_decl(
    asts: &mut ASTS,
    signature: Vec<SExpId>,
    body: SExpId,
) -> Option<(SExpId, HashSet<String>)> {
    println!("processing fn decl: {}", asts.fmt(body));
    let mut free_vars = HashSet::<String>::new();
    let signature = signature
        .iter()
        .map(|id| asts.get(*id).as_symbol().unwrap().to_string())
        .collect::<Vec<String>>();

    let body = process_fn_decl_body(asts, body, &signature, &mut free_vars);
    body.map(|id| (id, free_vars))
    // todo!()
}

const SPECIAL_FORMS: &[&str] = &["quasiquote", "+", "unquote"];

fn process_quasiquote(
    asts: &mut ASTS,
    sexp_ids: Vec<SExpId>,
    signature: &[String],
    free_vars: &mut HashSet<String>,
) -> Option<SExpId> {
    println!("processing quasiquote: {}", asts.fmt_list(&sexp_ids));
    let mut new_sexp_ids = sexp_ids.clone();
    let mut edited = false;
    for id in &mut new_sexp_ids {
        if let Some(list) = asts.get(*id).as_list() {
            let first = list[0];
            let list = list.to_vec();
            if is_symbol(first, asts, "unquote") {
                if let Some(new_id) = process_unquote(asts, list.clone(), signature, free_vars) {
                    *id = new_id;
                    edited = true;
                }
            }
            if let Some(new_id) = process_quasiquote(asts, list, signature, free_vars) {
                *id = new_id;
                edited = true;
            }
        }
    }
    if edited {
        let mut ast = AST::default();
        let id = ast.reserve();
        ast.set(id, SExp::List(new_sexp_ids));
        asts.add_ast(ast);
        Some(id)
    } else {
        None
    }
}

fn process_unquote(
    asts: &mut ASTS,
    sexp_ids: Vec<SExpId>,
    signature: &[String],
    free_vars: &mut HashSet<String>,
) -> Option<SExpId> {
    println!("processing unquote: {}", asts.fmt(sexp_ids[0]));
    let mut new_sexp_ids = sexp_ids.clone();
    let mut edited = false;
    for id in &mut new_sexp_ids {
        if let Some(new_id) = process_fn_decl_body(asts, *id, signature, free_vars) {
            *id = new_id;
            edited = true;
        }
    }
    if edited {
        let mut ast = AST::default();
        let id = ast.reserve();
        ast.set(id, SExp::List(new_sexp_ids));
        asts.add_ast(ast);
        Some(id)
    } else {
        None
    }
}
fn process_fn_decl_body(
    asts: &mut ASTS,
    body: SExpId,
    signature: &[String],
    free_vars: &mut HashSet<String>,
) -> Option<SExpId> {
    match asts.get(body) {
        SExp::Symbol(s) if SPECIAL_FORMS.contains(&s.as_str()) => None,
        SExp::Symbol(s) => {
            if !signature.contains(s) {
                free_vars.insert(s.clone());
                let mut ast = AST::default();
                let id = ast.reserve();
                // TODO : This is not very safe. We should have a special symbol category that is generated
                // by the pass and does not collide with any other symbol.
                let closure = ast.add_node(SExp::Symbol("_closure".to_string()));
                let symbol = ast.add_node(SExp::Symbol(s.clone()));
                ast.set(id, SExp::List(vec![closure, symbol]));
                asts.add_ast(ast);

                Some(id)
            } else {
                None
            }
        }
        SExp::List(sexp_ids) => {
            let first = sexp_ids[0];
            if is_symbol(first, asts, "quote") {
                return None;
            }
            if is_symbol(first, asts, "quasiquote") {
                return process_quasiquote(asts, sexp_ids.clone(), signature, free_vars);
            }

            let mut new_sexp_ids = sexp_ids.clone();
            let mut edited = false;
            for id in &mut new_sexp_ids {
                if let Some(new_id) = process_fn_decl_body(asts, *id, signature, free_vars) {
                    *id = new_id;
                    edited = true;
                }
            }
            if edited {
                let mut ast = AST::default();
                let id = ast.reserve();
                ast.set(id, SExp::List(new_sexp_ids));
                asts.add_ast(ast);
                Some(id)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn is_symbol(sexp_id: SExpId, asts: &ASTS, symbol: &str) -> bool {
    match asts.get(sexp_id) {
        SExp::Symbol(s) => s == symbol,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integration() -> test_runner::Result {
        test_runner::test_snapshots("docs/", "lift", |input, _deps| {
            eprintln!("---");
            let ast = crate::ast::AST::parse(input).unwrap();
            let root_id = ast.root_id().unwrap();
            let mut asts = ASTS::new(ast);
            let new_root = lift_lambdas(&mut asts, root_id).unwrap_or(root_id);
            let output = asts.fmt(new_root);
            output.to_string()
        })
    }
}
