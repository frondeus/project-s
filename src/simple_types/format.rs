use super::*;

impl TypeEnv {
    pub fn fmt(&self, ty: TypeId, buf: &mut String) -> Result<(), std::fmt::Error> {
        use std::fmt::Write;
        let ty = &self.types[ty.0];
        match ty {
            Type::Top => write!(buf, "⊤"),
            Type::Bottom => write!(buf, "⊥"),
            Type::Union { items } => {
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(buf, " ∨ ")?;
                    }
                    self.fmt(*item, buf)?;
                }
                Ok(())
            }
            Type::Intersection { items } => {
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(buf, " ∧ ")?;
                    }
                    self.fmt(*item, buf)?;
                }
                Ok(())
            }
            &Type::Function { lhs, rhs } => {
                self.fmt(lhs, buf)?;
                write!(buf, " → ")?;
                self.fmt(rhs, buf)?;
                Ok(())
            }
            Type::Record { fields } => {
                write!(buf, "{{")?;
                for (i, (name, ty)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(buf, ", ")?;
                    }
                    write!(buf, "{}: ", name)?;
                    self.fmt(*ty, buf)?;
                }
                write!(buf, "}}")?;
                Ok(())
            }
            Type::Recursive { name, body } => {
                self.fmt(*body, buf)?;
                write!(buf, " as ")?;
                write!(buf, "{}", name)?;
                Ok(())
            }
            Type::Variable { name } => write!(buf, "{}", name),
            Type::Literal { value } => write!(buf, "{}", value),
            Type::Primitive { name } => write!(buf, "{}", name),
            Type::Error => write!(buf, "error"),
            Type::Applicative {
                arg,
                ret,
                first_arg: _,
            } => {
                self.fmt(*arg, buf)?;
                write!(buf, " -?-> ")?;
                self.fmt(*ret, buf)?;
                Ok(())
            }
            Type::Tuple { items } => {
                write!(buf, "(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(buf, ", ")?;
                    }
                    self.fmt(*item, buf)?;
                }
                write!(buf, ")")?;
                Ok(())
            }
            Type::List { item } => {
                write!(buf, "[")?;
                self.fmt(*item, buf)?;
                write!(buf, "]")?;
                Ok(())
            }
            Type::Ref {
                read: Some(read),
                write: Some(write),
            } if read == write => {
                write!(buf, "refmut ")?;
                self.fmt(*read, buf)
            }
            Type::Ref {
                read: Some(read),
                write: Some(write),
            } => {
                write!(buf, "ref ")?;
                self.fmt(*read, buf)?;
                write!(buf, " mut ")?;
                self.fmt(*write, buf)
            }
            Type::Ref {
                read: Some(read),
                write: None,
            } => {
                write!(buf, "ref ")?;
                self.fmt(*read, buf)
            }
            Type::Ref {
                read: None,
                write: Some(write),
            } => {
                write!(buf, "mut ")?;
                self.fmt(*write, buf)
            }
            Type::Ref {
                read: None,
                write: None,
            } => {
                write!(buf, "<invalid ref>")
            }
        }
    }
}
