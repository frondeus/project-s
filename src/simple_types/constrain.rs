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
                (
                    &Variable {
                        id: lhs_var_id,
                        span: _,
                    },
                    _,
                ) => {
                    let lhs_lvl = self.vars[lhs_var_id.0].level;
                    let rhs_lvl = rhs_id.level(self);
                    tracing::trace!("Left LVL: {lhs_lvl} RIGHT LVL: {rhs_lvl}");
                    if rhs_lvl <= lhs_lvl {
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
                    } else {
                        let rhs_id = self.extrude(rhs_id, Polarity::Negative, lhs_lvl);
                        tracing::trace!("Extruded RHS ID: {}", rhs_id.0);
                        queue.push_back((lhs_id, rhs_id));
                        continue;
                    }
                }
                (
                    _,
                    &Variable {
                        id: rhs_var_id,
                        span: _,
                    },
                ) => {
                    let rhs_lvl = self.vars[rhs_var_id.0].level;
                    let lhs_lvl = lhs_id.level(self);

                    tracing::trace!("RHS VAR: {lhs_lvl} ? {rhs_lvl}");

                    if lhs_lvl <= rhs_lvl {
                        let rhs_vars = &mut self.vars[rhs_var_id.0];
                        tracing::trace!(
                            "Pushing {} to lower bounds of RHS: {}",
                            lhs_id.0,
                            rhs_id.0
                        );
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
                    } else {
                        let lhs_id = self.extrude(lhs_id, Polarity::Positive, rhs_lvl);
                        tracing::trace!("Extruded LHS ID: {}", lhs_id.0);
                        queue.push_back((lhs_id, rhs_id));
                        continue;
                    }
                }
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
                    Tuple {
                        items: left,
                        span: _,
                    },
                    Tuple {
                        items: right,
                        span: _,
                    },
                ) => {
                    if left.len() != right.len() {
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
                    } else {
                        for (l, r) in left.iter().copied().zip(right.iter().copied()) {
                            queue.push_back((l, r));
                        }
                        continue;
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
                _ => (),
            }

            diagnostics
                .add(lhs_span, "Type mismatch")
                .add_extra(format!("Expected {}", rhs), Some(rhs_span))
                .add_extra(format!("But found {}", lhs), Some(lhs_span));
        }
    }
}
