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

        // ---------- M02 .kgbone 읽기 ----------
        static string ReadSample(string rel) =>
            System.IO.File.ReadAllText(System.IO.Path.Combine(GoldenFile.RepoRoot(), rel));

        [Test]
        public void m02_loader_reads_samples()
        {
            var m = KGBoneLoader.Load(ReadSample("samples/kgbone/full-4.3.kgbone"));
            Assert.AreEqual("root", m.Bones[0].Name);
            Assert.AreEqual(-1, m.Bones[0].Parent);
            Assert.AreEqual(4, m.ConstraintCount);
            var body = m.Bones[m.BoneIndex("몸통")];
            Assert.AreEqual(90f, body.Rotation);
            Assert.AreEqual(0, body.Parent);
            Assert.IsNotNull(m.FindAnimation("꼬리흔들기"));
            var slot = m.Slots[m.SlotIndex("몸")];
            Assert.AreEqual("body", slot.Attachment);
            var att = m.FindSkin("default").Get("몸", "body") as RegionAttachment;
            Assert.IsNotNull(att);
            Assert.AreEqual(48f, att.Width);
            Assert.AreEqual(-90f, att.Rotation);
            Assert.AreEqual("body", att.ImagePath("body"));
        }

        [Test]
        public void m02_loader_defaults()
        {
            var m = KGBoneLoader.Load("{\"formatVersion\":1,\"skeleton\":{\"bones\":[{\"name\":\"root\"}]}}");
            Assert.AreEqual(1f, m.Bones[0].ScaleX);
            Assert.AreEqual(Inherit.Normal, m.Bones[0].Inherit);
            Assert.AreEqual("./images", m.ImagesPath);
            Assert.AreEqual(30f, m.Fps);
            Assert.AreEqual(0, m.ConstraintCount);
        }

        [Test]
        public void m02_loader_keys_curves_and_duration()
        {
            var m = KGBoneLoader.Load(@"{""skeleton"":{
                ""bones"":[{""name"":""root""}],
                ""slots"":[{""name"":""s"",""bone"":""root"",""color"":""FF000080"",""dark"":""00FF00"",""blend"":""screen""}],
                ""skins"":[{""name"":""default"",""attachments"":{""s"":{
                    ""a"":{""width"":10},
                    ""m"":{""type"":""mesh"",""path"":""p/q"",""uvs"":[0,0,1,0,1,1],""triangles"":[0,1,2],""vertices"":[0,0,1,0,1,1]},
                    ""seq"":{""sequence"":{""count"":3,""start"":1,""digits"":2,""setup"":1}}}}}],
                ""animations"":[{""name"":""w"",
                    ""bones"":{""root"":{""rotate"":[{""time"":0,""v"":[1],""curve"":[0.3,0,0.7,20]},{""time"":0.5,""v"":[2],""curve"":""stepped""},{""time"":2}]}},
                    ""drawOrder"":[{""time"":3,""offsets"":[{""slot"":""s"",""offset"":0}]}],
                    ""events"":[{""time"":1,""name"":""e""}]}]}}");
            var s = m.Slots[0];
            Assert.AreEqual(BlendMode.Screen, s.Blend);
            Assert.AreEqual(0.5f, s.Color.A, 0.003f);
            Assert.IsTrue(s.Dark.HasValue);
            Assert.AreEqual(1f, s.Dark.Value.G);
            var skin = m.FindSkin("default");
            Assert.AreEqual(32f, ((RegionAttachment)skin.Get("s", "a")).Height);
            Assert.AreEqual("p/q", skin.Get("s", "m").ImagePath("m"));
            Assert.AreEqual("seq02", skin.Get("s", "seq").ImagePath("seq"));
            Assert.IsTrue(m.HasSequence);
            var a = m.FindAnimation("w");
            Assert.AreEqual(3f, a.Duration);
            var keys = a.Bones[0].Keys;
            Assert.AreEqual("rotate", a.Bones[0].Name);
            Assert.AreEqual(CurveType.Bezier, keys[0].Curve);
            CollectionAssert.AreEqual(new[] { 0.3f, 0f, 0.7f, 20f }, keys[0].Bezier);
            Assert.AreEqual(CurveType.Stepped, keys[1].Curve);
            Assert.AreEqual(CurveType.Linear, keys[2].Curve);
            Assert.AreEqual(0, keys[2].V.Length);
            Assert.AreEqual("e", a.Events[0].Name);
            Assert.IsFalse(a.Events[0].Int.HasValue);
        }
    }
}
