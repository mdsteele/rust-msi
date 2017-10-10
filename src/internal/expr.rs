use internal::table::Row;
use internal::value::Value;
use std::ops;

// ========================================================================= //

/// An expression on database rows that can be used in queries.
pub struct Expr {
    ast: Ast,
}

impl Expr {
    fn unop(op: UnOp, ast: Ast) -> Expr {
        Expr { ast: Ast::UnOp(op, Box::new(ast)) }
    }

    fn binop(op: BinOp, ast1: Ast, ast2: Ast) -> Expr {
        Expr { ast: Ast::BinOp(op, Box::new(ast1), Box::new(ast2)) }
    }

    /// Returns an expression that evaluates to the value of the specified
    /// column.
    pub fn col(column_name: &str) -> Expr {
        Expr { ast: Ast::Column(column_name.to_string()) }
    }

    /// Returns an expression that evaluates to a null value.
    pub fn null() -> Expr { Expr { ast: Ast::Literal(Value::Null) } }

    /// Returns an expression that evaluates to the given integer value.
    pub fn integer(integer: i32) -> Expr {
        Expr { ast: Ast::Literal(Value::Int(integer)) }
    }

    /// Returns an expression that evaluates to the given string value.
    pub fn string(string: &str) -> Expr {
        Expr { ast: Ast::Literal(Value::Str(string.to_string())) }
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
    pub fn bitinv(self) -> Expr { Expr::unop(UnOp::BitNot, self.ast) }

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
    pub fn not(self) -> Expr { Expr::unop(UnOp::BoolNot, self.ast) }

    /// Evaluates the expression against the given row.  Any errors in the
    /// expression (such as dividing a number by zero, or applying a bitwise
    /// operator to a string) will result in a null value.
    pub fn eval(&self, row: &Row) -> Value { self.ast.eval(row) }
}

/// Produces an expression that evaluates to the negative of the subexpression.
/// If the subexpression evaluates to a non-number, the result will be a null
/// value.
impl ops::Neg for Expr {
    type Output = Expr;

    fn neg(self) -> Expr { Expr::unop(UnOp::Neg, self.ast) }
}

/// Produces an expression that evaluates to the sum of the two subexpressions
/// (if they are integers) or concatenation (if they are strings).  If the two
/// subexpressions evaluate to different types, or if either evaluates to a
/// null value, the result will be a null value.
impl ops::Add for Expr {
    type Output = Expr;

    fn add(self, rhs: Expr) -> Expr {
        Expr::binop(BinOp::Add, self.ast, rhs.ast)
    }
}

/// Produces an expression that evaluates to the difference of the two
/// subexpressions.  If either subexpression evaluates to a non-number, the
/// result will be a null value.
impl ops::Sub for Expr {
    type Output = Expr;

    fn sub(self, rhs: Expr) -> Expr {
        Expr::binop(BinOp::Sub, self.ast, rhs.ast)
    }
}

/// Produces an expression that evaluates to the product of the two
/// subexpressions.  If either subexpression evaluates to a non-number, the
/// result will be a null value.
impl ops::Mul for Expr {
    type Output = Expr;

    fn mul(self, rhs: Expr) -> Expr {
        Expr::binop(BinOp::Mul, self.ast, rhs.ast)
    }
}

/// Produces an expression that evaluates to the integer quotient of the two
/// subexpressions.  If either subexpression evaluates to a non-number, or if
/// the divisor evalulates to zero, the result will be a null value.
impl ops::Div for Expr {
    type Output = Expr;

    fn div(self, rhs: Expr) -> Expr {
        Expr::binop(BinOp::Div, self.ast, rhs.ast)
    }
}

/// Produces an expression that evaluates to the bitwise-and of the two
/// subexpressions.  If either subexpression evaluates to a non-number, the
/// result will be a null value.
impl ops::BitAnd for Expr {
    type Output = Expr;

    fn bitand(self, rhs: Expr) -> Expr {
        Expr::binop(BinOp::BitAnd, self.ast, rhs.ast)
    }
}

/// Produces an expression that evaluates to the bitwise-or of the two
/// subexpressions.  If either subexpression evaluates to a non-number, the
/// result will be a null value.
impl ops::BitOr for Expr {
    type Output = Expr;

    fn bitor(self, rhs: Expr) -> Expr {
        Expr::binop(BinOp::BitOr, self.ast, rhs.ast)
    }
}

/// Produces an expression that evaluates to the bitwise-xor of the two
/// subexpressions.  If either subexpression evaluates to a non-number, the
/// result will be a null value.
impl ops::BitXor for Expr {
    type Output = Expr;

    fn bitxor(self, rhs: Expr) -> Expr {
        Expr::binop(BinOp::BitXor, self.ast, rhs.ast)
    }
}

/// Produces an expression that evaluates to the value of the left-hand
/// subexpression bit-shifted left by the value of the right-hand
/// subexpression.  If either subexpression evaluates to a non-number, the
/// result will be a null value.
impl ops::Shl<Expr> for Expr {
    type Output = Expr;

    fn shl(self, rhs: Expr) -> Expr {
        Expr::binop(BinOp::Shl, self.ast, rhs.ast)
    }
}

/// Produces an expression that evaluates to the value of the left-hand
/// subexpression bit-shifted right by the value of the right-hand
/// subexpression.  If either subexpression evaluates to a non-number, the
/// result will be a null value.
impl ops::Shr<Expr> for Expr {
    type Output = Expr;

    fn shr(self, rhs: Expr) -> Expr {
        Expr::binop(BinOp::Shr, self.ast, rhs.ast)
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
            UnOp::Neg => {
                match arg {
                    Value::Int(number) => Value::Int(-number),
                    _ => Value::Null,
                }
            }
            UnOp::BitNot => {
                match arg {
                    Value::Int(number) => Value::Int(!number),
                    _ => Value::Null,
                }
            }
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
            BinOp::Add => {
                match (arg1, arg2) {
                    (Value::Int(num1), Value::Int(num2)) => {
                        Value::Int(num1 + num2)
                    }
                    (Value::Str(str1), Value::Str(str2)) => {
                        Value::Str(str1 + &str2)
                    }
                    _ => Value::Null,
                }
            }
            BinOp::Sub => {
                match (arg1, arg2) {
                    (Value::Int(num1), Value::Int(num2)) => {
                        Value::Int(num1 - num2)
                    }
                    _ => Value::Null,
                }
            }
            BinOp::Mul => {
                match (arg1, arg2) {
                    (Value::Int(num1), Value::Int(num2)) => {
                        Value::Int(num1 * num2)
                    }
                    _ => Value::Null,
                }
            }
            BinOp::Div => {
                match (arg1, arg2) {
                    (_, Value::Int(0)) => Value::Null,
                    (Value::Int(num1), Value::Int(num2)) => {
                        Value::Int(num1 / num2)
                    }
                    _ => Value::Null,
                }
            }
            BinOp::BitAnd => {
                match (arg1, arg2) {
                    (Value::Int(num1), Value::Int(num2)) => {
                        Value::Int(num1 & num2)
                    }
                    _ => Value::Null,
                }
            }
            BinOp::BitOr => {
                match (arg1, arg2) {
                    (Value::Int(num1), Value::Int(num2)) => {
                        Value::Int(num1 | num2)
                    }
                    _ => Value::Null,
                }
            }
            BinOp::BitXor => {
                match (arg1, arg2) {
                    (Value::Int(num1), Value::Int(num2)) => {
                        Value::Int(num1 ^ num2)
                    }
                    _ => Value::Null,
                }
            }
            BinOp::Shl => {
                match (arg1, arg2) {
                    (Value::Int(num1), Value::Int(num2)) => {
                        Value::Int(num1 << num2)
                    }
                    _ => Value::Null,
                }
            }
            BinOp::Shr => {
                match (arg1, arg2) {
                    (Value::Int(num1), Value::Int(num2)) => {
                        Value::Int(num1 >> num2)
                    }
                    _ => Value::Null,
                }
            }
        }
    }
}

// ========================================================================= //

#[cfg(test)]
mod tests {
    use super::Expr;
    use internal::column::Column;
    use internal::table::{Row, Table};
    use internal::value::Value;

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
            Value::Str("foo".to_string()),
            Value::Int(42),
            Value::Str("bar".to_string()),
            Value::Null,
            Value::Int(-17),
        ];
        let row = Row::new(&table, values);

        assert_eq!(Expr::col("Str2").gt(Expr::col("Str1")).eval(&row),
                   Value::from_bool(false));
        assert_eq!(Expr::col("Null")
                       .eq(Expr::null())
                       .and(Expr::col("Int2").lt(Expr::integer(0)))
                       .eval(&row),
                   Value::from_bool(true));
        assert_eq!(Expr::col("Null")
                       .or(Expr::col("Int1").ne(Expr::col("Int2")))
                       .eval(&row),
                   Value::from_bool(true));
        assert_eq!(((Expr::col("Int1") - Expr::col("Int2")) *
                        Expr::col("Int1"))
                       .eval(&row),
                   Value::Int(2478));
        assert_eq!(((Expr::col("Int1") << Expr::integer(2)) ^
                        Expr::col("Int2"))
                       .eval(&row),
                   Value::Int(-185));
        assert_eq!((Expr::col("Int2") / Expr::integer(0)).eval(&row),
                   Value::Null);
        assert_eq!((Expr::col("Str1") + Expr::string(":") +
                       Expr::col("Str2"))
                       .eval(&row),
                   Value::Str("foo:bar".to_string()));
    }
}

// ========================================================================= //
