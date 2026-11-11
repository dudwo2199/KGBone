//! JSON 텍스트 → JValue. 숫자 글자와 키 순서를 그대로 보존한다.
use super::JValue;
use crate::{CoreError, Result};

/// 텍스트를 읽는다. 앞의 BOM은 무시한다.
pub fn parse(text: &str) -> Result<JValue> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut p = Parser { s: text.as_bytes(), i: 0, text };
    p.ws();
    let v = p.value(0)?;
    p.ws();
    if p.i != p.s.len() {
        return Err(p.err("값 뒤에 남은 글자가 있습니다"));
    }
    Ok(v)
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
    text: &'a str,
}

impl Parser<'_> {
    fn err(&self, m: &str) -> CoreError {
        let line = self.text[..self.i.min(self.text.len())].matches('\n').count() + 1;
        CoreError::Parse(format!("JSON {line}번째 줄: {m}"))
    }
    fn ws(&mut self) {
        while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }
    fn expect(&mut self, c: u8) -> Result<()> {
        if self.peek() == Some(c) {
            self.i += 1;
            Ok(())
        } else {
            Err(self.err(&format!("'{}'가 필요합니다", c as char)))
        }
    }
    fn value(&mut self, depth: usize) -> Result<JValue> {
        if depth > 256 {
            return Err(self.err("너무 깊게 중첩되었습니다"));
        }
        match self.peek() {
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => Ok(JValue::Str(self.string()?)),
            Some(b't') => self.lit("true", JValue::Bool(true)),
            Some(b'f') => self.lit("false", JValue::Bool(false)),
            Some(b'n') => self.lit("null", JValue::Null),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.number(),
            _ => Err(self.err("값이 필요합니다")),
        }
    }
    fn lit(&mut self, word: &str, v: JValue) -> Result<JValue> {
        if self.s[self.i..].starts_with(word.as_bytes()) {
            self.i += word.len();
            Ok(v)
        } else {
            Err(self.err("알 수 없는 글자"))
        }
    }
    fn number(&mut self) -> Result<JValue> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        let digits = |p: &mut Self| {
            let st = p.i;
            while p.peek().is_some_and(|c| c.is_ascii_digit()) {
                p.i += 1;
            }
            p.i > st
        };
        if !digits(self) {
            return Err(self.err("숫자 형식이 잘못되었습니다"));
        }
        if self.peek() == Some(b'.') {
            self.i += 1;
            if !digits(self) {
                return Err(self.err("소수점 뒤 숫자가 필요합니다"));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.i += 1;
            }
            if !digits(self) {
                return Err(self.err("지수 숫자가 필요합니다"));
            }
        }
        Ok(JValue::Num(self.text[start..self.i].to_string()))
    }
    fn hex4(&mut self) -> Result<u32> {
        let h = self.text.get(self.i..self.i + 4).ok_or_else(|| self.err("\\u 뒤 16진 4자리가 필요합니다"))?;
        let v = u32::from_str_radix(h, 16).map_err(|_| self.err("\\u 형식이 잘못되었습니다"))?;
        self.i += 4;
        Ok(v)
    }
    fn string(&mut self) -> Result<String> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let start = self.i;
            while self.i < self.s.len() && self.s[self.i] != b'"' && self.s[self.i] != b'\\' {
                if self.s[self.i] < 0x20 {
                    return Err(self.err("문자열 안에 제어 문자가 있습니다"));
                }
                self.i += 1;
            }
            out.push_str(&self.text[start..self.i]);
            match self.peek() {
                Some(b'"') => {
                    self.i += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.i += 1;
                    let c = self.peek().ok_or_else(|| self.err("문자열이 끝나지 않았습니다"))?;
                    self.i += 1;
                    match c {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let mut cp = self.hex4()?;
                            if (0xD800..0xDC00).contains(&cp) && self.s[self.i..].starts_with(b"\\u") {
                                self.i += 2;
                                let lo = self.hex4()?;
                                cp = 0x10000 + ((cp - 0xD800) << 10) + (lo.wrapping_sub(0xDC00) & 0x3FF);
                            }
                            out.push(char::from_u32(cp).unwrap_or('\u{fffd}'));
                        }
                        _ => return Err(self.err("모르는 이스케이프")),
                    }
                }
                _ => return Err(self.err("문자열이 끝나지 않았습니다")),
            }
        }
    }
    fn array(&mut self, depth: usize) -> Result<JValue> {
        self.expect(b'[')?;
        let mut a = vec![];
        self.ws();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Ok(JValue::Arr(a));
        }
        loop {
            self.ws();
            a.push(self.value(depth + 1)?);
            self.ws();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    return Ok(JValue::Arr(a));
                }
                _ => return Err(self.err("',' 또는 ']'가 필요합니다")),
            }
        }
    }
    fn object(&mut self, depth: usize) -> Result<JValue> {
        self.expect(b'{')?;
        let mut m: Vec<(String, JValue)> = vec![];
        self.ws();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Ok(JValue::Obj(m));
        }
        loop {
            self.ws();
            let k = self.string()?;
            self.ws();
            self.expect(b':')?;
            self.ws();
            let v = self.value(depth + 1)?;
            m.push((k, v));
            self.ws();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    return Ok(JValue::Obj(m));
                }
                _ => return Err(self.err("',' 또는 '}'가 필요합니다")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_number_text_and_order() {
        let v = parse("{\"b\": 1.50, \"a\": [1e2, -0.0, \"한글\\n\"], \"c\": null}").unwrap();
        let JValue::Obj(m) = &v else { panic!() };
        assert_eq!(m[0].0, "b");
        assert_eq!(m[0].1, JValue::Num("1.50".into()));
        assert_eq!(v.get("a").unwrap().as_array().unwrap()[0], JValue::Num("1e2".into()));
        assert_eq!(v.get("a").unwrap().as_array().unwrap()[2], JValue::Str("한글\n".into()));
    }

    #[test]
    fn rejects_bad_json() {
        assert!(parse("{\"a\": }").is_err());
        assert!(parse("[1, 2").is_err());
        assert!(parse("1 2").is_err());
    }

    #[test]
    fn unicode_escape() {
        assert_eq!(parse("\"\\uD55C\\ud83d\\ude00\"").unwrap(), JValue::Str("한😀".into()));
    }
}
