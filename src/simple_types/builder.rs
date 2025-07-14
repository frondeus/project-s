use std::{cell::RefCell, collections::hash_map::Entry, rc::Rc};

use tree_sitter::Point;

use super::*;
use crate::source::SourceId;

#[derive(Default, Clone, Copy)]
pub struct OffsetPoint {
    pub point: Point,
    pub offset: usize,
}

pub struct SourceBuilder {
    cursor: OffsetPoint,
    source: String,
    source_id: SourceId,
}

impl SourceBuilder {
    pub fn new(source_id: SourceId) -> Self {
        SourceBuilder {
            cursor: Default::default(),
            source: String::new(),
            source_id,
        }
    }
    pub fn finalize(self) -> String {
        self.source
    }
    pub fn point(&self) -> OffsetPoint {
        self.cursor
    }
    fn range(&self, start: OffsetPoint, end: OffsetPoint) -> Range {
        Range {
            start_point: start.point,
            end_point: end.point,
            start_byte: start.offset,
            end_byte: end.offset,
        }
    }
    pub fn span(&self, start: OffsetPoint, end: OffsetPoint) -> Span {
        Span {
            range: self.range(start, end),
            source_id: self.source_id,
        }
    }
    pub fn append(&mut self, s: &str) -> Span {
        let start = self.cursor;
        let start_byte = self.cursor.offset;
        // let start_byte = self.offset;
        let end_byte = start_byte + s.len();
        let end = start.point.column + s.len();
        let end = Point {
            row: start.point.row,
            column: end,
        };
        self.source.push_str(s);
        self.cursor.point.column += s.len();
        self.cursor.offset += s.len();
        Span {
            range: Range {
                start_point: start.point,
                end_point: end,
                start_byte,
                end_byte,
            },
            source_id: self.source_id,
        }
    }
    pub fn new_line(&mut self) {
        self.source += "\n";
        self.cursor.point.row += 1;
        self.cursor.offset += 1;
    }
}

pub trait TypeBuilder {
    fn build(self, env: &mut TypeEnv, source: &mut SourceBuilder) -> InferedTypeId;
}

impl<F> TypeBuilder for F
where
    F: FnOnce(&mut TypeEnv, &mut SourceBuilder) -> InferedType,
{
    fn build(self, env: &mut TypeEnv, source: &mut SourceBuilder) -> InferedTypeId {
        let ty = self(env, source);
        env.add_infered(ty)
    }
}

struct IdFn<F> {
    f: F,
}

impl<F> TypeBuilder for IdFn<F>
where
    F: FnOnce(&mut TypeEnv, &mut SourceBuilder) -> InferedTypeId,
{
    fn build(self, env: &mut TypeEnv, source: &mut SourceBuilder) -> InferedTypeId {
        (self.f)(env, source)
    }
}

pub fn primitive(s: &str) -> impl TypeBuilder {
    move |_env: &mut TypeEnv, source: &mut SourceBuilder| {
        let span = source.append(s);
        InferedType::Primitive {
            name: s.into(),
            span,
        }
    }
}

pub fn number() -> impl TypeBuilder {
    primitive(TypeEnv::NUMBER)
}

pub fn boolean() -> impl TypeBuilder {
    primitive(TypeEnv::BOOLEAN)
}

pub fn id_fn(
    f: impl FnOnce(&mut TypeEnv, &mut SourceBuilder) -> InferedTypeId,
) -> impl TypeBuilder {
    IdFn { f }
}

#[derive(Default)]
pub struct Vars {
    vars: Rc<RefCell<HashMap<&'static str, InferedTypeId>>>,
}

impl Vars {
    pub fn var(&self, name: &'static str, level: usize) -> impl TypeBuilder + 'static {
        let vars = Rc::clone(&self.vars);
        id_fn(move |env, source| match vars.borrow_mut().entry(name) {
            Entry::Occupied(occupied_entry) => {
                source.append(name);
                *occupied_entry.into_mut()
            }
            Entry::Vacant(vacant_entry) => {
                let span = source.append(name);
                *vacant_entry.insert(env.fresh_var(span, level))
            }
        })
    }
}

pub fn var(name: &str, lvl: usize) -> impl TypeBuilder {
    id_fn(move |env: &mut TypeEnv, source: &mut SourceBuilder| {
        let span = source.append(name);
        env.fresh_var(span, lvl)
    })
}

pub fn function(args: impl TypeBuilder, ret: impl TypeBuilder) -> impl TypeBuilder {
    move |env: &mut TypeEnv, source: &mut SourceBuilder| {
        let from = source.point();
        let lhs = args.build(env, source);
        source.append(" -> ");
        let rhs = ret.build(env, source);
        let to = source.point();
        let span = source.span(from, to);
        InferedType::Function { lhs, rhs, span }
    }
}

pub fn list(arg: impl TypeBuilder) -> impl TypeBuilder {
    move |env: &mut TypeEnv, source: &mut SourceBuilder| {
        let from = source.point();
        source.append("[");
        let item = arg.build(env, source);
        source.append("]");
        let to = source.point();
        let span = source.span(from, to);

        InferedType::List { item, span }
    }
}

macro_rules! build_tuple {
    ($($item:tt),*) => {
        impl<$($item: TypeBuilder),*> TypeBuilder for ($($item,)*) {
            #[allow(non_snake_case)]
            fn build(self, env: &mut TypeEnv, source: &mut SourceBuilder) -> InferedTypeId {
                let from = source.point();
                source.append("(");
                let ($($item,)*) = self;
                $(
                    let $item = $item.build(env, source);
                    source.append(", ");
                )*
                source.append(")");
                let to =source.point();
                let span = source.span(from, to);
                env.add_infered(InferedType::Tuple { items: vec![$($item),*], rest: None, span })
            }
        }
    }

}

build_tuple!(T1);
build_tuple!(T1, T2);
build_tuple!(T1, T2, T3);
