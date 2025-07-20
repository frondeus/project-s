use std::cmp::Ordering;

use super::*;

impl TypeEnv {
    pub(crate) fn constrain(
        &mut self,
        lhs_id: InferedTypeId,
        rhs_id: InferedTypeId,
        diagnostics: &mut Diagnostics,
    ) {
        if self.constraint_cache.contains(&(lhs_id, rhs_id)) {
            return;
        }
        let mut queue = VecDeque::new();
        queue.push_back((lhs_id, rhs_id));

        loop {
            let Some((lhs_id, rhs_id)) = queue.pop_front() else {
                return;
            };
            if self.constraint_cache.contains(&(lhs_id, rhs_id)) {
                continue;
            }
            self.constraint_cache.insert((lhs_id, rhs_id));
            self.constraints.push((lhs_id, rhs_id));
            let lhs = self.get(lhs_id);
            let rhs = self.get(rhs_id);

            tracing::trace!(
                "Constraining ({} <: {}): {} <: {}",
                lhs_id.0,
                rhs_id.0,
                lhs,
                rhs
            );
            let lhs_span = lhs.span();
            let rhs_span = rhs.span();

            use InferedType::*;
            match (lhs, rhs) {
                (Error { span: _ }, _) | (_, Error { span: _ }) => continue,
                (
                    Primitive {
                        name: lhs_name,
                        span: _,
                    },
                    Primitive {
                        name: rhs_name,
                        span: _,
                    },
                ) if lhs_name == rhs_name => continue,
                (
                    Literal {
                        value: lhs_value,
                        span: _,
                    },
                    Literal {
                        value: rhs_value,
                        span: _,
                    },
                ) if lhs_value == rhs_value => continue,
                (
                    Literal {
                        value: lhs_value,
                        span: _,
                    },
                    Primitive {
                        name: rhs_name,
                        span: _,
                    },
                ) => match lhs_value {
                    LitValue::Bool(_) if rhs_name == "bool" => continue,
                    LitValue::Number(_) if rhs_name == "number" => continue,
                    LitValue::String(_) if rhs_name == "string" => continue,
                    LitValue::Keyword(_) if rhs_name == "keyword" => continue,
                    _ => (),
                },

                (
                    &Function {
                        lhs: pattern,
                        rhs: ret,
                        span: _,
                    },
                    &Function {
                        lhs: args,
                        rhs: ret_use,
                        span: _,
                    },
                ) => {
                    queue.push_back((args, pattern));
                    queue.push_back((ret, ret_use));
                    continue;
                }
                (
                    &Function {
                        lhs: pattern,
                        rhs: ret,
                        span: _,
                    },
                    &Applicative {
                        arg: args,
                        ret: ret_use,
                        first_arg: _,
                        span: _,
                    },
                ) => {
                    queue.push_back((args, pattern));
                    queue.push_back((ret, ret_use));
                    continue;
                }
                (
                    &Record {
                        fields: ref lhs,
                        proto: lhs_proto,
                        span: _,
                    },
                    &Record {
                        fields: ref rhs,
                        proto: rhs_proto,
                        span: _,
                    },
                ) => {
                    for (key, r) in rhs {
                        if let Some(l) = lhs.get(key) {
                            queue.push_back((*l, *r));
                        } else if let Some(lhs_proto) = lhs_proto {
                            queue.push_back((lhs_proto, rhs_id));
                        } else {
                            diagnostics
                                .add(rhs_span, "Field not found")
                                .add_extra("Accessed here", Some(rhs_span))
                                .add_extra("Record defined here", Some(lhs_span));
                        }
                    }
                    if let Some(rhs_proto) = rhs_proto {
                        queue.push_back((lhs_id, rhs_proto));
                    }
                    continue;
                }
                (
                    &Record {
                        ref fields,
                        proto,
                        span: _,
                    },
                    &Applicative {
                        arg: _,
                        ret,
                        first_arg,
                        span: _,
                    },
                ) => {
                    let Some(field) = first_arg.and_then(|first| {
                        self.find_in_all_relatives(first, InferedType::as_keyword_literal)
                    }) else {
                        diagnostics
                            .add(rhs_span, "expected a keyword literal")
                            .add_extra("This is a record", Some(lhs_span))
                            .add_extra(
                                "But here we have an application that is not asking about field",
                                Some(rhs_span),
                            );
                        continue;
                    };

                    if let Some(field_ty) = fields
                        .iter()
                        .find_map(|(name, ty)| if name == field { Some(ty) } else { None })
                    {
                        queue.push_back((*field_ty, ret));
                    } else if let Some(proto) = proto {
                        queue.push_back((proto, rhs_id));
                    } else {
                        diagnostics
                            .add(rhs_span, "Undefined field: {field}")
                            .add_extra("Used here", Some(rhs_span))
                            .add_extra("Record defined here", Some(lhs_span));
                    }
                    continue;
                }
                (
                    Module {
                        members: lhs,
                        span: _,
                    },
                    &Record {
                        fields: ref rhs,
                        proto: rhs_proto,
                        span: _,
                    },
                ) => {
                    let rhs = rhs.clone();
                    let lhs = lhs.clone();
                    for (key, r) in rhs {
                        if let Some(l) = lhs.get(&key).copied() {
                            let level = r.level(self);
                            let l = l.instantiate(self, level);
                            queue.push_back((l, r));
                        } else {
                            diagnostics
                                .add(rhs_span, "Field not found")
                                .add_extra("Accessed here", Some(rhs_span))
                                .add_extra("Module defined here", Some(lhs_span));
                        }
                    }
                    if let Some(rhs_proto) = rhs_proto {
                        queue.push_back((lhs_id, rhs_proto));
                    }
                    continue;
                }
                (
                    Module { members, span: _ },
                    &Applicative {
                        arg: _,
                        ret,
                        first_arg,
                        span: _,
                    },
                ) => {
                    let Some(member) = first_arg.and_then(|first| {
                        self.find_in_predecessors(first, InferedType::as_keyword_literal)
                    }) else {
                        diagnostics
                            .add(rhs_span, "expected a keyword literal")
                            .add_extra("This is a module", Some(lhs_span))
                            .add_extra(
                                "But here we have an application that is not asking about member",
                                Some(rhs_span),
                            );
                        continue;
                    };

                    if let Some(member_scheme) = members.get(member).copied() {
                        let rhs_level = rhs_id.level(self);
                        let member_ty = member_scheme.instantiate(self, rhs_level);
                        queue.push_back((member_ty, ret));
                    } else {
                        diagnostics
                            .add(rhs_span, format!("Undefined member: {member}"))
                            .add_extra("Used here", Some(rhs_span))
                            .add_extra("Module defined here", Some(lhs_span));
                    }
                    continue;
                }
                (
                    Tuple {
                        items: left,
                        rest: left_splice,
                        span: _,
                    },
                    &Applicative {
                        arg: _,
                        ret,
                        first_arg,
                        span: _,
                    },
                ) => {
                    let Some(index) = first_arg.and_then(|first| {
                        self.find_in_predecessors(first, InferedType::as_number_literal)
                    }) else {
                        diagnostics
                            .add(rhs_span, "expected a number literal")
                            .add_extra("This is a tuple", Some(lhs_span))
                            .add_extra(
                                "But here we have an application that is not asking about index",
                                Some(rhs_span),
                            );
                        continue;
                    };
                    // TODO : Make it more safe
                    let index = index as usize;
                    if index >= left.len() && left_splice.is_none() {
                        diagnostics
                            .add(rhs_span, "index out of bounds")
                            .add_extra(
                                format!("This is a tuple that has {} elements", left.len()),
                                Some(lhs_span),
                            )
                            .add_extra(
                                format!("But here we ask about {index} element"),
                                Some(rhs_span),
                            )
                            .add_extra("Note, tuples are zero-indexed", None);
                        continue;
                    } else if index >= left.len()
                        && let Some(rest) = left_splice
                    {
                        queue.push_back((*rest, ret));
                        continue;
                    }
                    queue.push_back((left[index], ret));
                    continue;
                }
                (
                    &List {
                        item: left,
                        span: _,
                    },
                    &Applicative {
                        arg: _,
                        ret,
                        first_arg,
                        span: _,
                    },
                ) => {
                    let Some(_index) = first_arg.and_then(|first| {
                        self.find_in_predecessors(first, InferedType::as_number_literal)
                    }) else {
                        diagnostics
                            .add(rhs_span, "expected a number literal")
                            .add_extra("This is a list", Some(lhs_span))
                            .add_extra(
                                "But here we have an application that is not asking about index",
                                Some(rhs_span),
                            );
                        continue;
                    };
                    // TODO : Make it more safe
                    // let index = index as usize;
                    // if index >= left.len() {
                    //     diagnostics
                    //         .add(rhs_span, "index out of bounds")
                    //         .add_extra(
                    //             format!("This is a tuple that has {} elements", left.len()),
                    //             Some(lhs_span),
                    //         )
                    //         .add_extra(
                    //             format!("But here we ask about {index} element"),
                    //             Some(rhs_span),
                    //         )
                    //         .add_extra("Note, tuples are zero-indexed", None);
                    //     continue;
                    // } else if index >= left.len()
                    //     && let Some(rest) = left_rest
                    // {
                    //     queue.push_back((*rest, ret));
                    //     continue;
                    // }
                    queue.push_back((left, ret));
                    continue;
                }
                (
                    // Every tuple can be treated as a list if it has the same type for every element.
                    Tuple {
                        items: left,
                        rest: left_splice,
                        span: _,
                    },
                    &List {
                        item: right,
                        span: _,
                    },
                ) => {
                    for left in left {
                        queue.push_back((*left, right));
                    }
                    if let Some(rest) = left_splice {
                        queue.push_back((*rest, rhs_id));
                    }
                    continue;
                }
                (
                    &List {
                        item: left,
                        span: _,
                    },
                    &List {
                        item: right,
                        span: _,
                    },
                ) => {
                    queue.push_back((left, right));
                    continue;
                }
                (
                    &List {
                        item: left,
                        span: _,
                    },
                    &Tuple {
                        items: ref right,
                        rest: Some(right_rest),
                        span: _,
                    },
                ) => {
                    for item in right {
                        queue.push_back((left, *item));
                    }
                    queue.push_back((left, right_rest));

                    continue;
                }
                (
                    &Tuple {
                        items: ref left,
                        rest: left_splice,
                        span: _,
                    },
                    &Tuple {
                        items: ref right,
                        rest: right_rest,
                        span: _,
                    },
                ) => {
                    let cmp = left.len().cmp(&right.len());
                    use Ordering::*;
                    tracing::trace!("Tuple vs Tuple");
                    tracing::trace!("Left: {left:?}..{left_splice:?}");
                    tracing::trace!("Right: {right:?}..{right_rest:?}");

                    match (left_splice, right_rest, cmp) {
                        (None, None, Equal) => {
                            for (l, r) in left.iter().copied().zip(right.iter().copied()) {
                                queue.push_back((l, r));
                            }
                            continue;
                        }
                        (None, None, Greater | Less)
                        | (None, Some(_), Less)
                        | (Some(_), None, Greater) => {
                            diagnostics
                                .add(lhs_span, "Type mismatch")
                                .add_extra(
                                    format!("Expected tuple of length {}", right.len()),
                                    Some(rhs_span),
                                )
                                .add_extra(
                                    format!("But found tuple of length {}", left.len()),
                                    Some(lhs_span),
                                );
                            continue;
                        }
                        (None, Some(right_rest), Equal | Greater) => {
                            for (l, r) in left
                                .iter()
                                .copied()
                                .take(right.len())
                                .zip(right.iter().copied())
                            {
                                queue.push_back((l, r));
                            }
                            for (l, r) in left
                                .iter()
                                .copied()
                                .skip(right.len())
                                .zip(std::iter::repeat(right_rest))
                            {
                                queue.push_back((l, r));
                            }
                            continue;
                        }
                        (Some(left_splice), None, Less | Equal) => {
                            for (l, r) in left
                                .iter()
                                .copied()
                                .zip(right.iter().take(left.len()).copied())
                            {
                                queue.push_back((l, r));
                            }
                            let right_rest = right.iter().skip(left.len()).copied().collect();
                            let new_tuple = self.tuple(right_rest, None, rhs_span);
                            queue.push_back((left_splice, new_tuple));
                            continue;
                        }
                        (Some(left_splice), Some(right_rest), _) => {
                            for (l, r) in left
                                .iter()
                                .take(right.len())
                                .copied()
                                .zip(right.iter().take(left.len()).copied())
                            {
                                queue.push_back((l, r));
                            }
                            for (r, l) in std::iter::repeat(right_rest)
                                .zip(left.iter().skip(right.len()).copied())
                            {
                                queue.push_back((l, r));
                            }
                            let new_right: Vec<_> =
                                right.iter().skip(left.len()).copied().collect();
                            tracing::error!("new_right: {:?}", new_right);
                            let new_tuple = self.tuple(new_right, Some(right_rest), rhs_span);

                            queue.push_back((left_splice, new_tuple));

                            continue;
                        }
                    }
                }
                (
                    &Ref {
                        read: lhs_read,
                        write: lhs_write,
                        span: _,
                    },
                    &Ref {
                        read: rhs_read,
                        write: rhs_write,
                        span: _,
                    },
                ) => {
                    if lhs_read.is_none() && lhs_write.is_none() {
                        diagnostics.add(lhs_span, "Reference is not readable or writable");
                        continue;
                    }
                    if let Some(rhs_read) = rhs_read {
                        if let Some(lhs_read) = lhs_read {
                            queue.push_back((lhs_read, rhs_read));
                        } else {
                            diagnostics.add(lhs_span, "Reference is not readable");
                        }
                    }
                    if let Some(rhs_write) = rhs_write {
                        if let Some(lhs_write) = lhs_write {
                            queue.push_back((lhs_write, rhs_write));
                        } else {
                            diagnostics.add(rhs_span, "Reference is not writable");
                        }
                    }
                    continue;
                }
                (
                    &Variable {
                        id: lhs_var_id,
                        span: _,
                    },
                    _,
                ) if rhs_id.level(self) <= lhs_id.level(self) => {
                    tracing::trace!(
                        "RHS LVL: {} <= LHS LVL: {}",
                        rhs_id.level(self),
                        lhs_id.level(self),
                    );

                    let lhs_vars = &mut self.vars[lhs_var_id.0];
                    tracing::trace!("Pushing {} to upper bounds of LHS {}", rhs_id.0, lhs_id.0);
                    lhs_vars.upper_bounds.push(rhs_id);
                    lhs_vars.lower_bounds.iter().copied().for_each(|lb| {
                        tracing::trace!(
                            "Constraint by extension of lower bound: {} :< {}",
                            lb.0,
                            rhs_id.0
                        );
                        queue.push_back((lb, rhs_id));
                    });
                    continue;
                }
                (
                    _,
                    &Variable {
                        id: rhs_var_id,
                        span: _,
                    },
                ) if lhs_id.level(self) <= rhs_id.level(self) => {
                    tracing::trace!(
                        "LHS LVL: {} <= RHS LVL: {}",
                        lhs_id.level(self),
                        rhs_id.level(self)
                    );
                    let rhs_vars = &mut self.vars[rhs_var_id.0];
                    tracing::trace!("Pushing {} to lower bounds of RHS: {}", lhs_id.0, rhs_id.0);
                    rhs_vars.lower_bounds.push(lhs_id);
                    rhs_vars.upper_bounds.iter().copied().for_each(|ub| {
                        tracing::trace!(
                            "Constraint by extension of upper bound: {} :< {}",
                            lhs_id.0,
                            ub.0,
                        );
                        queue.push_back((lhs_id, ub));
                    });
                    continue;
                }
                (&Variable { .. }, _) => {
                    tracing::trace!("LHS VAR, extruding RHS-");
                    let rhs_id = self.extrude(rhs_id, Polarity::Negative, lhs_id.level(self));
                    tracing::trace!("Extruded RHS ID: {}", rhs_id.0);
                    queue.push_back((lhs_id, rhs_id));
                    continue;
                }
                (_, &Variable { .. }) => {
                    tracing::trace!("RHS VAR, extruding LHS+");
                    let lhs_id = self.extrude(lhs_id, Polarity::Positive, rhs_id.level(self));
                    tracing::trace!("Extruded LHS ID: {}", lhs_id.0);
                    queue.push_back((lhs_id, rhs_id));
                    continue;
                }

                _ => (),
            }

            tracing::error!(
                "Type mismatch {lhs} ({}) <!: {rhs} ({})",
                lhs_id.0,
                rhs_id.0
            );

            diagnostics
                .add(lhs_span, "Type mismatch")
                .add_extra(format!("Expected {rhs}"), Some(rhs_span))
                .add_extra(format!("But found {lhs}"), Some(lhs_span));
        }
    }
}
