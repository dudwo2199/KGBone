// 유니티 런타임 2단계(제약) 손 계산 테스트. dotnet test와 유니티 EditMode 양쪽에서 돈다(순수 C#, NUnit).
using KGBone.Core;
using NUnit.Framework;

namespace KGBone.Tests
{
    public class ConstraintUnitTests
    {
        const string Bones = @"""bones"":[{""name"":""root""},{""name"":""a"",""parent"":""root"",""length"":50},{""name"":""t"",""parent"":""root"",""x"":80}]";

        static SkeletonModel Load(string skeletonBody, int formatVersion = 1) =>
            KGBoneLoader.Load("{\"formatVersion\":" + formatVersion + ",\"skeleton\":{" + Bones + "," + skeletonBody + "}}");

        // ---------- M13 불러오기 ----------
        [Test]
        public void m13_loads_ik_with_defaults()
        {
            var m = Load(@"""constraints"":[{""type"":""ik"",""name"":""k"",""bones"":[""a""],""target"":""t""}]");
            Assert.AreEqual(1, m.Constraints.Count);
            var k = (IkConstraintData)m.Constraints[0];
            Assert.AreEqual(ConstraintType.Ik, k.Type);
            Assert.AreEqual("k", k.Name);
            Assert.IsFalse(k.SkinRequired);
            CollectionAssert.AreEqual(new[] { 1 }, k.Bones);
            Assert.AreEqual(2, k.Target);
            Assert.AreEqual(1f, k.Mix);
            Assert.AreEqual(0f, k.Softness);
            Assert.IsTrue(k.BendPositive);
            Assert.IsFalse(k.Compress || k.Stretch || k.Uniform || k.Volume);
            Assert.AreEqual(0, m.ConstraintIndex("k"));
            Assert.AreEqual(-1, m.ConstraintIndex("없음"));
        }

        [Test]
        public void m13_loads_transform_default_mappings()
        {
            var m = Load(@"""constraints"":[{""type"":""transform"",""name"":""tr"",""bones"":[""a""],""source"":""t""}]");
            var c = (TransformConstraintData)m.Constraints[0];
            Assert.AreEqual(2, c.Source);
            Assert.AreEqual(6, c.Properties.Count);
            for (int i = 0; i < 6; i++)
            {
                var p = c.Properties[i];
                Assert.AreEqual((BoneProperty)i, p.From);
                Assert.AreEqual(0f, p.Offset);
                Assert.AreEqual(1, p.To.Count);
                Assert.AreEqual((BoneProperty)i, p.To[0].Property);
                Assert.AreEqual(0f, p.To[0].Offset);
                Assert.AreEqual(1f, p.To[0].Scale);
                Assert.AreEqual(0f, p.To[0].Max);
            }
            Assert.AreEqual(1f, c.MixRotate);
            Assert.AreEqual(1f, c.MixShearY);
            Assert.IsFalse(c.LocalSource || c.LocalTarget || c.Additive || c.Clamp);
        }

        [Test]
        public void m13_loads_transform_explicit_mapping()
        {
            var m = Load(@"""constraints"":[{""type"":""transform"",""name"":""tr"",""bones"":[""a""],""source"":""t"",
                ""properties"":[{""from"":""rotate"",""offset"":5,""to"":[{""property"":""x"",""offset"":1,""max"":9,""scale"":2}]}]}]");
            var c = (TransformConstraintData)m.Constraints[0];
            Assert.AreEqual(1, c.Properties.Count);
            Assert.AreEqual(BoneProperty.Rotate, c.Properties[0].From);
            Assert.AreEqual(5f, c.Properties[0].Offset);
            Assert.AreEqual(BoneProperty.X, c.Properties[0].To[0].Property);
            Assert.AreEqual(1f, c.Properties[0].To[0].Offset);
            Assert.AreEqual(9f, c.Properties[0].To[0].Max);
            Assert.AreEqual(2f, c.Properties[0].To[0].Scale);
        }

        [Test]
        public void m13_loads_physics_and_slider_fields()
        {
            var m = Load(@"""constraints"":[
                {""type"":""physics"",""name"":""p"",""bone"":""a"",""x"":0.5,""rotate"":1,""fps"":30,""limit"":100,""inertia"":0.4,""strength"":50,""damping"":2,""mass"":3,""wind"":4,""gravity"":5,""mix"":0.7,""volume"":true},
                {""type"":""physics"",""name"":""p0"",""bone"":""a""},
                {""type"":""slider"",""name"":""s"",""animation"":""walk"",""bone"":""t"",""property"":""rotate"",""propertyFrom"":10,""propertyTo"":20,""loop"":true,""additive"":true,""local"":true,""time"":0.3,""mix"":0.5},
                {""type"":""slider"",""name"":""s0"",""animation"":""walk""}]");
            var p = (PhysicsConstraintData)m.Constraints[0];
            Assert.AreEqual(1, p.Bone);
            Assert.AreEqual(0.5f, p.X);
            Assert.AreEqual(0f, p.Y);
            Assert.AreEqual(1f, p.Rotate);
            Assert.AreEqual(30f, p.Fps);
            Assert.AreEqual(100f, p.Limit);
            Assert.AreEqual(0.4f, p.Inertia);
            Assert.AreEqual(50f, p.Strength);
            Assert.AreEqual(2f, p.Damping);
            Assert.AreEqual(3f, p.Mass);
            Assert.AreEqual(4f, p.Wind);
            Assert.AreEqual(5f, p.Gravity);
            Assert.AreEqual(0.7f, p.Mix);
            Assert.IsTrue(p.Volume);
            var p0 = (PhysicsConstraintData)m.Constraints[1];
            Assert.AreEqual(5000f, p0.Limit);
            Assert.AreEqual(60f, p0.Fps);
            Assert.AreEqual(1f, p0.Inertia);
            Assert.AreEqual(100f, p0.Strength);
            Assert.AreEqual(1f, p0.Damping);
            Assert.AreEqual(1f, p0.Mass);
            Assert.AreEqual(1f, p0.Mix);
            Assert.AreEqual(0f, p0.ScaleX);
            Assert.AreEqual(0f, p0.ShearX);
            var s = (SliderConstraintData)m.Constraints[2];
            Assert.AreEqual("walk", s.Animation);
            Assert.AreEqual(2, s.Bone);
            Assert.AreEqual(BoneProperty.Rotate, s.Property);
            Assert.AreEqual(10f, s.PropertyFrom);
            Assert.AreEqual(20f, s.PropertyTo);
            Assert.IsTrue(s.Loop && s.Additive && s.Local);
            Assert.AreEqual(0.3f, s.Time);
            Assert.AreEqual(0.5f, s.Mix);
            var s0 = (SliderConstraintData)m.Constraints[3];
            Assert.AreEqual(-1, s0.Bone);
            Assert.AreEqual(BoneProperty.X, s0.Property);
            Assert.AreEqual(1f, s0.PropertyTo);
            Assert.AreEqual(1f, s0.Mix);
            Assert.IsFalse(s0.Loop || s0.Additive || s0.Local);
        }

        [Test]
        public void m13_legacy_arrays_merged_by_order()
        {
            var m = Load(@"""ik"":[{""name"":""k"",""order"":2,""bones"":[""a""],""target"":""t""}],
                ""transform"":[{""name"":""tr"",""order"":0,""bones"":[""a""],""source"":""t""}],
                ""physics"":[{""name"":""p"",""order"":1,""bone"":""a""}]", 0);
            Assert.AreEqual(3, m.Constraints.Count);
            Assert.AreEqual("tr", m.Constraints[0].Name);
            Assert.AreEqual("p", m.Constraints[1].Name);
            Assert.AreEqual("k", m.Constraints[2].Name);
            Assert.AreEqual(ConstraintType.Physics, m.Constraints[1].Type);
        }

        [Test]
        public void m13_legacy_same_order_keeps_kind_then_index()
        {
            // 핵심 migrate.rs: (order, 종류 순서 ik<transform<path<physics, 원래 순서)
            var m = Load(@"""physics"":[{""name"":""p"",""bone"":""a""}],
                ""ik"":[{""name"":""k2"",""bones"":[""a""],""target"":""t""},{""name"":""k1"",""bones"":[""a""],""target"":""t""}]", 0);
            Assert.AreEqual("k2", m.Constraints[0].Name);
            Assert.AreEqual("k1", m.Constraints[1].Name);
            Assert.AreEqual("p", m.Constraints[2].Name);
        }

        [Test]
        public void m13_constraint_timelines_loaded()
        {
            var m = Load(@"""constraints"":[{""type"":""physics"",""name"":""p"",""bone"":""a""},{""type"":""ik"",""name"":""k"",""bones"":[""a""],""target"":""t""}],
                ""animations"":[{""name"":""go"",""constraints"":{""k"":{""mix"":[{""time"":0,""v"":[1]},{""time"":2,""v"":[0]}]},""p"":{""reset"":[{""time"":0.5}]},""없음"":{""mix"":[{""time"":3,""v"":[0]}]}}}]");
            var a = m.FindAnimation("go");
            Assert.AreEqual(2, a.Constraints.Count);
            var k = a.Constraints.Find(x => x.Name == "mix");
            Assert.AreEqual(1, k.Constraint);
            Assert.AreEqual(2, k.Keys.Length);
            var r = a.Constraints.Find(x => x.Name == "reset");
            Assert.AreEqual(0, r.Constraint);
            Assert.AreEqual(0.5f, r.Keys[0].Time);
            Assert.AreEqual(3f, a.Duration);
        }

        [Test]
        public void m14_loads_path_attachment()
        {
            var m = Load(@"""slots"":[{""name"":""s"",""bone"":""root"",""attachment"":""p""}],
                ""skins"":[{""name"":""default"",""attachments"":{""s"":{""p"":{""type"":""path"",""closed"":true,""constantSpeed"":false,""vertexCount"":3,""vertices"":[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18],""lengths"":[10,20]},
                    ""q"":{""type"":""path""}}}}]");
            var p = (PathAttachmentData)m.FindSkin("default").Get("s", "p");
            Assert.IsTrue(p.Closed);
            Assert.IsFalse(p.ConstantSpeed);
            Assert.AreEqual(3, p.VertexCount);
            Assert.AreEqual(18, p.Vertices.Length);
            Assert.IsFalse(p.Weighted);
            CollectionAssert.AreEqual(new[] { 10f, 20f }, p.Lengths);
            Assert.IsNull(p.ImagePath("p"));
            var q = (PathAttachmentData)m.FindSkin("default").Get("s", "q");
            Assert.IsTrue(q.ConstantSpeed);
            Assert.AreEqual(0, q.VertexCount);
        }
    }
}
