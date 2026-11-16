// 읽기 전용 JSON 값. 객체 멤버는 파일 순서를 지킨다(애니메이션 타임라인 적용 순서가 파일 순서이므로).
using System;
using System.Collections.Generic;

namespace KGBone.Core
{
    public enum JsonKind { Null, Bool, Number, String, Array, Object }

    public sealed class JsonValue
    {
        static readonly IReadOnlyList<JsonValue> NoItems = new JsonValue[0];
        static readonly IReadOnlyList<KeyValuePair<string, JsonValue>> NoMembers = new KeyValuePair<string, JsonValue>[0];

        public JsonKind Kind { get; }
        readonly bool boolValue;
        readonly double number;
        readonly string text;
        readonly List<JsonValue> items;
        readonly List<KeyValuePair<string, JsonValue>> members;
        Dictionary<string, JsonValue> index;

        internal JsonValue(JsonKind kind, bool b = false, double n = 0, string s = null,
            List<JsonValue> items = null, List<KeyValuePair<string, JsonValue>> members = null)
        {
            Kind = kind;
            boolValue = b;
            number = n;
            text = s;
            this.items = items;
            this.members = members;
        }

        /// <summary>JSON 텍스트를 읽는다. 잘못된 텍스트면 줄:열이 담긴 KGBoneFormatException.</summary>
        public static JsonValue Parse(string text) => JsonParser.Parse(text);

        public IReadOnlyList<JsonValue> Items => items ?? NoItems;
        public IReadOnlyList<KeyValuePair<string, JsonValue>> Members => members ?? NoMembers;

        /// <summary>객체의 키 값. 객체가 아니거나 키가 없으면 null.</summary>
        public JsonValue Get(string key)
        {
            if (members == null) return null;
            if (index == null)
            {
                index = new Dictionary<string, JsonValue>(members.Count, StringComparer.Ordinal);
                foreach (var m in members) index[m.Key] = m.Value; // 같은 키가 여러 번이면 마지막 값
            }
            return index.TryGetValue(key, out var v) ? v : null;
        }

        public float AsFloat(float fallback = 0) => Kind == JsonKind.Number ? (float)number : fallback;
        public double AsDouble(double fallback = 0) => Kind == JsonKind.Number ? number : fallback;
        public int AsInt(int fallback = 0) => Kind == JsonKind.Number ? (int)number : fallback;
        public bool AsBool(bool fallback = false) => Kind == JsonKind.Bool ? boolValue : fallback;
        public string AsString(string fallback = null) => Kind == JsonKind.String ? text : fallback;

        /// <summary>숫자 배열. 배열이 아니면 빈 배열, 숫자가 아닌 원소는 0.</summary>
        public float[] AsFloatArray()
        {
            if (items == null) return new float[0];
            var r = new float[items.Count];
            for (int i = 0; i < r.Length; i++) r[i] = items[i].AsFloat();
            return r;
        }
    }

    /// <summary>.kgbone 형식 오류(한국어 메시지).</summary>
    public sealed class KGBoneFormatException : Exception
    {
        public KGBoneFormatException(string message) : base(message) { }
    }
}
