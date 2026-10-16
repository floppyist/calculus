use std::fmt;

// An exact fraction in lowest terms with a positive denominator
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rational {
    numerator: i128,
    denominator: i128,
}

impl Rational {
    pub fn from_integer(value: i128) -> Rational {
        Rational {
            numerator: value,
            denominator: 1,
        }
    }

    // Builds a reduced fraction; None when the denominator is zero or overflows
    pub fn new(numerator: i128, denominator: i128) -> Option<Rational> {
        if denominator == 0 {
            return None;
        }

        let sign = if denominator < 0 { -1 } else { 1 };
        let numerator = numerator.checked_mul(sign)?;
        let denominator = denominator.checked_mul(sign)?;
        let divisor = gcd(numerator, denominator);

        Some(Rational {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    // Reads an f64 that came from a decimal literal, keeping its exact value
    pub fn from_decimal(value: f64) -> Option<Rational> {
        if !value.is_finite() {
            return None;
        }

        if value == value.trunc() && value.abs() < 1e18 {
            return Some(Rational::from_integer(value as i128));
        }

        parse_decimal(&format!("{}", value))
    }

    pub fn add(self, other: Rational) -> Option<Rational> {
        let numerator = self
            .numerator
            .checked_mul(other.denominator)?
            .checked_add(other.numerator.checked_mul(self.denominator)?)?;

        Rational::new(numerator, self.denominator.checked_mul(other.denominator)?)
    }

    pub fn sub(self, other: Rational) -> Option<Rational> {
        self.add(other.neg())
    }

    pub fn mul(self, other: Rational) -> Option<Rational> {
        Rational::new(
            self.numerator.checked_mul(other.numerator)?,
            self.denominator.checked_mul(other.denominator)?,
        )
    }

    pub fn div(self, other: Rational) -> Option<Rational> {
        Rational::new(
            self.numerator.checked_mul(other.denominator)?,
            self.denominator.checked_mul(other.numerator)?,
        )
    }

    pub fn neg(self) -> Rational {
        Rational {
            numerator: -self.numerator,
            denominator: self.denominator,
        }
    }

    // Raises to an integer power, None on overflow or a zero negative base
    pub fn pow(self, exponent: i128) -> Option<Rational> {
        if exponent.unsigned_abs() > 128 {
            return None;
        }

        let mut result = Rational::from_integer(1);
        let base = if exponent < 0 {
            Rational::new(self.denominator, self.numerator)?
        } else {
            self
        };

        for _ in 0..exponent.unsigned_abs() {
            result = result.mul(base)?;
        }

        Some(result)
    }

    pub fn is_integer(self) -> bool {
        self.denominator == 1
    }

    pub fn to_integer(self) -> Option<i128> {
        if self.denominator == 1 {
            Some(self.numerator)
        } else {
            None
        }
    }

    pub fn numerator(self) -> i128 {
        self.numerator
    }

    pub fn denominator(self) -> i128 {
        self.denominator
    }

    pub fn to_f64(self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }
}

impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.denominator == 1 {
            write!(f, "{}", self.numerator)
        } else {
            write!(f, "{}/{}", self.numerator, self.denominator)
        }
    }
}

fn gcd(left: i128, right: i128) -> i128 {
    let mut left = left.unsigned_abs();
    let mut right = right.unsigned_abs();

    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }

    if left == 0 { 1 } else { left as i128 }
}

// Parses a plain decimal string such as "0.1", "-2.5" or "1e-3"
fn parse_decimal(text: &str) -> Option<Rational> {
    let (negative, text) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };

    let (mantissa, exponent) = match text.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, exponent.parse::<i32>().ok()?),
        None => (text, 0),
    };

    let (integer, fraction) = match mantissa.split_once('.') {
        Some((integer, fraction)) => (integer, fraction),
        None => (mantissa, ""),
    };

    let mut numerator = 0i128;
    let mut exponent = exponent;

    for character in integer.chars().chain(fraction.chars()) {
        let digit = character.to_digit(10)? as i128;

        numerator = numerator.checked_mul(10)?.checked_add(digit)?;
    }

    exponent -= fraction.chars().filter(|c| c.is_ascii_digit()).count() as i32;

    if exponent >= 0 {
        numerator = numerator.checked_mul(10i128.checked_pow(exponent as u32)?)?;
    }

    let denominator = 10i128.checked_pow((-exponent) as u32)?;

    if negative {
        numerator = numerator.checked_neg()?;
    }

    Rational::new(numerator, denominator)
}
