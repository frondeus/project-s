use crate::ast::SExpId;

use super::{Runtime, value::Value};

impl Runtime {
    pub(crate) fn make_list(&mut self, items: &[SExpId]) -> Value {
        let mut list = vec![];
        for item in items {
            list.push(self.eval(*item));
        }
        Value::List(list)
    }
}
