use super::*;

impl TypeEnv {
    pub(crate) fn error(&mut self, span: Span) -> InferedTypeId {
        self.add_infered(InferedType::Error { span })
    }

    pub(crate) fn unit(&mut self, span: Span) -> InferedTypeId {
        self.add_infered(InferedType::Tuple {
            items: vec![],
            span,
        })
    }

    pub(crate) fn literal(&mut self, value: Literal, span: Span) -> InferedTypeId {
        self.add_infered(InferedType::Literal { value, span })
    }

    pub(crate) fn primitive(&mut self, primitive: impl ToString, span: Span) -> InferedTypeId {
        self.add_infered(InferedType::Primitive {
            name: primitive.to_string(),
            span,
        })
    }

    pub(crate) fn reference(
        &mut self,
        read: Option<InferedTypeId>,
        write: Option<InferedTypeId>,
        span: Span,
    ) -> InferedTypeId {
        self.add_infered(InferedType::Ref { read, write, span })
    }

    pub(crate) fn tuple(&mut self, items: Vec<InferedTypeId>, span: Span) -> InferedTypeId {
        self.add_infered(InferedType::Tuple { items, span })
    }

    pub(crate) fn list(&mut self, item: InferedTypeId, span: Span) -> InferedTypeId {
        self.add_infered(InferedType::List { item, span })
    }

    pub(crate) fn function(
        &mut self,
        lhs: InferedTypeId,
        rhs: InferedTypeId,
        span: Span,
    ) -> InferedTypeId {
        self.add_infered(InferedType::Function { lhs, rhs, span })
    }

    pub(crate) fn applicative(
        &mut self,
        arg: InferedTypeId,
        ret: InferedTypeId,
        first_arg: Option<InferedTypeId>,
        span: Span,
    ) -> InferedTypeId {
        self.add_infered(InferedType::Applicative {
            arg,
            ret,
            first_arg,
            span,
        })
    }

    pub(crate) fn record(
        &mut self,
        fields: Vec<(String, InferedTypeId)>,
        proto: Option<InferedTypeId>,
        span: Span,
    ) -> InferedTypeId {
        self.add_infered(InferedType::Record {
            fields,
            proto,
            span,
        })
    }
}
