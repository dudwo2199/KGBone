//! 숫자 입력칸 수식.
//!
//! - `v`, `value`: 현재 값(대소문자 무시)
//! - `+=5`, `-=5`, `*=2`, `/=2`: 현재 값에 연산(오른쪽은 수식)
//! - 사칙연산 `+ - * /`, 나머지 `%`, 괄호, 단항 `-`/`+`, 소수·지수 표기(`1.5e2`)
use crate::{CoreError, Result};

/// 입력 글자 `input`을 현재 값 `current` 기준으로 계산한다.
pub fn eval(input: &str, current: f64) -> Result<f64> {
    let s = input.trim();
    if s.is_empty() {
        return Err(bad("수식이 비어 있습니다"));
    }
    let result = match s.get(..2) {
        Some(op @ ("+=" | "-=" | "*=" | "/=")) => {
            let rhs = parse(&s[2..], current)?;
            match op {
                "+=" => current + rhs,
                "-=" => current - rhs,
                "*=" => current * rhs,
                _ => divide(current, rhs)?,
            }
        }
        _ => parse(s, current)?,
    };
    if !result.is_finite() {
        return Err(bad("계산 결과가 숫자가 아닙니다"));
    }
    Ok(result)
}

fn bad(msg: &str) -> CoreError {
    CoreError::InvalidArgs(format!("수식: {msg}"))
}

fn divide(a: f64, b: f64) -> Result<f64> {
    if b == 0.0 {
        return Err(bad("0으로 나눌 수 없습니다"));
    }
    Ok(a / b)
}

fn parse(text: &str, current: f64) -> Result<f64> {
    let mut p = Parser { s: text.as_bytes(), i: 0, current };
    let v = p.expr()?;
    p.skip_ws();
    if p.i != p.s.len() {
        return Err(bad("알 수 없는 글자가 있습니다"));
    }
    Ok(v)
}

/// 재귀 하강 계산기: expr = term (('+'|'-') term)*, term = unary (('*'|'/'|'%') unary)*,
/// unary = ('-'|'+') unary | primary, primary = 숫자 | v | value | '(' expr ')'.
struct Parser<'a> {
    s: &'a [u8],
    i: usize,
    current: f64,
}

impl Parser<'_> {
    fn skip_ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.s.get(self.i).copied()
    }

    fn expr(&mut self) -> Result<f64> {
        let mut v = self.term()?;
        while let Some(c @ (b'+' | b'-')) = self.peek() {
            self.i += 1;
            let r = self.term()?;
            v = if c == b'+' { v + r } else { v - r };
        }
        Ok(v)
    }

    fn term(&mut self) -> Result<f64> {
        let mut v = self.unary()?;
        while let Some(c @ (b'*' | b'/' | b'%')) = self.peek() {
            self.i += 1;
            let r = self.unary()?;
            v = match c {
                b'*' => v * r,
                b'/' => divide(v, r)?,
                _ => {
                    if r == 0.0 {
                        return Err(bad("0으로 나눌 수 없습니다"));
                    }
                    v % r
                }
            };
        }
        Ok(v)
    }

    fn unary(&mut self) -> Result<f64> {
        match self.peek() {
            Some(b'-') => {
                self.i += 1;
                Ok(-self.unary()?)
            }
            Some(b'+') => {
                self.i += 1;
                self.unary()
            }
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Result<f64> {
        match self.peek() {
            Some(b'(') => {
                self.i += 1;
                let v = self.expr()?;
                if self.peek() != Some(b')') {
                    return Err(bad("닫는 괄호가 없습니다"));
                }
                self.i += 1;
                Ok(v)
            }
            Some(c) if c.is_ascii_digit() || c == b'.' => self.number(),
            Some(c) if c.is_ascii_alphabetic() => {
                let start = self.i;
                while self.i < self.s.len() && self.s[self.i].is_ascii_alphabetic() {
                    self.i += 1;
                }
                let word = std::str::from_utf8(&self.s[start..self.i]).unwrap_or("").to_ascii_lowercase();
                match word.as_str() {
                    "v" | "value" => Ok(self.current),
                    _ => Err(bad("알 수 없는 이름입니다(v 또는 value만 쓸 수 있습니다)")),
                }
            }
            Some(_) => Err(bad("알 수 없는 글자가 있습니다")),
            None => Err(bad("식이 끝나지 않았습니다")),
        }
    }

    fn number(&mut self) -> Result<f64> {
        let start = self.i;
        let digits = |p: &mut Self| {
            while p.i < p.s.len() && p.s[p.i].is_ascii_digit() {
                p.i += 1;
            }
        };
        digits(self);
        if self.s.get(self.i) == Some(&b'.') {
            self.i += 1;
            digits(self);
        }
        if matches!(self.s.get(self.i), Some(b'e' | b'E')) {
            let save = self.i;
            self.i += 1;
            if matches!(self.s.get(self.i), Some(b'+' | b'-')) {
                self.i += 1;
            }
            if self.s.get(self.i).is_some_and(u8::is_ascii_digit) {
                digits(self);
            } else {
                self.i = save;
            }
        }
        let text = std::str::from_utf8(&self.s[start..self.i]).unwrap_or("");
        text.parse::<f64>().map_err(|_| bad("숫자 형식이 잘못되었습니다"))
    }
}

#[cfg(test)]
mod tests {
    use super::eval;
    use crate::CoreError;

    fn ok(input: &str, current: f64) -> f64 {
        eval(input, current).unwrap_or_else(|e| panic!("{input}: {e}"))
    }

    #[test]
    fn f_i01_expressions() {
        // 현재 값 10 기준
        assert_eq!(ok("v*2", 10.0), 20.0); // 10 × 2
        assert_eq!(ok("value+10", 10.0), 20.0); // 10 + 10
        assert_eq!(ok("VALUE / 4", 10.0), 2.5);
        assert_eq!(ok("+=5", 10.0), 15.0); // 10 + 5
        assert_eq!(ok("-=5", 10.0), 5.0); // 10 − 5
        assert_eq!(ok("*=2", 10.0), 20.0); // 10 × 2
        assert_eq!(ok("/=4", 10.0), 2.5); // 10 ÷ 4
        assert_eq!(ok(" *= v - 8 ", 10.0), 20.0); // 10 × (10 − 8)
        assert_eq!(ok("(1+2)*3", 0.0), 9.0); // 3 × 3
        assert_eq!(ok("-v", 10.0), -10.0);
        assert_eq!(ok("1+2*3", 0.0), 7.0); // 곱셈 먼저
        assert_eq!(ok("2 - -3", 0.0), 5.0);
        assert_eq!(ok("7 % 3", 0.0), 1.0);
        assert_eq!(ok("1.5e2", 0.0), 150.0);
        assert_eq!(ok(".5", 0.0), 0.5);
        assert_eq!(ok("42", 0.0), 42.0);
        for bad in ["", "   ", "1+", "(1+2", "2x", "abc", "1..2", "*", "1 2", "+=", "v v"] {
            assert!(matches!(eval(bad, 1.0), Err(CoreError::InvalidArgs(_))), "{bad:?}는 오류여야 함");
        }
        for zero in ["1/0", "/=0", "5 % 0", "v/(v-v)"] {
            assert!(matches!(eval(zero, 3.0), Err(CoreError::InvalidArgs(_))), "{zero:?}는 0으로 나누기 오류여야 함");
        }
    }
}
