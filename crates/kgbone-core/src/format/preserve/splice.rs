//! 원본 텍스트 위에 새 트리를 덧쓴다. 같은 값은 원래 글자 그대로, 바뀐 값만 새로 쓴다.
//!
//! - 객체: 키 목록이 같으면 값마다 제자리 교체. 원래 키 뒤에 키가 덧붙기만 했으면 원래 구분 모양으로 덧붙인다.
//! - 배열: 길이가 같으면 원소마다 제자리 교체. 뒤에 덧붙기만 했으면 덧붙인다.
//! - 그 밖(키 삭제·순서 변경·종류 변경)은 그 값만 원본 파일 모양([`detect_style`])으로 새로 쓴다.
use crate::json::{self, detect_style, JValue, WriteStyle};

/// `text`: 원본 텍스트, `new`: 새 트리. 원본을 읽을 수 없으면 원본 모양으로 전체를 새로 쓴다.
pub(crate) fn splice(text: &str, new: &JValue) -> String {
    let style = WriteStyle { trailing_newline: false, ..detect_style(text) };
    let mut sc = Scanner { s: text.as_bytes(), i: 0 };
    sc.ws();
    let Some(root) = sc.value() else {
        return json::write(new, &detect_style(text));
    };
    let mut out = String::with_capacity(text.len() + 64);
    out.push_str(&text[..root.start]);
    let ctx = Ctx { text, style: &style };
    ctx.emit(&root, new, &mut out);
    out.push_str(&text[root.end..]);
    out
}

/// 원본 텍스트 속 값 하나의 위치.
struct Node {
    start: usize,
    end: usize,
    kind: Kind,
}

enum Kind {
    Leaf,
    Arr(Vec<Node>),
    /// (키 시작 위치, 키 이름, 값)
    Obj(Vec<(usize, String, Node)>),
}

struct Ctx<'a> {
    text: &'a str,
    style: &'a WriteStyle,
}

impl Ctx<'_> {
    fn leaf(&self, n: &Node) -> Option<JValue> {
        json::parse(&self.text[n.start..n.end]).ok()
    }

    /// 원본 값과 새 값이 같은가.
    fn same(&self, n: &Node, v: &JValue) -> bool {
        match (&n.kind, v) {
            (Kind::Leaf, _) => self.leaf(n).as_ref() == Some(v),
            (Kind::Arr(a), JValue::Arr(b)) => a.len() == b.len() && a.iter().zip(b).all(|(x, y)| self.same(x, y)),
            (Kind::Obj(a), JValue::Obj(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|((_, k, x), (bk, y))| k == bk && self.same(x, y))
            }
            _ => false,
        }
    }

    /// `pos`가 있는 줄의 앞 공백.
    fn line_indent(&self, pos: usize) -> &str {
        let line_start = self.text[..pos].rfind('\n').map_or(0, |i| i + 1);
        let line = &self.text[line_start..];
        &line[..line.len() - line.trim_start_matches([' ', '\t']).len()]
    }

    /// 새 값을 원본 모양으로 쓴다(여러 줄이면 `base` 들여쓰기를 붙인다).
    fn fresh(&self, v: &JValue, base: &str, out: &mut String) {
        let s = json::write(v, self.style);
        out.push_str(&s.replace('\n', &format!("\n{base}")));
    }

    /// 두 원소 사이 구분(쉼표 뒤 공백). 원소가 하나뿐이면 여는 괄호 뒤 공백.
    fn separator(&self, open: usize, first_start: usize, between: Option<(usize, usize)>) -> String {
        match between {
            Some((e, s)) => self.text[e..s].replacen(',', "", 1),
            None => self.text[open + 1..first_start].to_string(),
        }
    }

    fn emit(&self, n: &Node, v: &JValue, out: &mut String) {
        if self.same(n, v) {
            out.push_str(&self.text[n.start..n.end]);
            return;
        }
        match (&n.kind, v) {
            (Kind::Obj(members), JValue::Obj(nm)) if !members.is_empty() && nm.len() >= members.len() => {
                if !members.iter().zip(nm).all(|((_, k, _), (nk, _))| k == nk) {
                    return self.fresh(v, self.line_indent(n.start), out);
                }
                let mut cur = n.start;
                for ((_, _, child), (_, nv)) in members.iter().zip(nm) {
                    out.push_str(&self.text[cur..child.start]);
                    self.emit(child, nv, out);
                    cur = child.end;
                }
                if nm.len() > members.len() {
                    let last = &members[members.len() - 1];
                    let sep = self.separator(n.start, members[0].0, members.get(1).map(|m| (members[0].2.end, m.0)));
                    let base = self.line_indent(last.0).to_string();
                    for (k, nv) in &nm[members.len()..] {
                        out.push(',');
                        out.push_str(&sep);
                        out.push_str(&JValue::Str(k.clone()).to_compact());
                        out.push(':');
                        if self.style.indent.is_some() {
                            out.push(' ');
                        }
                        self.fresh(nv, &base, out);
                    }
                }
                out.push_str(&self.text[cur..n.end]);
            }
            (Kind::Arr(elems), JValue::Arr(na)) if !elems.is_empty() && na.len() >= elems.len() => {
                let mut cur = n.start;
                for (child, nv) in elems.iter().zip(na) {
                    out.push_str(&self.text[cur..child.start]);
                    self.emit(child, nv, out);
                    cur = child.end;
                }
                if na.len() > elems.len() {
                    let sep = self.separator(n.start, elems[0].start, elems.get(1).map(|e| (elems[0].end, e.start)));
                    let base = self.line_indent(elems[elems.len() - 1].start).to_string();
                    for nv in &na[elems.len()..] {
                        out.push(',');
                        out.push_str(&sep);
                        self.fresh(nv, &base, out);
                    }
                }
                out.push_str(&self.text[cur..n.end]);
            }
            _ => self.fresh(v, self.line_indent(n.start), out),
        }
    }
}

/// 위치만 기록하는 간단한 JSON 훑기(원본은 이미 [`json::parse`]로 검증된 텍스트).
struct Scanner<'a> {
    s: &'a [u8],
    i: usize,
}

impl Scanner<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }
    fn string_end(&mut self) -> Option<()> {
        if self.peek()? != b'"' {
            return None;
        }
        self.i += 1;
        loop {
            match self.peek()? {
                b'"' => {
                    self.i += 1;
                    return Some(());
                }
                b'\\' => self.i += 2,
                _ => self.i += 1,
            }
        }
    }
    fn value(&mut self) -> Option<Node> {
        let start = self.i;
        let kind = match self.peek()? {
            b'{' => {
                self.i += 1;
                let mut m = vec![];
                self.ws();
                if self.peek()? == b'}' {
                    self.i += 1;
                } else {
                    loop {
                        self.ws();
                        let ks = self.i;
                        self.string_end()?;
                        let key = match json::parse(std::str::from_utf8(&self.s[ks..self.i]).ok()?).ok()? {
                            JValue::Str(k) => k,
                            _ => return None,
                        };
                        self.ws();
                        if self.peek()? != b':' {
                            return None;
                        }
                        self.i += 1;
                        self.ws();
                        let v = self.value()?;
                        m.push((ks, key, v));
                        self.ws();
                        match self.peek()? {
                            b',' => self.i += 1,
                            b'}' => {
                                self.i += 1;
                                break;
                            }
                            _ => return None,
                        }
                    }
                }
                Kind::Obj(m)
            }
            b'[' => {
                self.i += 1;
                let mut a = vec![];
                self.ws();
                if self.peek()? == b']' {
                    self.i += 1;
                } else {
                    loop {
                        self.ws();
                        a.push(self.value()?);
                        self.ws();
                        match self.peek()? {
                            b',' => self.i += 1,
                            b']' => {
                                self.i += 1;
                                break;
                            }
                            _ => return None,
                        }
                    }
                }
                Kind::Arr(a)
            }
            b'"' => {
                self.string_end()?;
                Kind::Leaf
            }
            _ => {
                while self.i < self.s.len() && !matches!(self.s[self.i], b',' | b']' | b'}' | b' ' | b'\t' | b'\n' | b'\r') {
                    self.i += 1;
                }
                if self.i == start {
                    return None;
                }
                Kind::Leaf
            }
        };
        Some(Node { start, end: self.i, kind })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::parse;

    #[test]
    fn unchanged_is_identical() {
        let t = "{\n  \"a\": [1, 2.50],\n  \"b\": { \"k\": true }\n}\n";
        assert_eq!(splice(t, &parse(t).unwrap()), t);
    }

    #[test]
    fn replaces_only_changed_value() {
        let t = "{\n\t\"a\": 1.50,\n\t\"b\": { \"k\": true },\n\t\"c\": \"x\"\n}\n";
        let mut v = parse(t).unwrap();
        v.set("c", JValue::str("y"));
        assert_eq!(splice(t, &v), t.replace("\"x\"", "\"y\""));
    }

    #[test]
    fn appends_keys_and_elements() {
        let t = "{\n\t\"a\": [\n\t\t1,\n\t\t2\n\t]\n}\n";
        let mut v = parse(t).unwrap();
        v.set("a", parse("[1, 2, 3]").unwrap());
        v.set("n", parse("{\"x\": 1}").unwrap());
        assert_eq!(splice(t, &v), "{\n\t\"a\": [\n\t\t1,\n\t\t2,\n\t\t3\n\t],\n\t\"n\": {\n\t\t\"x\": 1\n\t}\n}\n");
    }

    #[test]
    fn removed_key_rewrites_that_object_only() {
        let t = "{\n\t\"keep\": 1.0,\n\t\"o\": {\n\t\t\"a\": 1,\n\t\t\"b\": 2\n\t}\n}\n";
        let mut v = parse(t).unwrap();
        v.get_mut("o").unwrap().remove("b");
        assert_eq!(splice(t, &v), "{\n\t\"keep\": 1.0,\n\t\"o\": {\n\t\t\"a\": 1\n\t}\n}\n");
    }
}
