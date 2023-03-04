use crate::internal::language::Language;
use crate::internal::stringpool::{StringPool, StringRef};
use std::convert::From;
use std::fmt;
use uuid::Uuid;

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
        matches!(*self, Value::Null)
    }

    /// Returns true if this is an integer value.
    pub fn is_int(&self) -> bool {
        matches!(*self, Value::Int(_))
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
        matches!(*self, Value::Str(_))
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
            Value::Null => "NULL".fmt(formatter),
            Value::Int(number) => number.fmt(formatter),
            Value::Str(ref string) => format!("{string:?}").fmt(formatter),
        }
    }
}

impl From<bool> for Value {
    fn from(boolean: bool) -> Value {
        Value::from_bool(boolean)
    }
}

impl From<i16> for Value {
    fn from(integer: i16) -> Value {
        Value::Int(integer as i32)
    }
}

impl From<u16> for Value {
    fn from(integer: u16) -> Value {
        Value::Int(integer as i32)
    }
}

impl From<i32> for Value {
    fn from(integer: i32) -> Value {
        Value::Int(integer)
    }
}

impl<'a> From<&'a str> for Value {
    fn from(string: &'a str) -> Value {
        Value::Str(string.to_string())
    }
}

impl From<String> for Value {
    fn from(string: String) -> Value {
        Value::Str(string)
    }
}

/// Returns a string value containing the code for the given language, suitable
/// for storing in a column with the `Language` category.
impl From<Language> for Value {
    fn from(language: Language) -> Value {
        Value::Str(format!("{}", language.code()))
    }
}

/// Returns a string value containing the codes for the given languages,
/// suitable for storing in a column with the `Language` category.
impl<'a> From<&'a [Language]> for Value {
    fn from(languages: &'a [Language]) -> Value {
        let codes: Vec<String> =
            languages.iter().map(|lang| lang.code().to_string()).collect();
        Value::Str(codes.join(","))
    }
}

/// Returns a string value containing the given UUID, suitable for storing in a
/// column with the `Guid` category.
impl From<Uuid> for Value {
    fn from(uuid: Uuid) -> Value {
        let mut string = format!("{{{}}}", uuid.hyphenated());
        string.make_ascii_uppercase();
        Value::Str(string)
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
    pub fn remove(self, string_pool: &mut StringPool) {
        match self {
            ValueRef::Null | ValueRef::Int(_) => {}
            ValueRef::Str(string_ref) => string_pool.decref(string_ref),
        }
    }

    /// Dereferences the `ValueRef` into a `Value`.
    pub fn to_value(self, string_pool: &StringPool) -> Value {
        match self {
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
    use crate::internal::codepage::CodePage;
    use crate::internal::language::Language;
    use crate::internal::stringpool::StringPool;
    use uuid::Uuid;

    #[test]
    fn format_value() {
        assert_eq!(format!("{}", Value::Null), "NULL".to_string());
        assert_eq!(format!("{}", Value::Int(42)), "42".to_string());
        assert_eq!(format!("{}", Value::Int(-137)), "-137".to_string());
        assert_eq!(
            format!("{}", Value::Str("Hello, world!".to_string())),
            "\"Hello, world!\"".to_string()
        );

        assert_eq!(format!("{:>6}", Value::Null), "  NULL".to_string());
        assert_eq!(format!("[{:<4}]", Value::Int(42)), "[42  ]".to_string());
        assert_eq!(
            format!("foo{:~>8}", Value::Str("bar".to_string())),
            "foo~~~\"bar\"".to_string()
        );
    }

    #[test]
    fn value_from() {
        assert_eq!(Value::from(false), Value::Int(0));
        assert_eq!(Value::from(true), Value::Int(1));
        assert_eq!(Value::from(-47i16), Value::Int(-47i32));
        assert_eq!(Value::from(47u16), Value::Int(47i32));
        assert_eq!(Value::from("foobar"), Value::Str("foobar".to_string()));
        assert_eq!(
            Value::from("foobar".to_string()),
            Value::Str("foobar".to_string())
        );
        assert_eq!(
            Value::from(Language::from_tag("en-US")),
            Value::Str("1033".to_string())
        );
        assert_eq!(
            Value::from(&[
                Language::from_code(1033),
                Language::from_code(2107),
                Language::from_code(3131),
            ] as &[Language],),
            Value::Str("1033,2107,3131".to_string())
        );
        assert_eq!(
            Value::from(
                Uuid::parse_str("34ab5c53-9b30-4e14-aef0-2c1c7ba826c0")
                    .unwrap()
            ),
            Value::Str("{34AB5C53-9B30-4E14-AEF0-2C1C7BA826C0}".to_string())
        );
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
