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

        // ---------- M04 곡선 ----------
        static Key K(float t, float[] v, CurveType c = CurveType.Linear, float[] bez = null) =>
            new Key { Time = t, V = v, Curve = c, Bezier = bez };

        [Test]
        public void m04_curve_linear_stepped_bezier()
        {
            var lin = new[] { K(0, new[] { 0f }), K(1, new[] { 10f }) };
            Assert.AreEqual(2.5f, Curve.ValueAt(lin, 0, 0.25f), 1e-5f);
            Assert.AreEqual(0f, Curve.ValueAt(lin, 0, -1f));
            Assert.AreEqual(10f, Curve.ValueAt(lin, 0, 2f));
            var step = new[] { K(0, new[] { 0f }, CurveType.Stepped), K(1, new[] { 10f }) };
            Assert.AreEqual(0f, Curve.ValueAt(step, 0, 0.99f));
            Assert.AreEqual(10f, Curve.ValueAt(step, 0, 1f));
            // 대칭 베지어: 가운데 시점에서 가운데 값
            var bez = new[] { K(0, new[] { 0f, 0f }, CurveType.Bezier, new[] { 0.3f, 0f, 0.7f, 20f }), K(1, new[] { 20f, 4f }) };
            Assert.AreEqual(10f, Curve.ValueAt(bez, 0, 0.5f), 1e-3f);
            // 채널 1에는 곡선이 없으므로 선형
            Assert.AreEqual(2f, Curve.ValueAt(bez, 1, 0.5f), 1e-5f);
            // 비대칭: 처음이 느림(ease-in) → 0.25에서 선형 값(5)보다 작다
            var easeIn = new[] { K(0, new[] { 0f }, CurveType.Bezier, new[] { 0.5f, 0f, 1f, 20f }), K(1, new[] { 20f }) };
            Assert.Less(Curve.ValueAt(easeIn, 0, 0.25f), 5f);
            Assert.AreEqual(0f, Curve.ValueAt(new Key[0], 0, 1f));
        }

        [Test]
        public void m04_ratio_uses_first_channel()
        {
            var keys = new[] { K(0, new float[0], CurveType.Bezier, new[] { 0.25f, 0f, 0.75f, 1f }), K(1, new float[0]) };
            Assert.IsTrue(Curve.RatioAt(keys, 0.5f, out int i, out float r));
            Assert.AreEqual(0, i);
            Assert.AreEqual(0.5f, r, 1e-3f);
            Assert.IsTrue(Curve.RatioAt(keys, 2f, out i, out r));
            Assert.AreEqual(1, i);
            Assert.AreEqual(0f, r);
            Assert.IsFalse(Curve.RatioAt(new Key[0], 0, out i, out r));
        }

        [Test]
        public void m04_step_index_before_first_is_zero()
        {
            var keys = new[] { K(1, new float[0]), K(2, new float[0]) };
            Assert.AreEqual(0, Curve.StepIndex(keys, 0f));
            Assert.AreEqual(0, Curve.StepIndex(keys, 1.5f));
            Assert.AreEqual(1, Curve.StepIndex(keys, 5f));
            Assert.AreEqual(-1, Curve.StepIndex(new Key[0], 0f));
        }

        // ---------- M03 본 월드 계산(상속 5종) ----------
        static WorldBone Child(WorldBone parent, Inherit inherit) =>
            BoneTransform.WorldOf(new LocalTransform { X = 5, ScaleX = 1, ScaleY = 1 }, inherit, parent);

        [Test]
        public void m03_inherit_modes_hand_computed()
        {
            // 부모: 회전 90, scaleX 2, 위치 (10, 20) → 행렬 [a=0, b=−1, c=2, d=0]
            var p = BoneTransform.WorldOf(new LocalTransform { X = 10, Y = 20, Rotation = 90, ScaleX = 2, ScaleY = 1 }, Inherit.Normal, null);
            Assert.AreEqual(0f, p.A, 1e-5f); Assert.AreEqual(-1f, p.B, 1e-5f); Assert.AreEqual(2f, p.C, 1e-5f); Assert.AreEqual(0f, p.D, 1e-5f);

            var n = Child(p, Inherit.Normal);
            Assert.AreEqual(10f, n.X, 1e-4f); Assert.AreEqual(30f, n.Y, 1e-4f);
            Assert.AreEqual(0f, n.A, 1e-5f); Assert.AreEqual(2f, n.C, 1e-5f);

            var t = Child(p, Inherit.OnlyTranslation);
            Assert.AreEqual(10f, t.X, 1e-4f); Assert.AreEqual(30f, t.Y, 1e-4f);
            Assert.AreEqual(1f, t.A, 1e-5f); Assert.AreEqual(0f, t.C, 1e-5f); Assert.AreEqual(1f, t.D, 1e-5f);

            var ns = Child(p, Inherit.NoScale); // 회전만: [0, −1; 1, 0]
            Assert.AreEqual(0f, ns.A, 1e-5f); Assert.AreEqual(1f, ns.C, 1e-5f); Assert.AreEqual(-1f, ns.B, 1e-5f);

            var nr = Child(p, Inherit.NoRotationOrReflection); // R(−90)·P = [2, 0; 0, 1]
            Assert.AreEqual(2f, nr.A, 1e-5f); Assert.AreEqual(0f, nr.C, 1e-5f); Assert.AreEqual(1f, nr.D, 1e-5f);
        }

        [Test]
        public void m03_reflected_parent_no_scale_keeps_reflection_only_when_asked()
        {
            // 부모 scaleX −1(반사): noScale은 반사를 물려받고(d = 1), noScaleOrReflection은 회전만(d = −1)
            var p = BoneTransform.WorldOf(new LocalTransform { ScaleX = -1, ScaleY = 1 }, Inherit.Normal, null);
            Assert.AreEqual(1f, Child(p, Inherit.NoScale).D, 1e-5f);
            Assert.AreEqual(-1f, Child(p, Inherit.NoScaleOrReflection).D, 1e-5f);
        }

        // ---------- M08 스킨 전용 본 ----------
        [Test]
        public void m08_skin_required_bone_inactive_without_skin()
        {
            var m = KGBoneLoader.Load(@"{""skeleton"":{
                ""bones"":[{""name"":""root""},{""name"":""s"",""parent"":""root"",""skinRequired"":true},{""name"":""c"",""parent"":""s""}],
                ""skins"":[{""name"":""default""},{""name"":""빨강"",""bones"":[""s""]}]}}");
            var active = new bool[3];
            ActiveBones.Compute(m, null, active);
            CollectionAssert.AreEqual(new[] { true, false, false }, active);
            ActiveBones.Compute(m, "빨강", active);
            CollectionAssert.AreEqual(new[] { true, true, true }, active);
        }

        // ---------- M05·M06 키 적용 ----------
        static SkeletonModel Model(string skeletonJson) => KGBoneLoader.Load("{\"skeleton\":" + skeletonJson + "}");

        [Test]
        public void m05_bone_keys_relative_to_setup()
        {
            var m = Model(@"{""bones"":[{""name"":""root"",""rotation"":30,""scaleX"":2,""x"":1}],
                ""animations"":[{""name"":""a"",""bones"":{""root"":{
                    ""rotate"":[{""time"":0,""v"":[10]}],""scale"":[{""time"":0,""v"":[1.5,1]}],""translatex"":[{""time"":0,""v"":[4]}]}}}]}");
            var pose = new PoseState(m);
            AnimationApplier.Apply(m, m.FindAnimation("a"), 0, null, pose);
            Assert.AreEqual(40f, pose.Locals[0].Rotation, 1e-5f);
            Assert.AreEqual(3f, pose.Locals[0].ScaleX, 1e-5f);
            Assert.AreEqual(5f, pose.Locals[0].X, 1e-5f);
        }

        [Test]
        public void m06_draw_order_key_moves_slot()
        {
            var m = Model(@"{""bones"":[{""name"":""root""}],
                ""slots"":[{""name"":""a"",""bone"":""root""},{""name"":""b"",""bone"":""root""},{""name"":""c"",""bone"":""root""}],
                ""animations"":[{""name"":""d"",""drawOrder"":[{""time"":0.5,""offsets"":[{""slot"":""a"",""offset"":2}]}]}]}");
            var pose = new PoseState(m);
            AnimationApplier.Apply(m, m.FindAnimation("d"), 1, null, pose);
            CollectionAssert.AreEqual(new[] { 1, 2, 0 }, pose.DrawOrder);
        }

        [Test]
        public void m06_attachment_key_without_s_clears()
        {
            var m = Model(@"{""bones"":[{""name"":""root""}],""slots"":[{""name"":""s"",""bone"":""root"",""attachment"":""x""}],
                ""animations"":[{""name"":""a"",""slots"":{""s"":{""attachment"":[{""time"":0,""s"":""y""},{""time"":1}]}}}]}");
            var pose = new PoseState(m);
            AnimationApplier.Apply(m, m.FindAnimation("a"), 0.5f, null, pose);
            Assert.AreEqual("y", pose.SlotAttachments[0]);
            pose.ResetToSetup(m);
            AnimationApplier.Apply(m, m.FindAnimation("a"), 1.5f, null, pose);
            Assert.IsNull(pose.SlotAttachments[0]);
        }

        [Test]
        public void m06_rgb2_on_slot_without_dark_starts_from_black()
        {
            var m = Model(@"{""bones"":[{""name"":""root""}],""slots"":[{""name"":""s"",""bone"":""root""}],
                ""animations"":[{""name"":""a"",""slots"":{""s"":{""rgb2"":[{""time"":0,""v"":[1,1,1,0,0,0]},{""time"":1,""v"":[0.5,0.5,1,0.2,0.2,0.4]}]}}}]}");
            var pose = new PoseState(m);
            AnimationApplier.Apply(m, m.FindAnimation("a"), 0.5f, null, pose);
            Assert.IsTrue(pose.SlotDarks[0].HasValue);
            Assert.AreEqual(0.2f, pose.SlotDarks[0].Value.B, 1e-5f);
            Assert.AreEqual(0.75f, pose.SlotColors[0].R, 1e-5f);
        }

        // ---------- M09 이벤트·반복 시점 ----------
        static SkeletonModel EventModel(string keys, float extraKeyTime) => Model(
            @"{""bones"":[{""name"":""root""}],""events"":[{""name"":""step"",""int"":7}],""animations"":[{""name"":""w"",
                ""bones"":{""root"":{""rotate"":[{""time"":" + extraKeyTime.ToString(System.Globalization.CultureInfo.InvariantCulture) + @",""v"":[0]}]}},
                ""events"":[" + keys + "]}]}");

        static float[] Fire(SkeletonModel m, float last, float time, bool looping)
        {
            var list = new System.Collections.Generic.List<FiredEvent>();
            EventFinder.Fired(m, m.FindAnimation("w"), last, time, looping, list);
            var r = new float[list.Count];
            for (int i = 0; i < r.Length; i++) r[i] = list[i].Time;
            return r;
        }

        [Test]
        public void m09_events_half_open_and_looping()
        {
            var m = EventModel(@"{""time"":0,""name"":""step"",""float"":0},{""time"":0.5,""name"":""step"",""float"":0.5},{""time"":1,""name"":""step"",""float"":1}", 0);
            CollectionAssert.AreEqual(new[] { 0f }, Fire(m, 0, 0.5f, false));
            CollectionAssert.AreEqual(new[] { 0.5f }, Fire(m, 0.5f, 1, false));
            Assert.AreEqual(0, Fire(m, 0.2f, 0.4f, false).Length);
            Assert.AreEqual(2, Fire(m, 0.9f, 1.2f, true).Length);
            CollectionAssert.AreEqual(new[] { 1f, 0f }, Fire(m, 0.9f, 0.1f, true));
            var list = new System.Collections.Generic.List<FiredEvent>();
            EventFinder.Fired(m, m.FindAnimation("w"), 0, 0.1f, false, list);
            Assert.AreEqual(7, list[0].Int); // 키에 없는 값은 정의 기본값
            Assert.AreEqual(1f, list[0].Volume);
        }

        [Test]
        public void m09_large_dt_crossing_two_loops_fires_each_lap()
        {
            // 길이 1(본 키가 1초), 이벤트 0·0.5. 실제 시간 [0.25, 2.75) → 실제 0.5, 1.0, 1.5, 2.0, 2.5에 5번 난다.
            // 이벤트 시간은 핵심처럼 키에 적힌 시간(애니메이션 안의 시간)이다: 0.5, 0, 0.5, 0, 0.5
            var m = EventModel(@"{""time"":0,""name"":""step""},{""time"":0.5,""name"":""step""}", 1);
            CollectionAssert.AreEqual(new[] { 0.5f, 0f, 0.5f, 0f, 0.5f }, Fire(m, 0.25f, 2.75f, true));
        }

        [Test]
        public void m09_zero_duration_loop_is_time_zero()
        {
            Assert.AreEqual(0f, AnimationApplier.AnimationTime(0, 3.7f, true));
            Assert.AreEqual(0.5f, AnimationApplier.AnimationTime(1, 2.5f, true), 1e-6f);
            Assert.AreEqual(0.75f, AnimationApplier.AnimationTime(1, -0.25f, true), 1e-6f);
            Assert.AreEqual(2.5f, AnimationApplier.AnimationTime(1, 2.5f, false));
            var m = EventModel(@"{""time"":0,""name"":""step""}", 0);
            CollectionAssert.AreEqual(new[] { 0f }, Fire(m, 0, 3.7f, true));
        }

        // ---------- M07 그리기 목록(손 계산) ----------
        [Test]
        public void m07_region_corners_hand_computed()
        {
            // 핵심 pose/draw.rs의 f_d01_region_corners_world와 같은 값.
            // region: 본 기준 (5, 0), scaleX 2, 폭 10 높이 4 → 로컬 모서리 (−5,−2) (15,−2) (15,2) (−5,2)
            // 본(회전 90°, (10, 20)): (x, y) → (10 − y, 20 + x) → (12,15) (12,35) (8,35) (8,15)
            var m = Model(@"{""bones"":[{""name"":""root""},{""name"":""b"",""parent"":""root"",""x"":10,""y"":20,""rotation"":90}],
                ""slots"":[{""name"":""s"",""bone"":""b"",""attachment"":""img""}],
                ""skins"":[{""name"":""default"",""attachments"":{""s"":{""img"":{""x"":5,""scaleX"":2,""width"":10,""height"":4}}}}]}");
            var r = new SkeletonPoser(m).Compute(new PoseRequest());
            Assert.AreEqual(1, r.Draw.Count);
            var d = r.Draw[0];
            var want = new[] { 12f, 15f, 12f, 35f, 8f, 35f, 8f, 15f };
            for (int i = 0; i < want.Length; i++) Assert.AreEqual(want[i], d.Positions[i], 1e-3f, "positions[" + i + "]");
            CollectionAssert.AreEqual(new[] { 0f, 1f, 1f, 1f, 1f, 0f, 0f, 0f }, d.Uvs);
            CollectionAssert.AreEqual(new[] { 0, 1, 2, 2, 3, 0 }, d.Indices);
            Assert.AreEqual("img", d.Image);
        }
    }
}
