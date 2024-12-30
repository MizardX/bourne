// Why did the digital archaeologist get excited about old software?
// Because they loved discovering ancient "bits" of history!
use std::str::FromStr;

use crate::{error::ParseError, Number, Value, ValueMap};

/// Result returned from JSON parsing.
pub type ParseResult<T> = Result<T, ParseError>;

/// Convert a hexadecimal character into a u16.
fn hex_value(chr: char) -> Option<u16> {
    match chr {
        '0'..='9' => Some(chr as u16 - '0' as u16),
        // The reason for subtracting 'W' is because 'W' is 10 less than 'a',
        // allowing to not have to add 10 to the result of chr - 'a'.
        'a'..='f' => Some(chr as u16 - 'W' as u16),
        // Same reasoning as above for subtracting 7.
        'A'..='F' => Some(chr as u16 - '7' as u16),
        _ => None,
    }
}

/// Unescape a string.
pub fn unescape_string<S: AsRef<str>>(string: S) -> ParseResult<String> {
    let s = string.as_ref();
    let mut buffer = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            buffer.push(c);
            continue;
        }
        buffer.push(match chars.next() {
            Some('f') => '\u{000c}',
            Some('b') => '\u{0008}',
            Some('n') => '\n',
            Some('r') => '\r',
            Some('t') => '\t',
            Some('u') => {
                // Read 4 hex-digits
                let mut hex: u16 = 0;
                for i in 0..4 {
                    let Some(digit) = chars.next() else {
                        return Err(ParseError::UnexpectedEOF);
                    };
                    let Some(value) = hex_value(digit) else {
                        return Err(ParseError::InvalidHex);
                    };
                    hex |= value;
                    // Do not shift if it's the last cycle.
                    if i < 3 {
                        hex <<= 4;
                    }
                }
                let Some(res) = char::from_u32(hex as u32) else {
                    return Err(ParseError::InvalidEscapeSequence);
                };
                res
            }
            // If the character is any other character, just return the character.
            // This allows to unescape \< to < without having to be explicit.
            // Also, I just think it's a good idea to unescape any character.
            Some(other) => other,
            None => return Err(ParseError::UnexpectedEOF),
        });
    }
    Ok(buffer)
}

/// A JSON parser.
#[derive(Debug, Clone, Copy)]
struct Parser<'a> {
    source: &'a str,
    index: usize,
}

impl<'a> Parser<'a> {
    /// Create a new [Parser] from a `source` string.
    fn new(source: &'a str) -> Self {
        Self { source, index: 0 }
    }

    /// Checks if the index is at the end of the stream.
    fn is_eof(&self) -> bool {
        self.index >= self.source.len()
    }

    /// Takes a look at the next byte in the stream without advancing the index.
    fn peek(&self) -> Option<u8> {
        if self.index < self.source.len() {
            Some(self.source.as_bytes()[self.index])
        } else {
            None
        }
    }

    /// Retrieve the next byte paired with its index, advancing the parser in the process.
    fn indexed_next(&mut self) -> Option<(usize, u8)> {
        if self.index < self.source.len() {
            let res = Some((self.index, self.source.as_bytes()[self.index]));
            self.index += 1;
            res
        } else {
            None
        }
    }

    /// Retrieve the next byte, advancing the parser in the process.
    fn next(&mut self) -> Option<u8> {
        if self.index < self.source.len() {
            let res = Some(self.source.as_bytes()[self.index]);
            self.index += 1;
            res
        } else {
            None
        }
    }

    /// Advance the index by `step`.
    fn advance(&mut self, step: usize) {
        self.index += step;
    }

    /// Decrement the index by one.
    fn rewind(&mut self) {
        self.index = self.index.checked_sub(1).unwrap_or(0);
    }

    /// Checks if the parser matches text at the current index.
    fn matches<S: AsRef<str>>(&mut self, text: S) -> bool {
        let s = text.as_ref();
        if self.index + s.len() <= self.source.len() {
            self.source[self.index..].starts_with(s)
        } else {
            false
        }
    }

    /// Consumes all whitespace, advancing the index.
    fn eat_whitespace(&mut self) {
        while let Some(peek) = self.peek() {
            if peek.is_ascii_whitespace() {
                self.advance(1);
            } else {
                break;
            }
        }
    }

    /// Parse the `null` keyword and return [Value::Null] on success.
    fn parse_null(&mut self) -> ParseResult<Value> {
        if self.matches("null") {
            self.advance(4);
            Ok(Value::Null)
        } else {
            Err(ParseError::InvalidCharacter(self.index))
        }
    }

    /// Parse `true` or `false` keywords into [bool].
    fn parse_boolean(&mut self) -> ParseResult<bool> {
        if self.matches("true") {
            self.advance(4);
            Ok(true)
        } else if self.matches("false") {
            self.advance(5);
            Ok(false)
        } else {
            Err(ParseError::InvalidCharacter(self.index))
        }
    }

    /// Parse a [Number].
    fn parse_number(&mut self) -> ParseResult<Number> {
        // Valid characters that can follow a number: '}', ']', ',', and whitespace.
        // Pattern for parsing numbers:
        // [+|-]?                   -> Optional sign (+ or -)
        // ( 0 | [1-9] [0-9]* )     -> Integer part: either a single zero or a non-zero digit followed by any number of digits
        // ( . [0-9]+ )?            -> Optional fractional part: a dot followed by one or more digits
        // ( [e|E] [+|-]? [0-9]+ )? -> Optional exponent part: 'e' or 'E', optional sign, followed by one or more digits
        enum State {
            /// Before everything.
            Start,
            /// Inside the integer part.
            AfterSign,
            /// After a leading zero. No more digits may follow.
            AfterZero,
            /// Inside the intger part.
            IntegerPart,
            /// After the decimal point. Must have at least one digit.
            AfterDecimalPoint,
            /// Inside the fractional part.
            FractionalPart,
            /// After the exponent character. Must have at least one digit.
            AfterExponent,
            /// After the exponent sign.
            AfterExponentSign,
            /// Inside the exponent part.
            ExponentPart,
        }
        let mut is_integer = true;
        let mut state = State::Start;
        let start = self.index;
        let mut end = self.index;
        while let Some((index, next)) = self.indexed_next() {
            match state {
                State::Start => match next {
                    b'+' | b'-' => {
                        state = State::AfterSign;
                    }
                    b'0' => {
                        state = State::AfterZero;
                    }
                    b'1'..=b'9' => {
                        state = State::IntegerPart;
                    }
                    _ => {
                        return Err(ParseError::InvalidCharacter(index));
                    }
                },
                State::AfterSign => match next {
                    b'0' => {
                        state = State::AfterZero;
                    }
                    b'1'..=b'9' => {
                        state = State::IntegerPart;
                    }
                    _ => {
                        return Err(ParseError::InvalidCharacter(index));
                    }
                },
                State::AfterZero => match next {
                    b'.' => {
                        state = State::AfterDecimalPoint;
                        is_integer = false;
                    }
                    b'e' | b'E' => {
                        state = State::AfterExponent;
                        is_integer = false;
                    }
                    b'}' | b']' | b',' => {
                        end = index;
                        self.rewind();
                        break;
                    }
                    ws if ws.is_ascii_whitespace() => {
                        end = index;
                        self.rewind();
                        break;
                    }
                    _ => {
                        return Err(ParseError::InvalidCharacter(index));
                    }
                },
                State::IntegerPart => match next {
                    b'0'..=b'9' => (),
                    b'.' => {
                        state = State::AfterDecimalPoint;
                        is_integer = false;
                    }
                    b'e' | b'E' => {
                        state = State::AfterExponent;
                        is_integer = false;
                    }
                    b'}' | b']' | b',' => {
                        end = index;
                        self.rewind();
                        break;
                    }
                    ws if ws.is_ascii_whitespace() => {
                        end = index;
                        self.rewind();
                        break;
                    }
                    _ => {
                        return Err(ParseError::InvalidCharacter(index));
                    }
                },
                State::AfterDecimalPoint => match next {
                    b'0'..=b'9' => {
                        state = State::FractionalPart;
                    }
                    _ => {
                        return Err(ParseError::InvalidCharacter(index));
                    }
                },
                State::FractionalPart => match next {
                    b'0'..=b'9' => (),
                    b'e' | b'E' => {
                        state = State::AfterExponent;
                    }
                    b'}' | b']' | b',' => {
                        end = index;
                        self.rewind();
                        break;
                    }
                    ws if ws.is_ascii_whitespace() => {
                        end = index;
                        self.rewind();
                        break;
                    }
                    _ => {
                        return Err(ParseError::InvalidCharacter(index));
                    }
                },
                State::AfterExponent => match next {
                    b'+' | b'-' => {
                        state = State::AfterExponentSign;
                    }
                    b'0'..=b'9' => {
                        state = State::ExponentPart;
                    }
                    _ => {
                        return Err(ParseError::InvalidCharacter(index));
                    }
                },
                State::AfterExponentSign => match next {
                    b'0'..=b'9' => {
                        state = State::ExponentPart;
                    }
                    _ => {
                        return Err(ParseError::InvalidCharacter(index));
                    }
                },
                State::ExponentPart => match next {
                    b'0'..=b'9' => (),
                    b'}' | b']' | b',' => {
                        end = index;
                        self.rewind();
                        break;
                    }
                    ws if ws.is_ascii_whitespace() => {
                        end = index;
                        self.rewind();
                        break;
                    }
                    _ => {
                        return Err(ParseError::InvalidCharacter(index));
                    }
                },
            }
        }
        let number = &self.source[start..end];
        if number.is_empty() {
            return Err(ParseError::InvalidCharacter(self.index));
        }
        Ok(if is_integer {
            Number::Int(number.parse::<i64>()?)
        } else {
            Number::Float(number.parse::<f64>()?)
        })
    }

    /// Parse a string between double quotes (`"`).
    ///
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
    ///
    /// #### Example:
    /// ```json
    /// "Hello, world!"
    /// ```
    fn parse_string(&mut self) -> ParseResult<String> {
        match self.peek() {
            Some(b'"') => {
                self.next();
            }
            Some(_) => {
                return Err(ParseError::InvalidCharacter(self.index));
            }
            None => {
                return Err(ParseError::UnexpectedEOF);
            }
        }
        let start = self.index;
        let string = loop {
            let Some((index, next)) = self.indexed_next() else {
                return Err(ParseError::UnexpectedEOFWhileParsingString(start));
            };
            match next {
                // Strings should not contain new-lines.
                b'\n' | b'\r' => {
                    return Err(ParseError::LineBreakWhileParsingString(index));
                }
                b'"' => break unescape_string(&self.source[start..index])?,
                b'\\' => {
                    self.advance(1);
                }
                _ => {}
            }
        };
        Ok(string)
    }

    /// Parse a JSON Array (JSON values in comma separated list between `[` and `]`).  
    /// Example:
    /// ```json
    /// [
    ///     true,
    ///     false,
    ///     null,
    ///     3.14,
    ///     "Hello, world!",
    ///     [1, 2, 3],
    ///     {
    ///         "example" : "The quick brown fox jumps over the lazy dog."
    ///     }
    /// ]
    /// ```
    fn parse_array(&mut self) -> ParseResult<Vec<Value>> {
        match self.indexed_next() {
            Some((_, b'[')) => (),
            Some((index, _)) => return Err(ParseError::InvalidCharacter(index)),
            None => return Err(ParseError::UnexpectedEOF),
        }
        let mut array = Vec::new();
        loop {
            self.eat_whitespace();
            match self.peek() {
                Some(b']') if array.is_empty() => {
                    self.advance(1);
                    break;
                }
                Some(b']' | b',') => return Err(ParseError::UnexpectedCommaInArrayOrObject(self.index)),
                Some(_) => {
                    array.push(self.parse_value()?);
                    self.eat_whitespace();
                    match self.indexed_next() {
                        Some((_, b']')) => break,
                        Some((_, b',')) => continue,
                        Some((index, _)) => return Err(ParseError::InvalidCharacter(index)),
                        None => return Err(ParseError::UnexpectedEOF),
                    }
                }
                None => return Err(ParseError::UnexpectedEOF),
            }
        }
        Ok(array)
    }

    /// Parse a JSON Object.
    ///
    /// #### Example:
    /// ```json
    /// {
    ///     "null" : null,
    ///     "boolean_array" : [false, true],
    ///     "number" : 3.14159265358979,
    ///     "string" : "Hello, world!",
    /// }
    /// ```
    fn parse_object(&mut self) -> ParseResult<ValueMap> {
        match self.indexed_next() {
            Some((_, b'{')) => (),
            Some((index, _)) => return Err(ParseError::InvalidCharacter(index)),
            None => return Err(ParseError::UnexpectedEOF),
        }
        let mut map = ValueMap::new();
        loop {
            self.eat_whitespace();
            match self.peek() {
                Some(b'"') => {
                    let key = self.parse_string()?;
                    self.eat_whitespace();
                    match self.indexed_next() {
                        Some((_, b':')) => (),
                        Some((index, _)) => return Err(ParseError::InvalidCharacter(index)),
                        None => return Err(ParseError::UnexpectedEOF),
                    }
                    self.eat_whitespace();
                    let value = self.parse_value()?;
                    map.insert(key, value);
                    self.eat_whitespace();
                    match self.indexed_next() {
                        Some((_, b',')) => continue,
                        Some((_, b'}')) => break,
                        Some((index, _)) => return Err(ParseError::InvalidCharacter(index)),
                        None => return Err(ParseError::UnexpectedEOF),
                    }
                }
                Some(b'}') if map.is_empty() => {
                    self.next();
                    break;
                }
                Some(b'}' | b',') => return Err(ParseError::UnexpectedCommaInArrayOrObject(self.index)),
                Some(_) => return Err(ParseError::InvalidCharacter(self.index)),
                None => return Err(ParseError::UnexpectedEOF),
            }
        }
        Ok(map)
    }

    /// Parse a JSON Value.
    fn parse_value(&mut self) -> ParseResult<Value> {
        Ok(match self.peek() {
            Some(b'n') => self.parse_null()?,
            Some(b't' | b'f') => Value::Boolean(self.parse_boolean()?),
            Some(b'+' | b'-' | b'0'..=b'9') => Value::Number(self.parse_number()?),
            Some(b'"') => Value::String(self.parse_string()?),
            Some(b'[') => Value::Array(self.parse_array()?),
            Some(b'{') => Value::Object(self.parse_object()?),
            Some(_) => return Err(ParseError::InvalidCharacter(self.index)),
            None => return Err(ParseError::UnexpectedEOF),
        })
    }
}

impl FromStr for Value {
    type Err = ParseError;
    /// Parse a JSON [Value] from a string.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parser = Parser::new(s);
        parser.eat_whitespace();
        let res = parser.parse_value()?;
        parser.eat_whitespace();
        if !parser.is_eof() {
            Err(ParseError::InvalidCharacter(parser.index))
        } else {
            Ok(res)
        }
    }
}
