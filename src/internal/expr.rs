use crate::internal::table::Row;
use crate::internal::value::Value;
use std::collections::HashSet;
use std::fmt;
use std::ops;

// ========================================================================= //

/// An expression on database rows that can be used in queries.
pub struct Expr {
    ast: Ast,
}

#[allow(clippy::should_implement_trait)]
impl Expr {
    fn unop(op: UnOp, ast: Ast) -> Expr {
        Expr {
            ast: match ast {
                Ast::Literal(value) => Ast::Literal(op.eval(value)),
                ast => Ast::UnOp(op, Box::new(ast)),
            },
        }
    }

    fn binop(op: BinOp, ast1: Ast, ast2: Ast) -> Expr {
        Expr {
            ast: match (ast1, ast2) {
                (Ast::Literal(value1), Ast::Literal(value2)) => {
                    Ast::Literal(op.eval(value1, value2))
                }
                (ast1, ast2) => Ast::BinOp(op, Box::new(ast1), Box::new(ast2)),
            },
        }
    }

    /// Returns an expression that evaluates to the value of the specified
    /// column.
    pub fn col<S: Into<String>>(column_name: S) -> Expr {
        Expr { ast: Ast::Column(column_name.into()) }
    }

    /// Returns an expression that evaluates to a null value.
    pub fn null() -> Expr {
        Expr { ast: Ast::Literal(Value::Null) }
    }

    /// Returns an expression that evaluates to the given boolean value.
    pub fn boolean(boolean: bool) -> Expr {
        Expr { ast: Ast::Literal(Value::from_bool(boolean)) }
    }

    /// Returns an expression that evaluates to the given integer value.
    pub fn integer(integer: i32) -> Expr {
        Expr { ast: Ast::Literal(Value::Int(integer)) }
    }

    /// Returns an expression that evaluates to the given string value.
    pub fn string<S: Into<String>>(string: S) -> Expr {
        Expr { ast: Ast::Literal(Value::Str(string.into())) }
    }

    /// Returns an expression that evaluates to true if the two subexpressions
    /// evaluate to equal values.
    pub fn eq(self, rhs: Expr) -> Expr {
        Expr::binop(BinOp::Eq, self.ast, rhs.ast)
    }

    /// Returns an expression that evaluates to true if the two subexpressions
    /// evaluate to unequal values.
    pub fn ne(self, rhs: Expr) -> Expr {
        Expr::binop(BinOp::Ne, self.ast, rhs.ast)
    }

    /// Returns an expression that evaluates to true if the left-hand
    /// subexpression evaluates to a strictly lesser value than the right-hand
    /// subexpression.
    pub fn lt(self, rhs: Expr) -> Expr {
        Expr::binop(BinOp::Lt, self.ast, rhs.ast)
    }

    /// Returns an expression that evaluates to true if the left-hand
    /// subexpression evaluates to a lesser-or-equal value than the right-hand
    /// subexpression.
    pub fn le(self, rhs: Expr) -> Expr {
        Expr::binop(BinOp::Le, self.ast, rhs.ast)
    }

    /// Returns an expression that evaluates to true if the left-hand
    /// subexpression evaluates to a strictly greater value than the right-hand
    /// subexpression.
    pub fn gt(self, rhs: Expr) -> Expr {
        Expr::binop(BinOp::Gt, self.ast, rhs.ast)
    }

    /// Returns an expression that evaluates to true if the left-hand
    /// subexpression evaluates to a greater-or-equal value than the right-hand
    /// subexpression.
    pub fn ge(self, rhs: Expr) -> Expr {
        Expr::binop(BinOp::Ge, self.ast, rhs.ast)
    }

    /// Returns an expression that computes the bitwise inverse of the
    /// subexpression.  If the subexpression evaluates to a non-number, the
    /// result will be a null value.
    ///
    /// This method exists instead of the `std::ops::Not` trait to distinguish
    /// it from the (logical) `not()` method.
    pub fn bitinv(self) -> Expr {
        Expr::unop(UnOp::BitNot, self.ast)
    }

    /// Returns an expression that evaluates to true if both subexpressions
    /// evaluate to true.
    pub fn and(self, rhs: Expr) -> Expr {
        Expr { ast: Ast::And(Box::new(self.ast), Box::new(rhs.ast)) }
    }

    /// Returns an expression that evaluates to true if either subexpression
    /// evaluates to true.
    pub fn or(self, rhs: Expr) -> Expr {
        Expr { ast: Ast::Or(Box::new(self.ast), Box::new(rhs.ast)) }
    }

    /// Returns an expression that evaluates to true if the subexpression
    /// evaluates to false.
    ///
    /// This method exists instead of the `std::ops::Not` trait to distinguish
    /// it from the (bitwise) `bitinv()` method.
    pub fn not(self) -> Expr {
        Expr::unop(UnOp::BoolNot, self.ast)
    }

    /// Evaluates the expression against the given row.  Any errors in the
    /// expression (such as dividing a number by zero, or applying a bitwise
    /// operator to a string) will result in a null value.
    pub fn eval(&self, row: &Row) -> Value {
        self.ast.eval(row)
    }

    /// Returns the set of all column names referenced by this expression.
    pub fn column_names(&self) -> HashSet<&str> {
        let mut names = HashSet::new();
        self.ast.populate_column_names(&mut names);
        names
    }
}

/// Produces an expression that evaluates to the negative of the subexpression.
/// If the subexpression evaluates to a non-number, the result will be a null
/// value.
impl ops::Neg for Expr {
    type Output = Expr;

    fn neg(self) -> Self::Output {
        Expr::unop(UnOp::Neg, self.ast)
    }
}

/// Produces an expression that evaluates to the sum of the two subexpressions
/// (if they are integers) or concatenation (if they are strings).  If the two
/// subexpressions evaluate to different types, or if either evaluates to a
/// null value, the result will be a null value.
impl ops::Add for Expr {
    type Output = Expr;

    fn add(self, rhs: Expr) -> Self::Output {
        Expr::binop(BinOp::Add, self.ast, rhs.ast)
    }
}

/// Produces an expression that evaluates to the difference of the two
/// subexpressions.  If either subexpression evaluates to a non-number, the
/// result will be a null value.
impl ops::Sub for Expr {
    type Output = Expr;

    fn sub(self, rhs: Expr) -> Self::Output {
        Expr::binop(BinOp::Sub, self.ast, rhs.ast)
    }
}

/// Produces an expression that evaluates to the product of the two
/// subexpressions.  If either subexpression evaluates to a non-number, the
/// result will be a null value.
impl ops::Mul for Expr {
    type Output = Expr;

    fn mul(self, rhs: Expr) -> Self::Output {
        Expr::binop(BinOp::Mul, self.ast, rhs.ast)
    }
}

/// Produces an expression that evaluates to the integer quotient of the two
/// subexpressions.  If either subexpression evaluates to a non-number, or if
/// the divisor evalulates to zero, the result will be a null value.
impl ops::Div for Expr {
    type Output = Expr;

    fn div(self, rhs: Expr) -> Self::Output {
        Expr::binop(BinOp::Div, self.ast, rhs.ast)
    }
}

/// Produces an expression that evaluates to the bitwise-and of the two
/// subexpressions.  If either subexpression evaluates to a non-number, the
/// result will be a null value.
impl ops::BitAnd for Expr {
    type Output = Expr;

    fn bitand(self, rhs: Expr) -> Self::Output {
        Expr::binop(BinOp::BitAnd, self.ast, rhs.ast)
    }
}

/// Produces an expression that evaluates to the bitwise-or of the two
/// subexpressions.  If either subexpression evaluates to a non-number, the
/// result will be a null value.
impl ops::BitOr for Expr {
    type Output = Expr;

    fn bitor(self, rhs: Expr) -> Self::Output {
        Expr::binop(BinOp::BitOr, self.ast, rhs.ast)
    }
}

/// Produces an expression that evaluates to the bitwise-xor of the two
/// subexpressions.  If either subexpression evaluates to a non-number, the
/// result will be a null value.
impl ops::BitXor for Expr {
    type Output = Expr;

    fn bitxor(self, rhs: Expr) -> Self::Output {
        Expr::binop(BinOp::BitXor, self.ast, rhs.ast)
    }
}

/// Produces an expression that evaluates to the value of the left-hand
/// subexpression bit-shifted left by the value of the right-hand
/// subexpression.  If either subexpression evaluates to a non-number, the
/// result will be a null value.
impl ops::Shl<Expr> for Expr {
    type Output = Expr;

    fn shl(self, rhs: Expr) -> Self::Output {
        Expr::binop(BinOp::Shl, self.ast, rhs.ast)
    }
}

/// Produces an expression that evaluates to the value of the left-hand
/// subexpression bit-shifted right by the value of the right-hand
/// subexpression.  If either subexpression evaluates to a non-number, the
/// result will be a null value.
impl ops::Shr<Expr> for Expr {
    type Output = Expr;

    fn shr(self, rhs: Expr) -> Self::Output {
        Expr::binop(BinOp::Shr, self.ast, rhs.ast)
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.ast.fmt(formatter)
    }
}

// ========================================================================= //

/// An abstract syntax tree for expressions.
enum Ast {
    Literal(Value),
    Column(String),
    UnOp(UnOp, Box<Ast>),
    BinOp(BinOp, Box<Ast>, Box<Ast>),
    And(Box<Ast>, Box<Ast>),
    Or(Box<Ast>, Box<Ast>),
}

impl Ast {
    fn eval(&self, row: &Row) -> Value {
        match *self {
            Ast::Literal(ref value) => value.clone(),
            Ast::Column(ref name) => row[name.as_str()].clone(),
            Ast::UnOp(op, ref arg) => op.eval(arg.eval(row)),
            Ast::BinOp(op, ref arg1, ref arg2) => {
                op.eval(arg1.eval(row), arg2.eval(row))
            }
            Ast::And(ref arg1, ref arg2) => {
                if arg1.eval(row).to_bool() {
                    Value::from_bool(arg2.eval(row).to_bool())
                } else {
                    Value::from_bool(false)
                }
            }
            Ast::Or(ref arg1, ref arg2) => {
                if arg1.eval(row).to_bool() {
                    Value::from_bool(true)
                } else {
                    Value::from_bool(arg2.eval(row).to_bool())
                }
            }
        }
    }

    fn populate_column_names<'a>(&'a self, names: &mut HashSet<&'a str>) {
        match *self {
            Ast::Literal(_) => {}
            Ast::Column(ref name) => {
                names.insert(name.as_str());
            }
            Ast::UnOp(_, ref arg) => arg.populate_column_names(names),
            Ast::BinOp(_, ref arg1, ref arg2)
            | Ast::And(ref arg1, ref arg2)
            | Ast::Or(ref arg1, ref arg2) => {
                arg1.populate_column_names(names);
                arg2.populate_column_names(names);
            }
        }
    }

    fn format_with_precedence(
        &self,
        formatter: &mut fmt::Formatter,
        parent_prec: i32,
    ) -> Result<(), fmt::Error> {
        match self {
            Ast::Literal(ref value) => fmt::Display::fmt(value, formatter),
            Ast::Column(ref name) => formatter.write_str(name.as_str()),
            Ast::UnOp(op, ref arg) => {
                match op {
                    UnOp::Neg => formatter.write_str("-")?,
                    UnOp::BitNot => formatter.write_str("~")?,
                    UnOp::BoolNot => formatter.write_str("NOT ")?,
                }
                arg.format_with_precedence(formatter, 10)
            }
            Ast::BinOp(op, ref arg1, ref arg2) => {
                let op_prec = op.precedence();
                if op_prec < parent_prec {
                    formatter.write_str("(")?;
                }
                arg1.format_with_precedence(formatter, op_prec)?;
                match op {
                    BinOp::Eq => formatter.write_str(" = ")?,
                    BinOp::Ne => formatter.write_str(" != ")?,
                    BinOp::Lt => formatter.write_str(" < ")?,
                    BinOp::Le => formatter.write_str(" <= ")?,
                    BinOp::Gt => formatter.write_str(" > ")?,
                    BinOp::Ge => formatter.write_str(" >= ")?,
                    BinOp::Add => formatter.write_str(" + ")?,
                    BinOp::Sub => formatter.write_str(" - ")?,
                    BinOp::Mul => formatter.write_str(" * ")?,
                    BinOp::Div => formatter.write_str(" / ")?,
                    BinOp::BitAnd => formatter.write_str(" & ")?,
                    BinOp::BitOr => formatter.write_str(" | ")?,
                    BinOp::BitXor => formatter.write_str(" ^ ")?,
                    BinOp::Shl => formatter.write_str(" << ")?,
                    BinOp::Shr => formatter.write_str(" >> ")?,
                }
                arg2.format_with_precedence(formatter, op_prec + 1)?;
                if op_prec < parent_prec {
                    formatter.write_str(")")?;
                }
                Ok(())
            }
            Ast::And(ref arg1, ref arg2) => {
                let op_prec = 2;
                if op_prec < parent_prec {
                    formatter.write_str("(")?;
                }
                arg1.format_with_precedence(formatter, op_prec)?;
                formatter.write_str(" AND ")?;
                arg2.format_with_precedence(formatter, op_prec + 1)?;
                if op_prec < parent_prec {
                    formatter.write_str(")")?;
                }
                Ok(())
            }
            Ast::Or(ref arg1, ref arg2) => {
                let op_prec = 1;
                if op_prec < parent_prec {
                    formatter.write_str("(")?;
                }
                arg1.format_with_precedence(formatter, op_prec)?;
                formatter.write_str(" OR ")?;
                arg2.format_with_precedence(formatter, op_prec + 1)?;
                if op_prec < parent_prec {
                    formatter.write_str(")")?;
                }
                Ok(())
            }
        }
    }
}

impl fmt::Display for Ast {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.format_with_precedence(formatter, 0)
    }
}

// ========================================================================= //

/// A unary operation.
#[derive(Clone, Copy)]
enum UnOp {
    Neg,
    BitNot,
    BoolNot,
}

impl UnOp {
    fn eval(&self, arg: Value) -> Value {
        match *self {
            UnOp::Neg => match arg {
                Value::Int(number) => Value::Int(-number),
                _ => Value::Null,
            },
            UnOp::BitNot => match arg {
                Value::Int(number) => Value::Int(!number),
                _ => Value::Null,
            },
            UnOp::BoolNot => Value::from_bool(!arg.to_bool()),
        }
    }
}

// ========================================================================= //

/// A binary operation.
#[derive(Clone, Copy)]
enum BinOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

impl BinOp {
    fn eval(&self, arg1: Value, arg2: Value) -> Value {
        match *self {
            BinOp::Eq => Value::from_bool(arg1 == arg2),
            BinOp::Ne => Value::from_bool(arg1 != arg2),
            BinOp::Lt => Value::from_bool(arg1 < arg2),
            BinOp::Le => Value::from_bool(arg1 <= arg2),
            BinOp::Gt => Value::from_bool(arg1 > arg2),
            BinOp::Ge => Value::from_bool(arg1 >= arg2),
            BinOp::Add => match (arg1, arg2) {
                (Value::Int(num1), Value::Int(num2)) => {
                    Value::Int(num1 + num2)
                }
                (Value::Str(str1), Value::Str(str2)) => {
                    Value::Str(str1 + &str2)
                }
                _ => Value::Null,
            },
            BinOp::Sub => match (arg1, arg2) {
                (Value::Int(num1), Value::Int(num2)) => {
                    Value::Int(num1 - num2)
                }
                _ => Value::Null,
            },
            BinOp::Mul => match (arg1, arg2) {
                (Value::Int(num1), Value::Int(num2)) => {
                    Value::Int(num1 * num2)
                }
                _ => Value::Null,
            },
            BinOp::Div => match (arg1, arg2) {
                (_, Value::Int(0)) => Value::Null,
                (Value::Int(num1), Value::Int(num2)) => {
                    Value::Int(num1 / num2)
                }
                _ => Value::Null,
            },
            BinOp::BitAnd => match (arg1, arg2) {
                (Value::Int(num1), Value::Int(num2)) => {
                    Value::Int(num1 & num2)
                }
                _ => Value::Null,
            },
            BinOp::BitOr => match (arg1, arg2) {
                (Value::Int(num1), Value::Int(num2)) => {
                    Value::Int(num1 | num2)
                }
                _ => Value::Null,
            },
            BinOp::BitXor => match (arg1, arg2) {
                (Value::Int(num1), Value::Int(num2)) => {
                    Value::Int(num1 ^ num2)
                }
                _ => Value::Null,
            },
            BinOp::Shl => match (arg1, arg2) {
                (Value::Int(num1), Value::Int(num2)) => {
                    Value::Int(num1 << num2)
                }
                _ => Value::Null,
            },
            BinOp::Shr => match (arg1, arg2) {
                (Value::Int(num1), Value::Int(num2)) => {
                    Value::Int(num1 >> num2)
                }
                _ => Value::Null,
            },
        }
    }

    fn precedence(&self) -> i32 {
        match *self {
            BinOp::Eq => 3,
            BinOp::Ne => 3,
            BinOp::Lt => 3,
            BinOp::Le => 3,
            BinOp::Gt => 3,
            BinOp::Ge => 3,
            BinOp::Add => 8,
            BinOp::Sub => 8,
            BinOp::Mul => 9,
            BinOp::Div => 9,
            BinOp::BitAnd => 6,
            BinOp::BitOr => 4,
            BinOp::BitXor => 5,
            BinOp::Shl => 7,
            BinOp::Shr => 7,
        }
    }
}

// ========================================================================= //

#[cfg(test)]
mod tests {
    use super::Expr;
    use crate::internal::column::Column;
    use crate::internal::table::{Row, Table};
    use crate::internal::value::Value;
    use std::collections::HashSet;

    #[test]
    fn evaluate() {
        let columns = vec![
            Column::build("Str1").string(10),
            Column::build("Int1").int16(),
            Column::build("Str2").string(10),
            Column::build("Null").nullable().int16(),
            Column::build("Int2").int32(),
        ];
        let table = Table::new("Example".to_string(), columns, false);
        let values = vec![
            Value::from("foo"),
            Value::Int(42),
            Value::from("bar"),
            Value::Null,
            Value::Int(-17),
        ];
        let row = Row::new(table, values);

        assert_eq!(
            Expr::col("Str2").gt(Expr::col("Str1")).eval(&row),
            Value::from_bool(false)
        );
        assert_eq!(
            Expr::col("Null")
                .eq(Expr::null())
                .and(Expr::col("Int2").lt(Expr::integer(0)))
                .eval(&row),
            Value::from_bool(true)
        );
        assert_eq!(
            Expr::col("Null")
                .or(Expr::col("Int1").ne(Expr::col("Int2")))
                .eval(&row),
            Value::from_bool(true)
        );
        assert_eq!(
            ((Expr::col("Int1") - Expr::col("Int2")) * Expr::col("Int1"))
                .eval(&row),
            Value::Int(2478)
        );
        assert_eq!(
            ((Expr::col("Int1") << Expr::integer(2)) ^ Expr::col("Int2"))
                .eval(&row),
            Value::Int(-185)
        );
        assert_eq!(
            (Expr::col("Int2") / Expr::integer(0)).eval(&row),
            Value::Null
        );
        assert_eq!(
            (Expr::col("Str1") + Expr::string(":") + Expr::col("Str2"))
                .eval(&row),
            Value::from("foo:bar")
        );
    }

    #[test]
    fn column_names() {
        let expr = (Expr::col("Foo") / Expr::integer(10))
            .le(Expr::col("Bar"))
            .or(Expr::col("Baz").ge(Expr::col("Foo")));
        let expected: HashSet<&str> =
            vec!["Foo", "Bar", "Baz"].into_iter().collect();
        assert_eq!(expr.column_names(), expected);
    }

    #[test]
    fn display() {
        let expr = (Expr::col("Foo") / Expr::integer(10))
            .le(Expr::col("Bar"))
            .or(Expr::col("Baz").ge(Expr::col("Foo")));
        assert_eq!(
            expr.to_string(),
            "Foo / 10 <= Bar OR Baz >= Foo".to_string()
        );

        let expr = Expr::col("Foo") * (Expr::integer(10) + Expr::col("Bar"));
        assert_eq!(expr.to_string(), "Foo * (10 + Bar)".to_string());

        let expr = (Expr::col("Foo") + Expr::integer(10)) * Expr::col("Bar");
        assert_eq!(expr.to_string(), "(Foo + 10) * Bar".to_string());

        let expr = Expr::col("Foo").and(Expr::col("Bar")).or(Expr::col("Baz"));
        assert_eq!(expr.to_string(), "Foo AND Bar OR Baz".to_string());

        let expr = Expr::col("Foo").or(Expr::col("Bar")).and(Expr::col("Baz"));
        assert_eq!(expr.to_string(), "(Foo OR Bar) AND Baz".to_string());

        let expr = Expr::col("Foo") - Expr::col("Bar") - Expr::col("Baz");
        assert_eq!(expr.to_string(), "Foo - Bar - Baz".to_string());

        let expr = Expr::col("Foo") - (Expr::col("Bar") - Expr::col("Baz"));
        assert_eq!(expr.to_string(), "Foo - (Bar - Baz)".to_string());

        let expr = Expr::col("Foo").or(Expr::col("Bar")).or(Expr::col("Baz"));
        assert_eq!(expr.to_string(), "Foo OR Bar OR Baz".to_string());

        let expr = Expr::col("Foo").or(Expr::col("Bar").or(Expr::col("Baz")));
        assert_eq!(expr.to_string(), "Foo OR (Bar OR Baz)".to_string());
    }

    #[test]
    fn constant_folding() {
        let expr = -Expr::integer(-5) + Expr::col("Foo");
        assert_eq!(expr.to_string(), "5 + Foo".to_string());

        let expr = Expr::integer(3) * Expr::integer(4) - Expr::integer(2);
        assert_eq!(expr.to_string(), "10".to_string());
    }
}

// ========================================================================= //
