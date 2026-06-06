use crate::internal::language::LanguageId;
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
    /// A binary stream
    Binary,
}

impl Value {
    /// Returns true if this is a null value.
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(*self, Self::Null)
    }

    /// Returns true if this is an integer value.
    #[must_use]
    pub fn is_int(&self) -> bool {
        matches!(*self, Self::Int(_))
    }

    /// Extracts the integer value if it is an integer.
    #[must_use]
    pub fn as_int(&self) -> Option<i32> {
        match *self {
            Self::Int(number) => Some(number),
            Self::Null | Self::Str(_) | Self::Binary => None,
        }
    }

    /// Returns true if this is a string value.
    #[must_use]
    pub fn is_str(&self) -> bool {
        matches!(*self, Self::Str(_))
    }

    /// Extracts the string value if it is a string.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match *self {
            Self::Str(ref string) => Some(string.as_str()),
            Self::Null | Self::Int(_) | Self::Binary => None,
        }
    }

    /// Creates a boolean value.
    pub(crate) fn from_bool(boolean: bool) -> Self {
        if boolean { Self::Int(1) } else { Self::Int(0) }
    }

    /// Coerces the `Value` to a boolean.  Returns false for null, zero, and
    /// empty string; returns true for all other values.
    pub(crate) fn to_bool(&self) -> bool {
        match *self {
            Self::Null => false,
            Self::Int(number) => number != 0,
            Self::Str(ref string) => !string.is_empty(),
            // Because binary streams cannot be null, we return true here.
            Self::Binary => true,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            Self::Null => "NULL".fmt(formatter),
            Self::Int(number) => number.fmt(formatter),
            Self::Str(ref string) => format!("{string:?}").fmt(formatter),
            Self::Binary => "BINARY_STREAM".fmt(formatter),
        }
    }
}

impl From<bool> for Value {
    fn from(boolean: bool) -> Self {
        Self::from_bool(boolean)
    }
}

impl From<i16> for Value {
    fn from(integer: i16) -> Self {
        Self::Int(integer as i32)
    }
}

impl From<u16> for Value {
    fn from(integer: u16) -> Self {
        Self::Int(integer as i32)
    }
}

impl From<i32> for Value {
    fn from(integer: i32) -> Self {
        Self::Int(integer)
    }
}

impl<'a> From<&'a str> for Value {
    fn from(string: &'a str) -> Self {
        Self::Str(string.to_string())
    }
}

impl From<String> for Value {
    fn from(string: String) -> Self {
        Self::Str(string)
    }
}

/// Returns a string value containing the code for the given language, suitable
/// for storing in a column with the `Language` category.
impl From<LanguageId> for Value {
    fn from(language: LanguageId) -> Self {
        Self::Str(format!("{}", language.id()))
    }
}

/// Returns a string value containing the codes for the given languages,
/// suitable for storing in a column with the `Language` category.
impl<'a> From<&'a [LanguageId]> for Value {
    fn from(languages: &'a [LanguageId]) -> Self {
        let codes: Vec<String> =
            languages.iter().map(|lang| lang.id().to_string()).collect();
        Self::Str(codes.join(","))
    }
}

/// Returns a string value containing the given UUID, suitable for storing in a
/// column with the `Guid` category.
impl From<Uuid> for Value {
    fn from(uuid: Uuid) -> Self {
        let mut string = format!("{{{}}}", uuid.hyphenated());
        string.make_ascii_uppercase();
        Self::Str(string)
    }
}

// ========================================================================= //

/// An indirect value from one cell in a database table row.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ValueRef {
    /// A null value.
    Null,
    /// An integer value.
    Int(i32),
    /// A string value.
    Str(StringRef),
    /// A binary stream
    Binary,
}

impl ValueRef {
    /// Interns the given value into the string pool (if it is a string), and
    /// returns a corresponding `ValueRef`.
    pub fn create(value: Value, string_pool: &mut StringPool) -> Self {
        match value {
            Value::Null => Self::Null,
            Value::Int(number) => Self::Int(number),
            Value::Str(string) => Self::Str(string_pool.incref(string)),
            Value::Binary => Self::Binary,
        }
    }

    /// Removes the reference from the string pool (if is a string reference).
    pub fn remove(self, string_pool: &mut StringPool) {
        match self {
            Self::Null | Self::Int(_) | Self::Binary => {}
            Self::Str(string_ref) => string_pool.decref(string_ref),
        }
    }

    /// Dereferences the `ValueRef` into a `Value`.
    pub fn to_value(self, string_pool: &StringPool) -> Value {
        match self {
            Self::Null => Value::Null,
            Self::Int(number) => Value::Int(number),
            Self::Str(string_ref) => {
                Value::Str(string_pool.get(string_ref).to_string())
            }
            Self::Binary => Value::Binary,
        }
    }
}

// ========================================================================= //

#[cfg(test)]
mod tests {
    use super::{Value, ValueRef};
    use crate::internal::codepage::CodePage;
    use crate::internal::language::LanguageId;
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
            Value::from(LanguageId::from_tag("en-US")),
            Value::Str("1033".to_string())
        );
        assert_eq!(
            Value::from(&[
                LanguageId::from_id(1033),
                LanguageId::from_id(2107),
                LanguageId::from_id(3131),
            ] as &[LanguageId],),
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
