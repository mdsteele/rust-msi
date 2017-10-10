use internal::stringpool::{StringPool, StringRef};
use std::fmt;

// ========================================================================= //

/// A value from one cell in a database table row.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Value {
    /// A null value.
    Null,
    /// An integer value.
    Int(i32),
    /// A string value.
    Str(String),
}

impl Value {
    /// Returns true if this is a null value.
    pub fn is_null(&self) -> bool {
        match *self {
            Value::Null => true,
            _ => false,
        }
    }

    /// Returns true if this is an integer value.
    pub fn is_int(&self) -> bool {
        match *self {
            Value::Int(_) => true,
            _ => false,
        }
    }

    /// Extracts the integer value if it is an integer.
    pub fn as_int(&self) -> Option<i32> {
        match *self {
            Value::Null => None,
            Value::Int(number) => Some(number),
            Value::Str(_) => None,
        }
    }

    /// Returns true if this is a string value.
    pub fn is_str(&self) -> bool {
        match *self {
            Value::Str(_) => true,
            _ => false,
        }
    }

    /// Extracts the string value if it is a string.
    pub fn as_str(&self) -> Option<&str> {
        match *self {
            Value::Null => None,
            Value::Int(_) => None,
            Value::Str(ref string) => Some(string.as_str()),
        }
    }

    /// Creates a boolean value.
    pub(crate) fn from_bool(boolean: bool) -> Value {
        if boolean {
            Value::Int(1)
        } else {
            Value::Int(0)
        }
    }

    /// Coerces the `Value` to a boolean.  Returns false for null, zero, and
    /// empty string; returns true for all other values.
    pub(crate) fn to_bool(&self) -> bool {
        match *self {
            Value::Null => false,
            Value::Int(number) => number != 0,
            Value::Str(ref string) => !string.is_empty(),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            Value::Null => formatter.write_str("NULL"),
            Value::Int(number) => number.fmt(formatter),
            Value::Str(ref string) => formatter.write_str(&string),
        }
    }
}

// ========================================================================= //

/// An indirect value from one cell in a database table row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueRef {
    /// A null value.
    Null,
    /// An integer value.
    Int(i32),
    /// A string value.
    Str(StringRef),
}

impl ValueRef {
    /// Interns the given value into the string pool (if it is a string), and
    /// returns a corresponding `ValueRef`.
    pub fn create(value: Value, string_pool: &mut StringPool) -> ValueRef {
        match value {
            Value::Null => ValueRef::Null,
            Value::Int(number) => ValueRef::Int(number),
            Value::Str(string) => ValueRef::Str(string_pool.incref(string)),
        }
    }

    /// Removes the reference from the string pool (if is a string reference).
    #[allow(dead_code)]
    pub fn remove(self, string_pool: &mut StringPool) {
        match self {
            ValueRef::Null | ValueRef::Int(_) => {}
            ValueRef::Str(string_ref) => string_pool.decref(string_ref),
        }
    }

    /// Dereferences the `ValueRef` into a `Value`.
    pub fn to_value(&self, string_pool: &StringPool) -> Value {
        match *self {
            ValueRef::Null => Value::Null,
            ValueRef::Int(number) => Value::Int(number),
            ValueRef::Str(string_ref) => {
                Value::Str(string_pool.get(string_ref).to_string())
            }
        }
    }
}

// ========================================================================= //

#[cfg(test)]
mod tests {
    use super::{Value, ValueRef};
    use internal::codepage::CodePage;
    use internal::stringpool::StringPool;

    #[test]
    fn format_value() {
        assert_eq!(format!("{}", Value::Null), "NULL".to_string());
        assert_eq!(format!("{}", Value::Int(42)), "42".to_string());
        assert_eq!(format!("{}", Value::Int(-137)), "-137".to_string());
        assert_eq!(format!("{}", Value::Str("Hello, world!".to_string())),
                   "Hello, world!".to_string());
    }

    #[test]
    fn create_value_ref() {
        let mut string_pool = StringPool::new(CodePage::default());

        let value = Value::Null;
        let value_ref = ValueRef::create(value.clone(), &mut string_pool);
        assert_eq!(value_ref.to_value(&string_pool), value);

        let value = Value::Int(1234567);
        let value_ref = ValueRef::create(value.clone(), &mut string_pool);
        assert_eq!(value_ref.to_value(&string_pool), value);

        let value = Value::Str("Hello, world!".to_string());
        let value_ref = ValueRef::create(value.clone(), &mut string_pool);
        assert_eq!(value_ref.to_value(&string_pool), value);
    }
}

// ========================================================================= //
