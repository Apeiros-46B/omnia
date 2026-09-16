use std::str::FromStr;

use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum PredicateAtom<'a> {
    Flag { key: &'a str, val: bool },
    Comparison {
        key: &'a str,
        cmp: Comparator,
        val: &'a str,
    },
}

impl<'a> PredicateAtom<'a> {
    pub fn parse(src: &'a str) -> Result<Self, PredicateParseError> {
        AtomParser::new(src)?.parse()
    }
}

#[derive(Error, Clone, Debug)]
pub enum PredicateParseError {
    #[error("expected input")]
    InputEmpty,
    #[error("invalid key")]
    InvalidKey,
    #[error("invalid comparator")]
    InvalidComparator,
    #[error("invalid value")]
    InvalidValue,
    #[error("cannot negate comparison predicate")]
    InvalidNegation,
    #[error("expected end of input")]
    ExpectedEnd,
}

pub fn parse_val<T: FromStr>(val: &str) -> Result<T, PredicateParseError> {
    val.parse().map_err(|_| PredicateParseError::InvalidValue)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Comparator {
    Lt,
    Lte,
    Eq,
    Neq,
    Gte,
    Gt,
}

impl Comparator {
    pub fn cmp<T: PartialEq + PartialOrd>(&self, x: T, y: T) -> bool {
        match self {
            Self::Lt  => x < y,
            Self::Lte => x <= y,
            Self::Eq  => x == y,
            Self::Neq => x != y,
            Self::Gte => x >= y,
            Self::Gt  => x > y,
        }
    }

    fn negate(self) -> Self {
        match self {
            Self::Lt  => Self::Gte,
            Self::Lte => Self::Gt,
            Self::Eq  => Self::Neq,
            Self::Neq => Self::Eq,
            Self::Gte => Self::Lt,
            Self::Gt  => Self::Lte,
        }
    }

    fn valid_ch(b: &u8) -> bool {
        match b {
            b'<' => true,
            b'>' => true,
            b'=' => true,
            b'!' => true,
            b':' => true,
            _    => false,
        }
    }

    fn from_bytes(s: &[u8]) -> Result<Self, PredicateParseError> {
        match s {
            b"<"  => Ok(Comparator::Lt),
            b"<=" => Ok(Comparator::Lte),
            b"="  => Ok(Comparator::Eq),
            b"!=" => Ok(Comparator::Neq),
            b">=" => Ok(Comparator::Gte),
            b">"  => Ok(Comparator::Gt),
            b":"  => Ok(Comparator::Eq), // special case
            _     => Err(PredicateParseError::InvalidComparator),
        }
    }
}

struct AtomParser<'a> {
    src: &'a [u8],
    cur: usize,
}

impl<'a> AtomParser<'a> {
    fn new(src: &'a str) -> Result<Self, PredicateParseError> {
        if src.is_empty() {
            return Err(PredicateParseError::InputEmpty);
        }
        Ok(Self {
            src: src.trim().as_bytes(),
            cur: 0,
        })
    }

    fn consume<T>(
        &mut self,
        p: impl Fn(&u8) -> bool,
        f: impl FnOnce(&'a [u8]) -> T,
    ) -> T {
        let start = self.cur;

        while self.cur < self.src.len() && p(&self.src[self.cur]) {
            self.cur += 1;
        }

        f(&self.src[start..self.cur])
    }

    fn key_valid_ch(b: &u8) -> bool {
        b.is_ascii_lowercase()
    }

    fn val_valid_ch(b: &u8) -> bool {
        b.is_ascii_alphanumeric() || *b == b'-' || *b == b'.'
    }

    fn parse(mut self) -> Result<PredicateAtom<'a>, PredicateParseError> {
        let bangs = self.consume(|c| *c == b'!', |s| s.len());

        let key = self.consume(Self::key_valid_ch, str::from_utf8).unwrap();
        if key.is_empty() {
            return Err(PredicateParseError::InvalidKey);
        }
        if self.cur >= self.src.len() {
            return Ok(PredicateAtom::Flag {
                key,
                val: bangs % 2 == 0,
            });
        }

        let cmp = self.consume(Comparator::valid_ch, Comparator::from_bytes)?;
        if bangs != 0 {
            return Err(PredicateParseError::InvalidNegation);
        }

        let val = self.consume(Self::val_valid_ch, str::from_utf8).unwrap();
        if val.is_empty() {
            return Err(PredicateParseError::InvalidValue);
        }

        if self.cur >= self.src.len() {
            return Ok(PredicateAtom::Comparison { key, cmp, val });
        } else {
            return Err(PredicateParseError::ExpectedEnd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Comparator, PredicateAtom};

    #[test]
    fn parse_flag() {
        assert_eq!(
            PredicateAtom::parse("abcxyz").unwrap(),
            PredicateAtom::Flag { key: "abcxyz", val: true },
        );
        assert_eq!(
            PredicateAtom::parse("!foobar").unwrap(),
            PredicateAtom::Flag { key: "foobar", val: false },
        );
        assert_eq!(
            PredicateAtom::parse("!!tspmo").unwrap(),
            PredicateAtom::Flag { key: "tspmo", val: true },
        );
    }

    #[test]
    fn parse_comparison() {
        assert_eq!(
            PredicateAtom::parse("rating!=safe").unwrap(),
            PredicateAtom::Comparison {
                key: "rating",
                cmp: Comparator::Neq,
                val: "safe",
            },
        );
        assert_eq!(
            PredicateAtom::parse("score<-10").unwrap(),
            PredicateAtom::Comparison {
                key: "score",
                cmp: Comparator::Lt,
                val: "-10",
            },
        );
        assert_eq!(
            PredicateAtom::parse("uses=1").unwrap(),
            PredicateAtom::Comparison {
                key: "uses",
                cmp: Comparator::Eq,
                val: "1",
            },
        );
        assert_eq!(
            PredicateAtom::parse("size>=1.5m").unwrap(),
            PredicateAtom::Comparison {
                key: "size",
                cmp: Comparator::Gte,
                val: "1.5m",
            },
        );
        assert_eq!(
            PredicateAtom::parse("collection:abc").unwrap(),
            PredicateAtom::Comparison {
                key: "collection",
                cmp: Comparator::Eq,
                val: "abc",
            },
        );
    }

    #[test]
    fn parse_malformed() {
        // bad string
        assert!(PredicateAtom::parse("").is_err());
        assert!(PredicateAtom::parse("   ").is_err());

        // invalid key
        assert!(PredicateAtom::parse("hello world=42").is_err());
        assert!(PredicateAtom::parse("71258=67").is_err());
        assert!(PredicateAtom::parse("💀").is_err());
        assert!(PredicateAtom::parse("rating💀=safe").is_err());
        assert!(PredicateAtom::parse("abc💀").is_err());

        // invalid comparator
        assert!(PredicateAtom::parse("abc&&def").is_err());
        assert!(PredicateAtom::parse("abc*def").is_err());
        assert!(PredicateAtom::parse("abc%def").is_err());
        assert!(PredicateAtom::parse("!score=10").is_err());

        // invalid value
        assert!(PredicateAtom::parse("foobar=spa ce").is_err());
        assert!(PredicateAtom::parse("foobar=hi!").is_err());
        assert!(PredicateAtom::parse("rating=safé").is_err());

        // generally malformed
        assert!(PredicateAtom::parse("foo=bar=baz").is_err());
    }
}
