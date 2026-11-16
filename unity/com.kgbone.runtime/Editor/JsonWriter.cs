// JsonValue → 한 줄 JSON 텍스트. 가져오기 도구가 편집 상태(editor)를 뺀 JSON을 에셋에 담을 때 쓴다.
using System.Globalization;
using System.Text;
using KGBone.Core;

namespace KGBone.Editor
{
    static class JsonWriter
    {
        /// <summary>최상위 객체에서 skip 키를 뺀 JSON.</summary>
        public static string WithoutTopLevel(JsonValue root, string skip)
        {
            var sb = new StringBuilder();
            sb.Append('{');
            bool first = true;
            foreach (var m in root.Members)
            {
                if (m.Key == skip) continue;
                if (!first) sb.Append(',');
                first = false;
                WriteString(sb, m.Key);
                sb.Append(':');
                Write(sb, m.Value);
            }
            sb.Append('}');
            return sb.ToString();
        }

        static void Write(StringBuilder sb, JsonValue v)
        {
            switch (v.Kind)
            {
                case JsonKind.Null: sb.Append("null"); break;
                case JsonKind.Bool: sb.Append(v.AsBool() ? "true" : "false"); break;
                case JsonKind.Number: sb.Append(v.AsDouble().ToString("R", CultureInfo.InvariantCulture)); break;
                case JsonKind.String: WriteString(sb, v.AsString()); break;
                case JsonKind.Array:
                    sb.Append('[');
                    for (int i = 0; i < v.Items.Count; i++)
                    {
                        if (i > 0) sb.Append(',');
                        Write(sb, v.Items[i]);
                    }
                    sb.Append(']');
                    break;
                default:
                    sb.Append('{');
                    for (int i = 0; i < v.Members.Count; i++)
                    {
                        if (i > 0) sb.Append(',');
                        WriteString(sb, v.Members[i].Key);
                        sb.Append(':');
                        Write(sb, v.Members[i].Value);
                    }
                    sb.Append('}');
                    break;
            }
        }

        static void WriteString(StringBuilder sb, string s)
        {
            sb.Append('"');
            foreach (char c in s)
            {
                switch (c)
                {
                    case '"': sb.Append("\\\""); break;
                    case '\\': sb.Append("\\\\"); break;
                    case '\n': sb.Append("\\n"); break;
                    case '\r': sb.Append("\\r"); break;
                    case '\t': sb.Append("\\t"); break;
                    default:
                        if (c < 0x20) sb.Append("\\u").Append(((int)c).ToString("x4", CultureInfo.InvariantCulture));
                        else sb.Append(c);
                        break;
                }
            }
            sb.Append('"');
        }
    }
}
