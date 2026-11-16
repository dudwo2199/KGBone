// 작은 재귀 하강 JSON 파서. 외부 패키지에 기대지 않는다.
using System.Collections.Generic;
using System.Globalization;
using System.Text;

namespace KGBone.Core
{
    static class JsonParser
    {
        public static JsonValue Parse(string text)
        {
            if (text == null) throw new KGBoneFormatException("JSON이 비어 있습니다");
            var p = new Reader(text);
            if (p.Peek() == '﻿') p.pos++;
            p.SkipWs();
            var v = p.Value();
            p.SkipWs();
            if (!p.End) p.Fail("JSON 뒤에 남은 글자가 있습니다");
            return v;
        }

        sealed class Reader
        {
            readonly string s;
            public int pos;
            public Reader(string s) { this.s = s; }

            public bool End => pos >= s.Length;
            public char Peek() => pos < s.Length ? s[pos] : '\0';

            public void Fail(string what)
            {
                int line = 1, col = 1;
                for (int i = 0; i < pos && i < s.Length; i++)
                {
                    if (s[i] == '\n') { line++; col = 1; }
                    else col++;
                }
                throw new KGBoneFormatException($"JSON 형식 오류 {line}:{col}: {what}");
            }

            public void SkipWs()
            {
                while (pos < s.Length && (s[pos] == ' ' || s[pos] == '\t' || s[pos] == '\n' || s[pos] == '\r')) pos++;
            }

            void Expect(char c)
            {
                if (Peek() != c) Fail($"'{c}'가 와야 합니다");
                pos++;
            }

            public JsonValue Value()
            {
                switch (Peek())
                {
                    case '{': return Object();
                    case '[': return Array();
                    case '"': return new JsonValue(JsonKind.String, s: Str());
                    case 't': Word("true"); return new JsonValue(JsonKind.Bool, b: true);
                    case 'f': Word("false"); return new JsonValue(JsonKind.Bool, b: false);
                    case 'n': Word("null"); return new JsonValue(JsonKind.Null);
                    default:
                        char c = Peek();
                        if (c == '-' || (c >= '0' && c <= '9')) return Number();
                        Fail("값이 와야 합니다");
                        return null;
                }
            }

            void Word(string w)
            {
                if (string.CompareOrdinal(s, pos, w, 0, w.Length) != 0) Fail($"'{w}'가 와야 합니다");
                pos += w.Length;
            }

            JsonValue Object()
            {
                Expect('{');
                var members = new List<KeyValuePair<string, JsonValue>>();
                SkipWs();
                if (Peek() == '}') { pos++; return new JsonValue(JsonKind.Object, members: members); }
                while (true)
                {
                    SkipWs();
                    if (Peek() != '"') Fail("키(문자열)가 와야 합니다");
                    string key = Str();
                    SkipWs();
                    Expect(':');
                    SkipWs();
                    members.Add(new KeyValuePair<string, JsonValue>(key, Value()));
                    SkipWs();
                    if (Peek() == ',') { pos++; continue; }
                    Expect('}');
                    return new JsonValue(JsonKind.Object, members: members);
                }
            }

            JsonValue Array()
            {
                Expect('[');
                var items = new List<JsonValue>();
                SkipWs();
                if (Peek() == ']') { pos++; return new JsonValue(JsonKind.Array, items: items); }
                while (true)
                {
                    SkipWs();
                    items.Add(Value());
                    SkipWs();
                    if (Peek() == ',') { pos++; continue; }
                    Expect(']');
                    return new JsonValue(JsonKind.Array, items: items);
                }
            }

            string Str()
            {
                Expect('"');
                var sb = new StringBuilder();
                while (true)
                {
                    if (End) Fail("문자열이 끝나지 않았습니다");
                    char c = s[pos++];
                    if (c == '"') return sb.ToString();
                    if (c != '\\') { sb.Append(c); continue; }
                    if (End) Fail("문자열이 끝나지 않았습니다");
                    char e = s[pos++];
                    switch (e)
                    {
                        case '"': sb.Append('"'); break;
                        case '\\': sb.Append('\\'); break;
                        case '/': sb.Append('/'); break;
                        case 'b': sb.Append('\b'); break;
                        case 'f': sb.Append('\f'); break;
                        case 'n': sb.Append('\n'); break;
                        case 'r': sb.Append('\r'); break;
                        case 't': sb.Append('\t'); break;
                        case 'u':
                            int code = 0;
                            if (pos + 4 > s.Length || !int.TryParse(s.Substring(pos, 4), NumberStyles.HexNumber, CultureInfo.InvariantCulture, out code))
                                Fail("\\u 뒤에 16진 숫자 4개가 와야 합니다");
                            sb.Append((char)code);
                            pos += 4;
                            break;
                        default: pos--; Fail("알 수 없는 이스케이프"); break;
                    }
                }
            }

            JsonValue Number()
            {
                int start = pos;
                if (Peek() == '-') pos++;
                while (!End && ((s[pos] >= '0' && s[pos] <= '9') || s[pos] == '.' || s[pos] == 'e' || s[pos] == 'E' || s[pos] == '+' || s[pos] == '-')) pos++;
                if (!double.TryParse(s.Substring(start, pos - start), NumberStyles.Float, CultureInfo.InvariantCulture, out double n))
                {
                    pos = start;
                    Fail("숫자 형식이 잘못됐습니다");
                }
                return new JsonValue(JsonKind.Number, n: n);
            }
        }
    }
}
