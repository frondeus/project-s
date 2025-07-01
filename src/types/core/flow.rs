use super::*;

impl TypeCheckerCore {
    #[allow(clippy::result_large_err)]
    pub fn flow(&mut self, lhs: Value, rhs: Use, diagnostics: &mut Diagnostics) {
        let mut pending_edges = vec![(lhs, rhs)];
        let mut type_pairs_to_check = Vec::new();
        while let Some((lhs, rhs)) = pending_edges.pop() {
            self.r.add_edge(lhs.0, rhs.0, &mut type_pairs_to_check);

            // Check if adding that edge resulted in any new type pairs needing to be checked
            while let Some((lhs, rhs)) = type_pairs_to_check.pop() {
                if let TypeNode::Value(lhs_head, lhs_span) = &self.types[lhs] {
                    if let TypeNode::Use(rhs_head, rhs_span) = &self.types[rhs] {
                        Self::check_heads(
                            &self.types,
                            &self.r,
                            lhs_head,
                            rhs_head,
                            lhs,
                            rhs,
                            *lhs_span,
                            *rhs_span,
                            &mut pending_edges,
                            diagnostics,
                        );
                    }
                }
            }
        }
        assert!(pending_edges.is_empty() && type_pairs_to_check.is_empty());
    }

    fn find_value<'a>(
        nodes: &'a [TypeNode],
        r: &Reachability,
        val: Value,
    ) -> Option<&'a VTypeHead> {
        let node = &nodes[val.0];
        match node {
            TypeNode::Var(_) => {
                let v = r.predecessors(val.0).find_map(|id| match &nodes[id] {
                    TypeNode::Value(head, _) => Some(head),
                    _ => None,
                });
                v
            }
            TypeNode::Value(head, _) => Some(head),
            TypeNode::Use(_, _) => None,
        }
    }

    #[allow(clippy::result_large_err, clippy::too_many_arguments)]
    fn check_heads(
        nodes: &[TypeNode],
        r: &Reachability,
        lhs: &VTypeHead,
        rhs: &UTypeHead,
        lhs_id: ID,
        rhs_id: ID,
        lhs_span: Span,
        rhs_span: Span,
        out: &mut Vec<(Value, Use)>,
        diagnostics: &mut Diagnostics,
    ) {
        use UTypeHead::*;
        use VTypeHead::*;

        match (lhs, rhs) {
            (_, UError) | (VError, _) => (), // We assume that error type is like ! type in Rust.
            (VLiteral(vlit), ULiteral(ulit)) if vlit == ulit => (),
            (VLiteral(vlit), UPrimitive(uprim)) if vlit.ty_() == uprim => (),
            (VPrimitive(lhs_name), UPrimitive(rhs_name)) if lhs_name == rhs_name => (),
            (VStruct { fields, proto }, UStruct { fields: fields_use }) => {
                for (field_name, field_use) in fields_use {
                    if let Some(field_ty) = fields.get(field_name) {
                        out.push((*field_ty, *field_use));
                    } else if let Some(proto) = proto {
                        out.push((*proto, Use(rhs_id)));
                    } else {
                        diagnostics
                            .add(lhs_span, format!("Object has no field: {}", field_name))
                            .add_extra("Defined here", Some(lhs_span))
                            .add_extra("Used here", Some(rhs_span));
                    }
                }
            }
            (
                &VList { item },
                &UApplication {
                    args,
                    ret: ret_use,
                    // field: (ref field_name, field_use),
                    // index: (index, index_use),
                    ..
                },
            ) => {
                out.push((item, ret_use));
                // out.push((args, index_use));
            }
            (
                VTuple { items },
                &UApplication {
                    args,
                    ret: ret_use,
                    first_arg,
                    // index: (index, index_use),
                    ..
                },
            ) => {
                let Some(first_arg) = first_arg else {
                    diagnostics.add(lhs_span, "Expected int literal to access tuple element");
                    return;
                };
                let Some(first_arg) = Self::find_value(nodes, r, first_arg) else {
                    diagnostics.add(lhs_span, "Expected int literal to access tuple element");
                    return;
                };
                let Some(lit) = first_arg.as_number_literal() else {
                    diagnostics.add(lhs_span, "Expected int literal to access tuple element");
                    return;
                };
                let index = lit as usize;
                if index >= items.len() {
                    diagnostics.add(
                        lhs_span,
                        format!(
                            "Tuple index out of bounds: {} expected {}",
                            index,
                            items.len()
                        ),
                    );
                    return;
                }
                out.push((items[index], ret_use));
            }
            (
                &VFunc { pattern, ret },
                &UApplication {
                    args, ret: ret_use, ..
                },
            ) => {
                out.push((args, pattern));
                out.push((ret, ret_use));
            }
            (
                VStruct { fields, proto },
                &UApplication {
                    args,
                    ret,
                    first_arg, // field: (ref field, field_use),
                },
            ) => {
                let Some(field) = first_arg else {
                    diagnostics
                        .add(rhs_span, "Expected field name")
                        .add_extra("Used here", Some(rhs_span))
                        .add_extra("Expected here", Some(lhs_span));
                    return;
                };

                let Some(field) = Self::find_value(nodes, r, field) else {
                    diagnostics
                        .add(rhs_span, "Expected keyword literal")
                        .add_extra("Used here", Some(rhs_span))
                        .add_extra("Expected here", Some(lhs_span));
                    return;
                };

                let Some(field) = field.as_keyword_literal() else {
                    diagnostics
                        .add(rhs_span, "Expected keyword literal")
                        .add_extra("Used here", Some(rhs_span))
                        .add_extra("Expected here", Some(lhs_span));
                    return;
                };

                if let Some(field_ty) = fields.get(&field) {
                    // out.push((args, field_use));
                    out.push((*field_ty, ret));
                } else if let Some(proto) = proto {
                    out.push((*proto, Use(rhs_id)));
                } else {
                    diagnostics
                        .add(rhs_span, format!("Undefined field: {}", field))
                        .add_extra("Used here", Some(rhs_span))
                        .add_extra("Record defined here", Some(lhs_span));
                }
            }
            (
                VTuple { items },
                &UList {
                    items: args,
                    min_len,
                    max_len,
                },
            ) => {
                if items.len() < min_len {
                    diagnostics.add(
                        lhs_span,
                        format!(
                            "Wrong number of arguments: {} expected {}",
                            items.len(),
                            min_len
                        ),
                    );
                }
                if let Some(max_len) = max_len {
                    if items.len() > max_len {
                        diagnostics.add(
                            lhs_span,
                            format!(
                                "Wrong number of arguments: {} expected {}",
                                items.len(),
                                max_len
                            ),
                        );
                    }
                }
                for item in items {
                    out.push((*item, args));
                }
            }
            (
                VFunc { pattern, ret },
                UFunc {
                    pattern: args,
                    ret: ret_use,
                },
            ) => {
                out.push((*args, *pattern));
                out.push((*ret, *ret_use));
            }
            (VList { item }, UList { items: args, .. }) => {
                out.push((*item, *args));
            }
            (VList { item }, UTuple { items: args }) => {
                // TODO: Length
                for arg in args {
                    out.push((*item, *arg));
                }
            }
            (VTuple { items }, UTuple { items: args }) => {
                if items.len() != args.len() {
                    diagnostics.add(
                        lhs_span,
                        format!(
                            "Wrong number of arguments: {} expected {}",
                            items.len(),
                            args.len()
                        ),
                    );
                }

                for (item, arg) in items.iter().zip(args) {
                    out.push((*item, *arg));
                }
            }
            (
                &VRef { write, read },
                &URef {
                    write: write_use,
                    read: read_use,
                },
            ) => {
                if write.is_none() && read.is_none() {
                    diagnostics.add(rhs_span, "Reference is not readable or writable");
                    return;
                }

                if let Some(read_use) = read_use {
                    if let Some(read) = read {
                        out.push((read, read_use));
                    } else {
                        diagnostics.add(rhs_span, "Reference is not readable");
                    }
                }
                if let Some(write_use) = write_use {
                    if let Some(write) = write {
                        out.push((write_use, write));
                    } else {
                        diagnostics.add(rhs_span, "Reference is not writable");
                    }
                }
            }
            _ => {
                diagnostics
                    .add(rhs_span, "Incompatible types")
                    .add_extra(format!("Expected {rhs}"), Some(rhs_span))
                    .add_extra(format!("But got {lhs}"), Some(lhs_span));
            }
        }
    }
}
