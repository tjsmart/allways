use std::collections::hash_set::IntoIter;
use std::collections::HashSet;
use std::str::FromStr;

use anyhow::Error;
use anyhow::Result;

use rustpython_parser::ast::Expression;
use rustpython_parser::ast::ExpressionType;
use rustpython_parser::ast::ImportSymbol;
use rustpython_parser::ast::Program;
use rustpython_parser::ast::Statement;
use rustpython_parser::ast::StatementType;
use rustpython_parser::ast::WithItem;
use rustpython_parser::parser::parse_program;

pub struct NameParser {
    names: HashSet<String>,
}

impl NameParser {
    fn new() -> Self {
        Self {
            names: HashSet::new(),
        }
    }
}

impl IntoIterator for NameParser {
    type Item = String;
    type IntoIter = IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.names.into_iter()
    }
}

impl FromStr for NameParser {
    type Err = Error;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let program = parse_program(src)?;
        Ok(program.into())
    }
}

impl From<Program> for NameParser {
    fn from(program: Program) -> Self {
        program.statements.into()
    }
}

impl From<Vec<Statement>> for NameParser {
    fn from(statements: Vec<Statement>) -> Self {
        let mut parser = NameParser::new();
        parser.add_statements(statements);
        parser
    }
}

impl NameParser {
    fn insert(&mut self, name: String) {
        self.names.insert(name);
    }

    fn remove(&mut self, name: &String) {
        self.names.remove(name);
    }

    fn insert_many(&mut self, names: impl Iterator<Item = String>) {
        self.names.extend(names);
    }

    fn remove_many(&mut self, names: impl Iterator<Item = String>) {
        for name in names {
            self.remove(&name);
        }
    }

    fn take_from(&mut self, other: Self) {
        self.insert_many(other.into_iter());
    }

    fn remove_from(&mut self, other: Self) {
        self.remove_many(other.into_iter());
    }
}

impl NameParser {
    fn add_statements(&mut self, statements: Vec<Statement>) {
        for statement in statements {
            self.add_statement(statement);
        }
    }

    fn add_statement(&mut self, statement: Statement) {
        match statement.node {
            StatementType::FunctionDef {
                is_async: _, name, ..
            }
            | StatementType::ClassDef { name, .. } => {
                self.insert(name);
            }
            StatementType::Delete { targets } => {
                self.remove_from(targets.into());
            }
            StatementType::Assign { targets, .. } => {
                self.take_from(targets.into());
            }
            StatementType::AugAssign { target, .. } | StatementType::AnnAssign { target, .. } => {
                self.take_from((*target).into());
            }
            StatementType::For {
                is_async: _,
                target,
                iter: _,
                body,
                orelse,
            } => {
                self.take_from((*target).into());
                self.add_statements(body);
                if let Some(body) = orelse {
                    self.add_statements(body);
                }
            }
            StatementType::While {
                test: target,
                body,
                orelse,
            }
            | StatementType::If {
                test: target,
                body,
                orelse,
            } => {
                if let ExpressionType::NamedExpression { left, .. } = target.node {
                    self.take_from((*left).into());
                }
                self.add_statements(body);
                if let Some(body) = orelse {
                    self.add_statements(body);
                }
            }
            StatementType::With {
                is_async: _,
                items,
                body,
            } => {
                self.take_from(items.into());
                self.add_statements(body);
            }
            StatementType::Try {
                body,
                handlers,
                orelse,
                finalbody,
            } => {
                self.add_statements(body);
                for handler in handlers {
                    self.add_statements(handler.body);
                }
                if let Some(body) = orelse {
                    self.add_statements(body);
                }
                if let Some(body) = finalbody {
                    self.add_statements(body);
                }
            }
            StatementType::Import { names: symbols }
            | StatementType::ImportFrom {
                level: _,
                module: _,
                names: symbols,
            } => self.take_from(symbols.into()),
            _ => {}
        }
    }
}

impl From<Vec<Expression>> for NameParser {
    fn from(expressions: Vec<Expression>) -> Self {
        let mut parser = NameParser::new();
        for expression in expressions {
            parser.take_from(expression.into());
        }
        parser
    }
}

impl From<Expression> for NameParser {
    fn from(expression: Expression) -> Self {
        match expression.node {
            ExpressionType::Identifier { name } => {
                let mut parser = NameParser::new();
                parser.insert(name);
                parser
            }
            ExpressionType::Tuple { elements } => NameParser::from(elements),
            _ => NameParser::new(),
        }
    }
}

impl From<Vec<WithItem>> for NameParser {
    fn from(items: Vec<WithItem>) -> Self {
        let mut parser = NameParser::new();
        for vars in items.into_iter().filter_map(|item| item.optional_vars) {
            parser.take_from(vars.into());
        }
        parser
    }
}

impl From<Vec<ImportSymbol>> for NameParser {
    fn from(symbols: Vec<ImportSymbol>) -> Self {
        let mut parser = NameParser::new();
        for symbol in symbols {
            let name = symbol.alias.unwrap_or(symbol.symbol);
            if name == "*" {
                // star imports can be ignored
                continue;
            }
            parser.insert(name);
        }
        parser
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Names = HashSet<String>;

    fn assert_src_parses_to_expected(src: &str, expected_names: Vec<&str>) {
        let parsed_names = src.parse::<NameParser>().unwrap().names;
        let expected_names = Names::from_iter(expected_names.into_iter().map(|s| s.to_string()));
        assert_eq!(parsed_names, expected_names);
    }

    #[test]
    fn basic_expressions() {
        let src = "
bar()
x
3 * y
";
        assert_src_parses_to_expected(src, vec![]);
    }

    #[test]
    fn basic_import() {
        let src = "
import sys
";
        assert_src_parses_to_expected(src, vec!["sys"]);
    }

    #[test]
    fn import_multi() {
        let src = "
import sys, os
";
        assert_src_parses_to_expected(src, vec!["sys", "os"]);
    }

    #[test]
    fn import_alias() {
        let src = "
import sys as foo
";
        assert_src_parses_to_expected(src, vec!["foo"]);
    }

    #[test]
    fn basic_from_import() {
        let src = "
from foo import bar
";
        assert_src_parses_to_expected(src, vec!["bar"]);
    }

    #[test]
    fn from_import_multi() {
        let src = "
from foo import (
    bar,
    baz,
)
";
        assert_src_parses_to_expected(src, vec!["bar", "baz"]);
    }

    #[test]
    fn from_import_alias() {
        let src = "
from foo import bar as baz
";
        assert_src_parses_to_expected(src, vec!["baz"]);
    }

    #[test]
    fn basic_function() {
        let src = "
def foo(x: int) -> str | None:
    z = 3
";
        assert_src_parses_to_expected(src, vec!["foo"]);
    }

    #[test]
    fn async_function() {
        let src = "
async def foo():
    ...
";
        assert_src_parses_to_expected(src, vec!["foo"]);
    }

    #[test]
    fn basic_class() {
        let src = "
class Foo(Bar, Baz):
    x = 10
    def bar(self, y: int) -> list[int]:
        ...
";
        assert_src_parses_to_expected(src, vec!["Foo"]);
    }

    #[test]
    fn basic_assignment() {
        let src = "
x = 1
";
        assert_src_parses_to_expected(src, vec!["x"]);
    }

    #[test]
    fn multi_target_assignment() {
        let src = "
foo = bar = baz = 1
";
        assert_src_parses_to_expected(src, vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn unpack_assignment() {
        let src = "
foo, bar = 1, 2
";
        assert_src_parses_to_expected(src, vec!["foo", "bar"]);
    }

    #[test]
    fn augmented_assignment() {
        let src = "
foo += 1
";
        assert_src_parses_to_expected(src, vec!["foo"]);
    }

    #[test]
    fn annotated_assignment() {
        let src = "
foo: int = 1
";
        assert_src_parses_to_expected(src, vec!["foo"]);
    }

    #[test]
    fn basic_if() {
        let src = "
if bar:
    A = 1
    def fooey():
        ...
";
        assert_src_parses_to_expected(src, vec!["A", "fooey"]);
    }

    #[test]
    fn elif_else() {
        let src = "
if bar:
    A = 1
elif fooey:
    B = 2
else:
    C = 3
";
        assert_src_parses_to_expected(src, vec!["A", "B", "C"]);
    }

    #[test]
    fn walrus_if() {
        let src = "
if (foo := bar()):
    ...
";
        assert_src_parses_to_expected(src, vec!["foo"]);
    }

    #[test]
    fn basic_while() {
        let src = "
while True:
    x = []
";
        assert_src_parses_to_expected(src, vec!["x"]);
    }

    #[test]
    fn walrus_while() {
        let src = "
while (foo := bar()):
    ...
";
        assert_src_parses_to_expected(src, vec!["foo"]);
    }

    #[test]
    fn basic_for() {
        let src = "
for foo in bar:
    x = 3
";
        assert_src_parses_to_expected(src, vec!["foo", "x"]);
    }

    #[test]
    fn unpack_for() {
        let src = "
for foo, bar in baz:
    ...
";
        assert_src_parses_to_expected(src, vec!["foo", "bar"]);
    }

    #[test]
    fn async_for() {
        let src = "
async for foo in bar:
    ...
";
        assert_src_parses_to_expected(src, vec!["foo"]);
    }

    #[test]
    fn basic_with() {
        let src = "
with foo(), bar() as baz, xy as (x, y):
    fooey = 3
";
        assert_src_parses_to_expected(src, vec!["baz", "x", "y", "fooey"]);
    }

    #[test]
    fn async_with() {
        let src = "
async with foo as bar:
    fooey = 3
";
        assert_src_parses_to_expected(src, vec!["bar", "fooey"]);
    }

    #[test]
    fn basic_del() {
        let src = "
x, y = 1, 2
del y
";
        assert_src_parses_to_expected(src, vec!["x"]);
    }

    #[test]
    fn reassigned_after_del() {
        let src = "
x = 1
del x
x = 1
";
        assert_src_parses_to_expected(src, vec!["x"]);
    }

    #[test]
    fn multi_target_del() {
        let src = "
x, y, z = 1, 2, 3
del x, z
";
        assert_src_parses_to_expected(src, vec!["y"]);
    }

    #[test]
    fn basic_try_except_else_finally() {
        let src = "
try:
    intry = 1
except ValueError:
    inexcept = 2
except (TypeError, AttributeError):
    inexcept2 = 2.5
else:
    inelse = 3
finally:
    infinally = 4
";
        assert_src_parses_to_expected(
            src,
            vec!["intry", "inexcept", "inexcept2", "inelse", "infinally"],
        );
    }

    #[test]
    fn ignore_try_expection_capture() {
        let src = "
try:
    foo()
except ValueError as e:
    ...
";
        assert_src_parses_to_expected(src, vec![]);
    }

    #[test]
    fn comprehension_does_not_leak() {
        let src = "
[foo for foo in bar]
";
        assert_src_parses_to_expected(src, vec![]);
    }

    #[test]
    fn putting_it_all_together() {
        let src = "
import sys, os
import sys as foo

from foo import (
    bar as baz,
    alpha
)

def my_func():
    ...

class MyClass:
    ...

MYCONSTANT = 1
";

        assert_src_parses_to_expected(
            src,
            vec![
                "sys",
                "os",
                "foo",
                "baz",
                "alpha",
                "my_func",
                "MyClass",
                "MYCONSTANT",
            ],
        );
    }

    #[test]
    fn star_import_not_supported_yet() {
        // TODO: Need to extend __all__ with submodule's __all__
        // e.g. __all__ += submodule.__all__
        let src = "
from submodule import *
";
        assert_src_parses_to_expected(src, vec![]);
    }

    #[test]
    fn truthy_falsey_checks_not_supported_yet() {
        // TODO: Some simple cases could be detected and ignored
        let src = "
if True:
    ...
else:
    x = 1

if False:
    y = 2
";
        assert_src_parses_to_expected(src, vec!["x", "y"]);
    }

    #[test]
    fn simple_raise_conditions_not_checked() {
        // TODO: Could see statically that a ValueError is always being raised
        // And that A will never be set.
        let src = "
try:
    raise ValueError('Fooey!')
except TypeError:
    A = 1
";
        assert_src_parses_to_expected(src, vec!["A"]);
    }

    #[test]
    fn unreachable_code_not_supported_yet() {
        // TODO: Could see that we raise before assignment
        let src = "
assert False
x = 1
";
        assert_src_parses_to_expected(src, vec!["x"]);
    }
}
