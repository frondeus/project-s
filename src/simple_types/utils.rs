use super::*;

pub(crate) fn variable_letters(mut i: usize) -> String {
    const LETTERS: &[char] = &[
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
        's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    ];
    let letter = LETTERS[i % LETTERS.len()];
    let mut result = format!("'{letter}");
    if i < LETTERS.len() {
        return result;
    }
    let mut num = 1;
    while i > 0 {
        i = match i.checked_sub(LETTERS.len()) {
            Some(i) => i,
            None => break,
        };
        num += 1;
    }
    result.push_str(&num.to_string());
    result
}

impl TypeEnv {
    pub fn top_env(&self) -> &Env {
        self.envs.envs.last().unwrap()
    }
    pub(crate) fn span_of(sexp: SExpId, asts: &ASTS) -> Span {
        let sexp = asts.get(sexp);
        sexp.span
    }

    pub(crate) fn is_symbols(asts: &ASTS, sexp: SExpId, names: &[&str]) -> bool {
        let sexp = asts.get(sexp);
        match &**sexp {
            SExp::Symbol(symbol) => names.contains(&symbol.as_str()),
            _ => false,
        }
    }

    pub(crate) fn is_symbol(asts: &ASTS, sexp: SExpId, name: &str) -> bool {
        let sexp = asts.get(sexp);
        match &**sexp {
            SExp::Symbol(symbol) => symbol == name,
            _ => false,
        }
    }

    pub(crate) fn as_keyword(asts: &ASTS, sexp: SExpId) -> Option<&str> {
        let sexp = asts.get(sexp);
        match &**sexp {
            SExp::Keyword(s) => Some(s),
            _ => None,
        }
    }

    pub(crate) fn get(&self, id: InferedTypeId) -> &InferedType {
        &self.infered[id.0]
    }

    pub(crate) fn vars_of(&mut self, id: InferedTypeId) -> &mut VarState {
        let InferedType::Variable { id: var, .. } = self.infered[id.0] else {
            panic!("Expected variable type");
        };
        &mut self.vars[var.0]
    }

    pub(crate) fn find_in_relatives_inner<'a, O, F>(
        &'a self,
        id: InferedTypeId,
        polarity: Polarity,
        filter: &F,
    ) -> Option<O>
    where
        F: Fn(&'a InferedType) -> Option<O>,
        O: 'a,
    {
        match self.get(id) {
            InferedType::Variable {
                id: var_id,
                span: _,
            } => self
                .relatives(id, *var_id, polarity)
                .find_map(move |bound| self.find_in_relatives_inner(bound, polarity, filter)),
            t => filter(t),
        }
    }
    pub(crate) fn find_in_relatives<'a, O, F>(
        &'a self,
        id: InferedTypeId,
        polarity: Polarity,
        filter: F,
    ) -> Option<O>
    where
        F: Fn(&'a InferedType) -> Option<O>,
        O: 'a,
    {
        self.find_in_relatives_inner(id, polarity, &filter)
    }

    //-----
    pub(crate) fn find_in_predecessors<'a, O, F>(
        &'a self,
        id: InferedTypeId,
        filter: F,
    ) -> Option<O>
    where
        F: Fn(&'a InferedType) -> Option<O>,
        O: 'a,
    {
        self.find_in_relatives(id, Polarity::Negative, &filter)
    }

    pub(crate) fn find_in_successors<'a, O, F>(&'a self, id: InferedTypeId, filter: F) -> Option<O>
    where
        F: Fn(&'a InferedType) -> Option<O>,
        O: 'a,
    {
        self.find_in_relatives(id, Polarity::Positive, &filter)
    }

    pub(crate) fn find_in_all_relatives<'a, O, F>(
        &'a self,
        id: InferedTypeId,
        filter: F,
    ) -> Option<O>
    where
        F: Fn(&'a InferedType) -> Option<O>,
        O: 'a,
    {
        self.find_in_relatives(id, Polarity::Positive, &filter)
            .or_else(|| self.find_in_relatives(id, Polarity::Negative, &filter))
    }

    pub(crate) fn fresh_var(&mut self, span: Span, level: usize) -> InferedTypeId {
        let id = self.vars.len();
        self.vars.push(VarState {
            lower_bounds: Vec::new(),
            upper_bounds: Vec::new(),
            level,
        });
        tracing::trace!("Adding fresh variable {id} with level {level}");
        self.add_infered(InferedType::Variable {
            id: VarId(id),
            span,
        })
    }

    pub(crate) fn add_sexp(
        &mut self,
        asts: &ASTS,
        sexp: SExpId,
        infered: InferedTypeId,
    ) -> InferedTypeId {
        tracing::trace!("`{}` with type N{infered}", asts.fmt(sexp));
        self.sexps.insert(sexp, infered);
        infered
    }

    pub(crate) fn add_infered(&mut self, infered: InferedType) -> InferedTypeId {
        let id = self.infered.len();
        tracing::trace!("Adding infered {infered} as N{id}: {infered:?}");
        self.infered.push(infered);
        InferedTypeId(id)
    }

    pub(crate) fn add_type(&mut self, ty: Type) -> TypeId {
        let id = self.types.len();
        tracing::trace!("Adding {ty:?} as {id}");
        self.types.push(ty);
        TypeId(id)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (InferedTypeId, &InferedType)> {
        self.infered
            .iter()
            .enumerate()
            .map(|(idx, ty)| (InferedTypeId(idx), ty))
    }

    pub(crate) fn relatives(
        &self,
        id: InferedTypeId,
        var: VarId,
        polarity: Polarity,
    ) -> impl Iterator<Item = InferedTypeId> {
        let direct = self.vars[var.0].following_bounds(polarity).iter().copied();

        let indirect = self
            .vars
            .iter()
            .enumerate()
            .filter_map(|(from_var, var)| {
                var.preceding_bounds(polarity)
                    .contains(&id)
                    .then_some(VarId(from_var))
            })
            .collect::<HashSet<_>>();

        let indirect = self.iter().filter_map(move |(id, ty)| match ty {
            InferedType::Variable {
                id: var_id,
                span: _,
            } => {
                if indirect.contains(var_id) {
                    Some(id)
                } else {
                    None
                }
            }
            _ => None,
        });

        direct.chain(indirect)
    }

    pub(crate) fn predecessors(
        &self,
        id: InferedTypeId,
        var: VarId,
    ) -> impl Iterator<Item = InferedTypeId> {
        self.relatives(id, var, Polarity::Negative)
    }

    pub(crate) fn successors(
        &self,
        id: InferedTypeId,
        var: VarId,
    ) -> impl Iterator<Item = InferedTypeId> {
        self.relatives(id, var, Polarity::Positive)
    }
}
