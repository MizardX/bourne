pub mod error;
pub mod parse;
pub mod format;
pub use bournemacro::json;

/// The Mapping that [Value] uses for [Value::Object].  
/// Uses [hashbrown::HashMap].
#[cfg(not(feature = "preserve_order"))]
pub type ValueMap = hashbrown::HashMap<String, Value>;
/// The Mapping that [Value] uses for [Value::Object].  
/// Uses [indexmap::IndexMap] (`preserve_order` feature is on)
#[cfg(feature = "preserve_order")]
pub type ValueMap = indexmap::IndexMap<String, Value>;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Number {
    Float(f64),
    Int(i64),
}

/// JSON Value.
#[derive(Debug, Clone)]
pub enum Value {
    /// Null value.
    /// ```no_run,json
    /// null
    /// ```
    Null,
    /// A boolean value.
    /// ```no_run,json
    /// true
    /// ```
    /// or
    /// ```no_run,json
    /// false
    /// ```
    Boolean(bool),
    /// An [f64] or [i64] number.
    /// ```no_run,json
    /// 3.14159265358979
    /// ```
    Number(Number),
    /// A UTF-8 encoded string.
    /// ```no_run,json
    /// "The quick brown fox jumps over the lazy dog.\nhello, world"
    /// ```
    /// The following characters must be escaped:  
    /// * `\u{0}`to `\u{1f}` (inclusive)
    /// * `\n` (newline)
    /// * `\r` (carriage return)
    /// * `\t` (tab) (optional)
    /// * `"`
    /// * `'` (optional)
    /// * `\`
    /// * `/` (optional)
    /// * `\u{8}`
    /// * `\u{c}`
    String(String),
    /// An array of JSON [Value]s.
    /// ```no_run,json
    /// [
    ///     "hello, world",
    ///     1234,
    ///     3.14,
    ///     true,
    ///     false,
    ///     null,
    /// ]
    /// ```
    Array(Vec<Value>),
    /// A Mapping of JSON [Value]s by their name.
    /// ```no_run,json
    /// {
    ///     "tag": null,
    ///     "registered": true,
    ///     "age": 197,
    ///     "name": "Fred",
    ///     "classes": [
    ///         "Algebra",
    ///         "History of Programming",
    ///         "Algorithms and Datastructures",
    ///         "Cryptography",
    ///     ],
    ///     "rgb_for_some_reason": {
    ///         "r": 4,
    ///         "g": 7
    ///         "b": 3,
    ///     }
    /// }
    /// ```
    Object(ValueMap),
}

impl From<bool> for Value {
    /// Create a [Value] from a [bool].
    fn from(value: bool) -> Self {
        Value::Boolean(value)
    }
}

impl From<f64> for Value {
    /// Create a [Value] from an [f64].
    fn from(value: f64) -> Self {
        Value::Number(Number::Float(value))
    }
}

impl From<String> for Value {
    /// Create a [Value] from a [String].
    fn from(value: String) -> Self {
        Value::String(value)
    }
}

impl From<&str> for Value {
    /// Create a [Value] from string.
    fn from(value: &str) -> Self {
        Value::String(value.to_owned())
    }
}

impl From<Vec<Value>> for Value {
    /// Create a [Value] from a [Vec<Value>]
    fn from(value: Vec<Value>) -> Self {
        Value::Array(value)
    }
}

impl From<ValueMap> for Value {
    /// Create a [Value] from a [ValueMap]. (`hashbrown::HashMap` when not `preserve_order`, `indexmap::IndexMap` when `preserve_order`)
    fn from(value: ValueMap) -> Self {
        Value::Object(value)
    }
}

impl From<i64> for Value {
    /// Create a [Value] from an [i64]. Note that there is loss in precision because this value will be converted to [f64].
    fn from(value: i64) -> Self {
        Value::Number(Number::Int(value))
    }
}

/// Allows for indexing into a [Value] by [String] or [usize]
pub trait IndexOrKey {
    /// Get an immutable reference to a [Value].
    fn get(self, value: &Value) -> Option<&Value>;
    /// Get a mutable reference to a [Value].
    fn get_mut(self, value: &mut Value) -> Option<&mut Value>;
    /// Get a mutable reference or insert [Value::Null] and return a mutable reference to that.
    fn get_or_insert(self, value: &mut Value) -> &mut Value;
}

impl IndexOrKey for usize {
    /// Get an immutable reference to a [Value] in a [Value::Array].
    fn get(self, value: &Value) -> Option<&Value> {
        let Value::Array(array) = value else {
            return None;
        };
        array.get(self)
    }

    /// Get an immutable reference to a [Value] in a [Value::Array].
    fn get_mut(self, value: &mut Value) -> Option<&mut Value> {
        let Value::Array(array) = value else {
            return None;
        };
        array.get_mut(self)
    }

    /// Get a mutable refence to a [Value] in a [Value::Array]. This function will panic if
    /// the [Value] is not an array.
    fn get_or_insert(self, _value: &mut Value) -> &mut Value {
        let Value::Array(array) = _value else {
            panic!("Not an array.");
        };
        &mut array[self]
    }
}

impl IndexOrKey for &str {
    /// Get an immutable reference to a [Value] in a [Value::Object].
    fn get(self, value: &Value) -> Option<&Value> {
        let Value::Object(object) = value else {
            return None;
        };
        object.get(self)
    }

    /// Get a mutable reference to a [Value] in a [Value::Object].
    fn get_mut(self, value: &mut Value) -> Option<&mut Value> {
        let Value::Object(object) = value else {
            return None;
        };
        object.get_mut(self)
    }

    /// Get a mutable reference to a [Value] in a [Value::Object] if it exists, otherwise
    /// insert [Value::Null] and return a mutable reference to that.
    fn get_or_insert(self, value: &mut Value) -> &mut Value {
        if let Value::Null = value {
            *value = Value::Object(ValueMap::new());
        }
        let Value::Object(object) = value else {
            panic!("Not an object.");
        };
        object.entry(self.to_owned()).or_insert(Value::Null)
    }
}

impl IndexOrKey for String {
    /// Get an immutable reference to a [Value] in a [Value::Object].
    fn get(self, value: &Value) -> Option<&Value> {
        let Value::Object(object) = value else {
            return None;
        };
        object.get(&self)
    }

    /// Get a mutable reference to a [Value] in a [Value::Object].
    fn get_mut(self, value: &mut Value) -> Option<&mut Value> {
        let Value::Object(object) = value else {
            return None;
        };
        object.get_mut(&self)
    }

    /// Get a mutable reference to a [Value] in a [Value::Object] if it exists, otherwise
    /// insert [Value::Null] and return a mutable reference to that.
    fn get_or_insert(self, value: &mut Value) -> &mut Value {
        if let Value::Null = value {
            *value = Value::Object(ValueMap::new());
        }
        let Value::Object(object) = value else {
            panic!("Not an object.");
        };
        object.entry(self).or_insert(Value::Null)
    }
}

// By implementing InsertKey for String and &str, I can make Value::insert(k, v) generic for the key type.
pub trait InsertKey {
    fn insert_into(self, map: &mut ValueMap, value: Value) -> Option<Value>;
}

impl InsertKey for String {
    fn insert_into(self, map: &mut ValueMap, value: Value) -> Option<Value> {
        map.insert(self, value)
    }
}

impl InsertKey for &str {
    fn insert_into(self, map: &mut ValueMap, value: Value) -> Option<Value> {
        map.insert(self.to_owned(), value)
    }
}

impl Value {
    /// Push `value` into a [Value::Array]. If the [Value] is [Value::Null], convert it
    /// into a [Value::Array] and push `value` into it.
    /// 
    /// Panics if self [Value] is not [Value::Null] or [Value::Array].
    pub fn push<T: Into<Value>>(&mut self, value: T) {
        if let Value::Null = self {
            *self = Value::Array(Vec::new());
        }
        let Value::Array(array) = self else {
            panic!("Not an array.");
        };
        array.push(value.into());
    }

    /// Insert `value` into a [Value::Object]. If the [Value] is [Value::Null], convert it
    /// into a [Value::Array] and insert `value` into it.
    /// 
    /// Panics if self [Value] is not [Value::Null] or [Value::Array].
    pub fn insert<T: Into<Value>, K: InsertKey>(&mut self, k: K, v: T) -> Option<Value> {
        if let Value::Null = self {
            *self = Value::Object(ValueMap::new());
        }
        let Value::Object(object) = self else {
            panic!("Not an object.");
        };
        k.insert_into(object, v.into())
    }

    /// Get an immutable reference to a [Value] by index or key.
    pub fn get<I: IndexOrKey>(&self, i_k: I) -> Option<&Value> {
        i_k.get(self)
    }

    /// Get a mutable reference to a [Value] by index or key.
    pub fn get_mut<I: IndexOrKey>(&mut self, i_k: I) -> Option<&mut Value> {
        i_k.get_mut(self)
    }

    /// Get the length of the [Value] if it is one of the following variants:
    /// * [Value::String]
    /// * [Value::Array]
    /// * [Value::Object]
    pub fn len(&self) -> usize {
        match self {
            Value::String(string) => string.len(),
            Value::Array(array) => array.len(),
            Value::Object(object) => object.len(),
            _ => 0,
        }
    }
}

impl<I: IndexOrKey> std::ops::Index<I> for Value {
    type Output = Value;
    fn index(&self, index: I) -> &Self::Output {
        static NULL: Value = Value::Null;
        index.get(self).unwrap_or(&NULL)
    }
}

impl<I: IndexOrKey> std::ops::IndexMut<I> for Value {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        index.get_or_insert(self)
    }
}

pub trait ArrayExt {
    fn push_value<T: Into<Value>>(&mut self, value: T);
}

impl ArrayExt for Vec<Value> {
    fn push_value<T: Into<Value>>(&mut self, value: T) {
        self.push(value.into());
    }
}

pub trait ObjectExt {
    fn insert_value<T: Into<Value>>(&mut self, key: String, value: T) -> Option<Value>;
}

impl ObjectExt for ValueMap {
    fn insert_value<T: Into<Value>>(&mut self, k: String, v: T) -> Option<Value> {
        self.insert(k, v.into())
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    #[test]
    fn parse_number_test() -> Result<(), crate::error::ParseError> {
        let object = Value::from_str(r#"
            {
                "int": 9223372036854775807,
                "float": 3.14159265358979
            }
        "#)?;
        assert!(matches!(object["int"], Value::Number(Number::Int(i64::MAX))));
        assert!(matches!(object["float"], Value::Number(Number::Float(3.14159265358979))));
        let json_text = object.to_string();
        assert_eq!(json_text, r#"{"int":9223372036854775807,"float":3.14159265358979}"#);
        Ok(())
    }

    #[test]
    fn parse_pass_1() -> Result<(), crate::error::ParseError> {
        let _ = Value::from_str(include_str!("../tests/pass1.json"))?;
        Ok(())
    }

    #[test]
    fn parse_pass_2() -> Result<(), crate::error::ParseError> {
        let _ = Value::from_str(include_str!("../tests/pass2.json"))?;
        Ok(())
    }

    #[test]
    fn parse_pass_3() -> Result<(), crate::error::ParseError> {
        let _ = Value::from_str(include_str!("../tests/pass3.json"))?;
        Ok(())
    }

    #[test]
    #[ignore = "This check is not yet implemented."]
    fn test_fail_1() {
        let result = Value::from_str(include_str!("../tests/fail1.json"));
        assert!(result.is_err(), "A JSON payload should be an object or array, not a string.");
    }

    #[test]
    fn test_fail_2() {
        let result = Value::from_str(include_str!("../tests/fail2.json"));
        assert!(result.is_err(), "Unclosed array");
    }

    #[test]
    fn test_fail_3() {
        let result = Value::from_str(include_str!("../tests/fail3.json"));
        assert!(result.is_err(), "keys must be quoted");
    }

    #[test]
    fn test_fail_4() {
        let result = Value::from_str(include_str!("../tests/fail4.json"));
        assert!(result.is_err(), "extra comma");
    }

    #[test]
    fn test_fail_5() {
        let result = Value::from_str(include_str!("../tests/fail5.json"));
        assert!(result.is_err(), "double extra comma");
    }

    #[test]
    fn test_fail_6() {
        let result = Value::from_str(include_str!("../tests/fail6.json"));
        assert!(result.is_err(), "missing value before comma");
    }

    #[test]
    fn test_fail_7() {
        let result = Value::from_str(include_str!("../tests/fail7.json"));
        assert!(result.is_err(), "Comma after the close");
    }

    #[test]
    fn test_fail_8() {
        let result = Value::from_str(include_str!("../tests/fail8.json"));
        assert!(result.is_err(), "Extra close");
    }

    #[test]
    fn test_fail_9() {
        let result = Value::from_str(include_str!("../tests/fail9.json"));
        assert!(result.is_err(), "Extra comma");
    }

    #[test]
    fn test_fail_10() {
        let result = Value::from_str(include_str!("../tests/fail10.json"));
        assert!(result.is_err(), "Extra value after close");
    }

    #[test]
    fn test_fail_11() {
        let result = Value::from_str(include_str!("../tests/fail11.json"));
        assert!(result.is_err(), "Illegal expression");
    }

    #[test]
    fn test_fail_12() {
        let result = Value::from_str(include_str!("../tests/fail12.json"));
        assert!(result.is_err(), "Illegal invocation");
    }

    #[test]
    fn test_fail_13() {
        let result = Value::from_str(include_str!("../tests/fail13.json"));
        assert!(result.is_err(), "Numbers cannot have leading zeroes");
    }

    #[test]
    fn test_fail_14() {
        let result = Value::from_str(include_str!("../tests/fail14.json"));
        assert!(result.is_err(), "Numbers cannot be hex");
    }

    #[test]
    #[ignore = "This is explicitly allowed by this parser"]
    fn test_fail_15() {
        let result = Value::from_str(include_str!("../tests/fail15.json"));
        assert!(result.is_err(), "Illegal backslash escape");
    }

    #[test]
    fn test_fail_16() {
        let result = Value::from_str(include_str!("../tests/fail16.json"));
        assert!(result.is_err(), "Text without quotes");
    }

    #[test]
    #[ignore = "This is explicitly allowed by this parser"]
    fn test_fail_17() {
        let result = Value::from_str(include_str!("../tests/fail17.json"));
        assert!(result.is_err(), "Illegal backslash escape");
    }

    #[test]
    #[ignore = "Need to be able to specify the maximum depth"]
    fn test_fail_18() {
        let result = Value::from_str(include_str!("../tests/fail18.json"));
        assert!(result.is_err(), "Too deep");
    }

    #[test]
    fn test_fail_19() {
        let result = Value::from_str(include_str!("../tests/fail19.json"));
        assert!(result.is_err(), "Missing colon");
    }

    #[test]
    fn test_fail_20() {
        let result = Value::from_str(include_str!("../tests/fail20.json"));
        assert!(result.is_err(), "Double colon");
    }

    #[test]
    fn test_fail_21() {
        let result = Value::from_str(include_str!("../tests/fail21.json"));
        assert!(result.is_err(), "Comma instead of colon");
    }

    #[test]
    fn test_fail_22() {
        let result = Value::from_str(include_str!("../tests/fail22.json"));
        assert!(result.is_err(), "Colon instead of comma");
    }

    #[test]
    fn test_fail_23() {
        let result = Value::from_str(include_str!("../tests/fail23.json"));
        assert!(result.is_err(), "Bad value");
    }

    #[test]
    fn test_fail_24() {
        let result = Value::from_str(include_str!("../tests/fail24.json"));
        assert!(result.is_err(), "Single quote");
    }

    #[test]
    #[ignore = "This is explicitly allowed by this parser"]
    fn test_fail_25() {
        let result = Value::from_str(include_str!("../tests/fail25.json"));
        assert!(result.is_err(), "Tab character in string");
    }

    #[test]
    #[ignore = "This is explicitly allowed by this parser"]
    fn test_fail_26() {
        let result = Value::from_str(include_str!("../tests/fail26.json"));
        assert!(result.is_err(), "Escaped tab character in string");
    }

    #[test]
    fn test_fail_27() {
        let result = Value::from_str(include_str!("../tests/fail27.json"));
        assert!(result.is_err(), "Newline in string");
    }

    #[test]
    fn test_fail_28() {
        let result = Value::from_str(include_str!("../tests/fail28.json"));
        assert!(result.is_err(), "Escaped newline in string");
    }

    #[test]
    fn test_fail_29() {
        let result = Value::from_str(include_str!("../tests/fail29.json"));
        assert!(result.is_err(), "Unexpected end of number");
    }

    #[test]
    fn test_fail_30() {
        let result = Value::from_str(include_str!("../tests/fail30.json"));
        assert!(result.is_err(), "Unexpected end of number");
    }

    #[test]
    fn test_fail_31() {
        let result = Value::from_str(include_str!("../tests/fail31.json"));
        assert!(result.is_err(), "Double signed exponent");
    }

    #[test]
    fn test_fail_32() {
        let result = Value::from_str(include_str!("../tests/fail32.json"));
        assert!(result.is_err(), "Comma instead of close curly brace");
    }

    #[test]
    fn test_fail_33() {
        let result = Value::from_str(include_str!("../tests/fail33.json"));
        assert!(result.is_err(), "Mismatched close curly brace");
    }
}