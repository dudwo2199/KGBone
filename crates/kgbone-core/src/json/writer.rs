//! JValue → 텍스트.
use super::JValue;

/// 출력 모양.
#[derive(Clone, Debug, PartialEq)]
pub struct WriteStyle {
    /// 들여쓰기 한 단계 글자. None이면 한 줄(압축) 출력.
    pub indent: Option<String>,
    /// 원소가 모두 원시값인 배열을 한 줄로.
    pub inline_primitive_arrays: bool,
    /// 끝에 줄바꿈.
    pub trailing_newline: bool,
}

impl Default for WriteStyle {
    /// .kgbone 표준 모양: 탭 들여쓰기, 원시값 배열 한 줄, 끝 줄바꿈.
    fn default() -> Self {
        WriteStyle { indent: Some("\t".into()), inline_primitive_arrays: true, trailing_newline: true }
    }
}

impl WriteStyle {
    pub fn compact() -> Self {
        WriteStyle { indent: None, inline_primitive_arrays: true, trailing_newline: false }
    }
}

/// 기존 텍스트의 들여쓰기 모양을 알아낸다(탭/공백 n개/압축). 원시값 배열을 여러 줄로 쓴 파일이면 그대로 따른다.
pub fn detect_style(text: &str) -> WriteStyle {
    let mut style = WriteStyle { trailing_newline: text.ends_with('\n'), ..WriteStyle::default() };
    let mut lines = text.lines().skip(1);
    let indent = lines.find(|l| !l.trim().is_empty()).map(|l| l[..l.len() - l.trim_start().len()].to_string());
    match indent {
        None => style.indent = None,
        Some(i) if i.is_empty() => style.indent = None,
        Some(i) => style.indent = Some(i),
    }
    // 숫자 하나만 있는 줄이 많으면 원시값 배열을 여러 줄로 쓴 파일이다.
    let lone_numbers = text
        .lines()
        .filter(|l| {
            let t = l.trim().trim_end_matches(',');
            !t.is_empty() && t.parse::<f64>().is_ok()
        })
        .count();
    if lone_numbers > 3 {
        style.inline_primitive_arrays = false;
    }
    style
}

pub fn write(v: &JValue, style: &WriteStyle) -> String {
    let mut out = String::new();
    write_value(&mut out, v, style, 0);
    if style.trailing_newline {
        out.push('\n');
    }
    out
}

fn is_primitive(v: &JValue) -> bool {
    !matches!(v, JValue::Arr(_) | JValue::Obj(_))
}

fn newline(out: &mut String, style: &WriteStyle, depth: usize) {
    if let Some(ind) = &style.indent {
        out.push('\n');
        for _ in 0..depth {
            out.push_str(ind);
        }
    }
}

fn write_value(out: &mut String, v: &JValue, style: &WriteStyle, depth: usize) {
    match v {
        JValue::Null => out.push_str("null"),
        JValue::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        JValue::Num(n) => out.push_str(n),
        JValue::Str(s) => write_string(out, s),
        JValue::Arr(a) => {
            if a.is_empty() {
                out.push_str("[]");
                return;
            }
            let inline = style.indent.is_none() || (style.inline_primitive_arrays && a.iter().all(is_primitive));
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                    if inline && style.indent.is_some() {
                        out.push(' ');
                    }
                }
                if !inline {
                    newline(out, style, depth + 1);
                }
                write_value(out, x, style, depth + 1);
            }
            if !inline {
                newline(out, style, depth);
            }
            out.push(']');
        }
        JValue::Obj(m) => {
            if m.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push('{');
            for (i, (k, x)) in m.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                newline(out, style, depth + 1);
                write_string(out, k);
                out.push(':');
                if style.indent.is_some() {
                    out.push(' ');
                }
                write_value(out, x, style, depth + 1);
            }
            newline(out, style, depth);
            out.push('}');
        }
    }
}

fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::parse;

    #[test]
    fn canonical_roundtrip_is_identical() {
        let text = "{\n\t\"a\": [1, 2.50, \"x\"],\n\t\"b\": {\n\t\t\"c\": []\n\t},\n\t\"d\": [\n\t\t{\n\t\t\t\"e\": true\n\t\t}\n\t]\n}\n";
        let v = parse(text).unwrap();
        assert_eq!(write(&v, &detect_style(text)), text);
    }

    #[test]
    fn compact() {
        let v = parse("{ \"a\" : [ 1 , { \"b\" : \"한\\\"\" } ] }").unwrap();
        assert_eq!(v.to_compact(), "{\"a\":[1,{\"b\":\"한\\\"\"}]}");
    }
}
