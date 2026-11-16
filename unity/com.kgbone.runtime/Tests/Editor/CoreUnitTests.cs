// 유니티 런타임 계산부 손 계산 테스트. dotnet test와 유니티 EditMode 양쪽에서 돈다(순수 C#, NUnit).
using KGBone.Core;
using NUnit.Framework;

namespace KGBone.Tests
{
    public class CoreUnitTests
    {
        // ---------- M02 JSON ----------
        [Test]
        public void m02_json_parses_nested_ordered_members()
        {
            var v = JsonValue.Parse("{\"b\":1,\"a\":[1.5,-2e1,\"한글\u0041\"],\"c\":{\"t\":true,\"n\":null}}");
            Assert.AreEqual(JsonKind.Object, v.Kind);
            Assert.AreEqual("b", v.Members[0].Key);
            Assert.AreEqual("a", v.Members[1].Key);
            Assert.AreEqual("c", v.Members[2].Key);
            Assert.AreEqual(-20f, v.Get("a").Items[1].AsFloat());
            Assert.AreEqual(1.5f, v.Get("a").Items[0].AsFloat());
            Assert.AreEqual("한글A", v.Get("a").Items[2].AsString());
            Assert.AreEqual(true, v.Get("c").Get("t").AsBool());
            Assert.AreEqual(JsonKind.Null, v.Get("c").Get("n").Kind);
            Assert.IsNull(v.Get("없음"));
            Assert.AreEqual(1, v.Get("b").AsInt());
        }

        [Test]
        public void m02_json_error_reports_position()
        {
            var e = Assert.Throws<KGBoneFormatException>(() => JsonValue.Parse("{\"a\":}"));
            StringAssert.Contains("1:6", e.Message);
        }

        [Test]
        public void m02_json_skips_bom_and_reads_float_array()
        {
            var v = JsonValue.Parse("\uFEFF{\"v\":[1, 2.5, -3]}");
            CollectionAssert.AreEqual(new[] { 1f, 2.5f, -3f }, v.Get("v").AsFloatArray());
        }
    }
}
