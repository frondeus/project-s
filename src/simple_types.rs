#![allow(dead_code, clippy::unnecessary_to_owned)]
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use itertools::Itertools;
use levels::WithLevel;
use tree_sitter::Range;

use crate::{
    ast::{ASTS, SExp, SExpId},
    diagnostics::{Diagnostics, SExpDiag as _},
    patterns::Pattern,
    source::{Sources, Span},
};

mod coalesce;
mod constrain;
mod constructors;
mod debug;
mod extrude;
mod format;
mod freshen_above;
mod levels;
mod type_term;
mod utils;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InferedTypeId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VarId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(usize);

#[derive(Debug)]
pub enum Type {
    /// ⊤ - Any type
    Top,
    /// ⊥ - Never type
    Bottom,
    /// v - | type
    Union {
        items: Vec<TypeId>,
    },
    /// ∧ - & type
    Intersection {
        items: Vec<TypeId>,
    },
    /// a -> b
    Function {
        lhs: TypeId,
        rhs: TypeId,
    },
    /// { :foo type }
    Record {
        fields: Vec<(String, TypeId)>,
    },
    /// type as 'a
    Recursive {
        name: String,
        body: TypeId,
    },
    /// 'a
    Variable {
        name: String,
    },
    /// 5
    Literal {
        value: Literal,
    },
    /// number
    Primitive {
        name: String,
    },
    /// The same as Never type
    Error,
    /// function application, tuple indexing, record selection
    Applicative {
        arg: TypeId,
        ret: TypeId,
        first_arg: Option<TypeId>,
    },
    Tuple {
        items: Vec<TypeId>,
    },
    List {
        item: TypeId,
    },
    Ref {
        write: Option<TypeId>,
        read: Option<TypeId>,
    },
    Module {
        members: Vec<(String, TypeScheme)>,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum TypeScheme {
    Monomorphic(TypeId),
    Polymorphic(PolymorphicType),
}
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct PolymorphicType {
    level: usize,
    body: TypeId,
}

pub enum InferedType {
    Error {
        span: Span,
    },
    Variable {
        id: VarId,
        span: Span,
    },
    Primitive {
        name: String,
        span: Span,
    },
    Literal {
        value: Literal,
        span: Span,
    },
    Function {
        lhs: InferedTypeId,
        rhs: InferedTypeId,
        span: Span,
    },
    /// A type that can apply arguments to.
    /// Function, Record (field selection), Tuple
    // Array, List
    Applicative {
        arg: InferedTypeId,
        ret: InferedTypeId,
        first_arg: Option<InferedTypeId>,
        span: Span,
    },
    Tuple {
        items: Vec<InferedTypeId>,
        span: Span,
    },
    Record {
        fields: Vec<(String, InferedTypeId)>,
        proto: Option<InferedTypeId>,
        span: Span,
    },
    List {
        item: InferedTypeId,
        span: Span,
    },
    Ref {
        write: Option<InferedTypeId>,
        read: Option<InferedTypeId>,
        span: Span,
    },
    Module {
        members: BTreeMap<String, InferedTypeScheme>,
        span: Span,
    },
}

impl std::fmt::Debug for InferedType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error { span: _ } => f.debug_struct("Error").finish(),
            Self::Variable { id, span: _ } => f.debug_struct("Variable").field("id", id).finish(),
            Self::Primitive { name, span: _ } => {
                f.debug_struct("Primitive").field("name", name).finish()
            }
            Self::Literal { value, span: _ } => {
                f.debug_struct("Literal").field("value", value).finish()
            }
            Self::Function { lhs, rhs, span: _ } => f
                .debug_struct("Function")
                .field("lhs", lhs)
                .field("rhs", rhs)
                .finish(),
            Self::Applicative {
                arg,
                ret,
                first_arg,
                span: _,
            } => f
                .debug_struct("Applicative")
                .field("arg", arg)
                .field("ret", ret)
                .field("first_arg", first_arg)
                .finish(),
            Self::Tuple { items, span: _ } => {
                f.debug_struct("Tuple").field("items", items).finish()
            }
            Self::Record {
                fields,
                proto,
                span: _,
            } => f
                .debug_struct("Record")
                .field("fields", fields)
                .field("proto", proto)
                .finish(),
            Self::List { item, span: _ } => f.debug_struct("List").field("item", item).finish(),
            Self::Ref {
                write,
                read,
                span: _,
            } => f
                .debug_struct("Ref")
                .field("write", write)
                .field("read", read)
                .finish(),
            Self::Module { members, span: _ } => {
                f.debug_struct("Module").field("members", members).finish()
            }
        }
    }
}

impl InferedType {
    pub fn span(&self) -> Span {
        match *self {
            InferedType::Error { span } => span,
            InferedType::Variable { span, .. } => span,
            InferedType::Primitive { span, .. } => span,
            InferedType::Literal { span, .. } => span,
            InferedType::Function { span, .. } => span,
            InferedType::Record { span, .. } => span,
            InferedType::Tuple { span, .. } => span,
            InferedType::Applicative { span, .. } => span,
            InferedType::List { span, .. } => span,
            InferedType::Ref { span, .. } => span,
            InferedType::Module { span, .. } => span,
        }
    }

    pub fn as_keyword_literal(&self) -> Option<&str> {
        match self {
            InferedType::Literal {
                value: Literal::Keyword(name),
                ..
            } => Some(name),
            _ => None,
        }
    }

    pub fn ids(&self) -> impl Iterator<Item = InferedTypeId> {
        let mut ids = vec![];

        match self {
            InferedType::Error { .. } => (),
            InferedType::Variable { .. } => {}
            InferedType::Primitive { .. } => (),
            InferedType::Literal { .. } => (),
            InferedType::Function { lhs, rhs, span: _ } => {
                ids.push(*lhs);
                ids.push(*rhs);
            }
            InferedType::Applicative {
                arg,
                ret,
                first_arg: _,
                span: _,
            } => {
                ids.push(*arg);
                ids.push(*ret);
            }
            InferedType::Tuple { items, span: _ } => {
                ids.extend(items.iter().copied());
            }
            InferedType::Record {
                fields,
                proto,
                span: _,
            } => {
                for (_, field) in fields {
                    ids.push(*field);
                }
                ids.extend(proto);
            }
            InferedType::List { item, span: _ } => {
                ids.push(*item);
            }
            InferedType::Ref {
                write,
                read,
                span: _,
            } => {
                ids.extend(*write);
                ids.extend(*read);
            }
            InferedType::Module { members, span: _ } => {
                for member in members.values() {
                    if let InferedTypeScheme::Monomorphic(d) = member {
                        ids.push(*d);
                    }
                }
            }
        }

        ids.into_iter()
    }
}

impl std::fmt::Display for InferedType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InferedType::Error { .. } => write!(f, "error"),
            InferedType::Variable { .. } => write!(f, "variable"),
            InferedType::Primitive { name, .. } => write!(f, "{name}"),
            InferedType::Literal { value, .. } => write!(f, "{value}"),
            InferedType::Function { .. } => write!(f, "function"),
            InferedType::Record { .. } => write!(f, "record"),
            InferedType::Tuple { .. } => write!(f, "tuple"),
            InferedType::Applicative { .. } => write!(f, "applicative"),
            InferedType::List { .. } => write!(f, "list"),
            InferedType::Ref { .. } => write!(f, "ref"),
            InferedType::Module { .. } => write!(f, "module"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Bool(bool),
    Number(f64),
    String(String),
    Keyword(String),
}
type LitValue = Literal;

impl std::fmt::Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Literal::Bool(value) => write!(f, "{value}"),
            Literal::Number(value) => write!(f, "{value}"),
            Literal::String(value) => write!(f, "\"{value}\""),
            Literal::Keyword(value) => write!(f, ":{value}"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Polarity {
    Positive,
    Negative,
}

impl Polarity {
    pub fn negate(self) -> Self {
        match self {
            Polarity::Positive => Polarity::Negative,
            Polarity::Negative => Polarity::Positive,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Copy, Hash, Eq)]
pub struct PolarVariable {
    polarity: Polarity,
    id: VarId,
}

#[derive(Default)]
pub struct TypeEnv {
    infered: Vec<InferedType>,
    vars: Vec<VarState>,
    sexps: HashMap<SExpId, InferedTypeId>,
    envs: Envs,
    constraint_cache: HashSet<(InferedTypeId, InferedTypeId)>,
    constraints: Vec<(InferedTypeId, InferedTypeId)>,
    types: Vec<Type>,
}

#[derive(Clone, Debug)]
pub(crate) struct VarState {
    level: usize,
    lower_bounds: Vec<InferedTypeId>,
    upper_bounds: Vec<InferedTypeId>,
}

#[derive(Debug, Clone, Copy)]
pub enum InferedTypeScheme {
    Monomorphic(InferedTypeId),
    Polymorphic(InferedPolymorphicType),
}

#[derive(Clone, Copy, Debug)]

enum TypeSchemeKind {
    Monomorphic,
    Polymorphic { level: usize },
}

impl WithLevel for InferedTypeScheme {
    fn level(&self, type_env: &TypeEnv) -> usize {
        match self {
            InferedTypeScheme::Monomorphic(id) => id.level(type_env),
            InferedTypeScheme::Polymorphic(poly) => poly.level,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct InferedPolymorphicType {
    level: usize,
    body: InferedTypeId,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self::default()
    }

    pub const NUMBER: &str = "number";
    pub const STRING: &str = "string";
    pub const BOOLEAN: &str = "bool";
    pub const KEYWORD: &str = "keyword";

    pub fn with_prelude(mut self, sources: &mut Sources) -> Self {
        let builtin = sources.add("<builtin>", "");
        let span = Span::new_empty(builtin);
        {
            // "-"
            let lhs = self.primitive(Self::NUMBER, span);
            let rhs = self.primitive(Self::NUMBER, span);
            let args = self.tuple(vec![lhs, rhs], span);
            let rhs = self.primitive(Self::NUMBER, span);
            let ty = self.function(args, rhs, span);
            self.envs.set("-", InferedTypeScheme::Monomorphic(ty));
        }
        {
            // "="
            let lhs = self.fresh_var(span, 1);
            let rhs = self.fresh_var(span, 1);
            let args = self.tuple(vec![lhs, rhs], span);
            let rhs = self.primitive(Self::BOOLEAN, span);
            let ty = self.function(args, rhs, span);
            self.envs.set(
                "=",
                InferedTypeScheme::Polymorphic(InferedPolymorphicType { level: 1, body: ty }),
            );
        }

        self
    }
}

#[derive(Default, Debug)]
struct Env {
    vars: BTreeMap<String, InferedTypeScheme>,
}

#[derive(Debug)]
struct Envs {
    envs: Vec<Env>,
}

impl Default for Envs {
    fn default() -> Self {
        Self::new()
    }
}

pub struct EnvSavePoint(usize);

impl Envs {
    pub fn new() -> Self {
        Self {
            envs: vec![Env::default()],
        }
    }

    pub fn set(&mut self, name: &str, value: InferedTypeScheme) {
        self.envs
            .last_mut()
            .unwrap()
            .vars
            .insert(name.to_string(), value);
    }

    pub fn get(&self, name: &str) -> Option<&InferedTypeScheme> {
        self.envs.iter().rev().find_map(|env| env.vars.get(name))
    }

    pub fn push(&mut self) -> EnvSavePoint {
        let point = self.save();
        self.envs.push(Env::default());
        point
    }

    pub fn save(&mut self) -> EnvSavePoint {
        EnvSavePoint(self.envs.len())
    }

    pub fn pop(&mut self) -> Option<BTreeMap<String, InferedTypeScheme>> {
        self.envs.pop().map(|env| env.vars)
    }

    pub fn restore(&mut self, point: EnvSavePoint) {
        self.envs.truncate(point.0);
    }

    // pub fn with<T>(&mut self, f: impl FnOnce() -> T) -> T {
    //     self.push();
    //     let result = f();
    //     self.pop();
    //     result
    // }
}

#[cfg(test)]
mod tests {
    use tracing_subscriber::{Layer, layer::SubscriberExt};

    use crate::{
        level_from_args,
        modules::{MemoryModules, ModuleProvider},
        process_ast,
        s_std::prelude,
    };

    use super::*;

    #[test]
    fn simple_type_traces() -> test_runner::Result {
        use std::io::Read;
        unsafe { std::env::set_var("NO_COLOR", "1") }
        test_runner::test_snapshots(
            "docs/",
            &["s", ""],
            "simple-type-traces",
            |input, _deps, args| {
                let mut reader = tempfile::NamedTempFile::new().unwrap();

                let writer = reader.reopen().unwrap();
                {
                    let level = level_from_args(args);
                    let (writer, _guard) = tracing_appender::non_blocking(writer);

                    let file_layer = tracing_subscriber::fmt::Layer::new()
                        // .compact()
                        .with_file(args.contains("file"))
                        .with_line_number(args.contains("line"))
                        .with_writer(writer)
                        .without_time()
                        .with_ansi(false);

                    let console_layer = tracing_subscriber::fmt::Layer::new()
                        // .compact()
                        .with_file(args.contains("file"))
                        .with_line_number(args.contains("line"))
                        .with_ansi(true);

                    let subscriber = tracing_subscriber::registry()
                        .with(console_layer.with_filter(level))
                        .with(file_layer.with_filter(level));

                    tracing::subscriber::with_default(subscriber, move || {
                        let mut asts = ASTS::new();
                        let (mut modules, source_id) = MemoryModules::from_deps(input, _deps);
                        let ast = asts
                            .parse(source_id, modules.sources().get(source_id))
                            .expect("Failed to parse");

                        let root = ast.root_id().unwrap();

                        let mut env = TypeEnv::new().with_prelude(modules.sources_mut());

                        // let mut env = TypeEnv::new(modules).with_prelude();

                        let prelude = prelude();
                        let (root, mut diagnostics) = process_ast(&mut asts, root, &[prelude]);
                        let infered = env.type_term(&mut asts, root, &mut diagnostics, 0);

                        env.coalesce(infered);
                    });
                }

                let mut buf = String::new();
                reader.read_to_string(&mut buf).unwrap();
                buf
                // env.to_string(infered)
            },
        )
    }

    #[test]
    fn simple_type() -> test_runner::Result {
        unsafe { std::env::set_var("NO_COLOR", "1") }
        test_runner::test_snapshots("docs/", &["s", ""], "simple-type", |input, _deps, _args| {
            let mut asts = ASTS::new();
            let (mut modules, source_id) = MemoryModules::from_deps(input, _deps);
            let ast = asts
                .parse(source_id, modules.sources().get(source_id))
                .expect("Failed to parse");

            let root = ast.root_id().unwrap();

            let mut env = TypeEnv::new().with_prelude(modules.sources_mut());
            // let mut env = TypeEnv::new(modules).with_prelude();

            let prelude = prelude();
            let (root, mut diagnostics) = process_ast(&mut asts, root, &[prelude]);
            let infered = env.type_term(&mut asts, root, &mut diagnostics, 0);

            // let infered = env.check(&mut asts, root, &mut diagnostics);
            if diagnostics.has_errors() {
                // let modules = env.finish();
                return diagnostics.pretty_print(modules.sources());
            }

            let ty = env.coalesce(infered);
            let mut out = String::new();
            env.fmt(ty, &mut out).expect("Failed to format type");
            out

            // env.to_string(infered)
        })
    }

    #[test]
    fn simple_type_dot() -> test_runner::Result {
        unsafe { std::env::set_var("NO_COLOR", "1") }
        test_runner::test_snapshots(
            "docs/",
            &["s", ""],
            "simple-type-dot",
            |input, _deps, _args| {
                let mut asts = ASTS::new();
                let (mut modules, source_id) = MemoryModules::from_deps(input, _deps);
                let ast = asts
                    .parse(source_id, modules.sources().get(source_id))
                    .expect("Failed to parse");

                let root = ast.root_id().unwrap();

                let mut env = TypeEnv::new().with_prelude(modules.sources_mut());
                // let mut env = TypeEnv::new(modules).with_prelude();

                let prelude = prelude();
                let (root, mut diagnostics) = process_ast(&mut asts, root, &[prelude]);
                let infered = env.type_term(&mut asts, root, &mut diagnostics, 0);

                env.debug_dot(&asts, infered)
            },
        )
    }
}
