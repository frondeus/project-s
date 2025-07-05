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

    pub(crate) fn fresh_var(&mut self, span: Span, level: usize) -> InferedTypeId {
        let id = self.vars.len();
        self.vars.push(VarState {
            lower_bounds: Vec::new(),
            upper_bounds: Vec::new(),
            level,
        });
        self.add_infered(InferedType::Variable {
            id: VarId(id),
            span,
        })
    }

    pub(crate) fn add_infered(&mut self, infered: InferedType) -> InferedTypeId {
        let id = self.infered.len();
        self.infered.push(infered);
        InferedTypeId(id)
    }

    pub(crate) fn add_type(&mut self, ty: Type) -> TypeId {
        let id = self.types.len();
        self.types.push(ty);
        TypeId(id)
    }
}
