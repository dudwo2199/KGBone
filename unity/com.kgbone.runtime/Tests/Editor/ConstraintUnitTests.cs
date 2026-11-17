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
        public void m13_empty_constraint_name_targets_all_physics()
        {
            var m = Load(@"""constraints"":[{""type"":""physics"",""name"":""p"",""bone"":""a""},{""type"":""ik"",""name"":""k"",""bones"":[""a""],""target"":""t""},{""type"":""physics"",""name"":""q"",""bone"":""a""}],
                ""animations"":[{""name"":""wind"",""constraints"":{"""":{""wind"":[{""time"":0,""v"":[3]}]}}}]");
            var pose = new PoseState(m);
            AnimationApplier.Apply(m, m.FindAnimation("wind"), 0, null, pose);
            Assert.AreEqual(3f, pose.ConstraintValues[0][4]);
            Assert.AreEqual(5, pose.ConstraintValues[1].Length);
            Assert.AreEqual(3f, pose.ConstraintValues[2][4]);
        }

        // ---------- M13 제약 값·키·섞기 ----------
        [Test]
        public void m13_constraint_setup_values()
        {
            var m = Load(@"""constraints"":[{""type"":""ik"",""name"":""k"",""bones"":[""a""],""target"":""t"",""mix"":0.5,""bendPositive"":false,""stretch"":true},
                {""type"":""slider"",""name"":""s"",""animation"":""x"",""time"":2,""mix"":0.25}]");
            var pose = new PoseState(m);
            CollectionAssert.AreEqual(new[] { 0.5f, 0f, -1f, 0f, 1f }, pose.ConstraintValues[0]);
            CollectionAssert.AreEqual(new[] { 2f, 0.25f }, pose.ConstraintValues[1]);
        }

        [Test]
        public void m13_constraint_key_sets_ik_mix()
        {
            var m = Load(@"""constraints"":[{""type"":""ik"",""name"":""k"",""bones"":[""a""],""target"":""t""}],
                ""animations"":[{""name"":""go"",""constraints"":{""k"":{""ik"":[{""time"":0,""v"":[1,0,1,0,0]},{""time"":1,""v"":[0,10,-1,1,1]}]}}}]");
            var pose = new PoseState(m);
            AnimationApplier.Apply(m, m.FindAnimation("go"), 0.5f, null, pose);
            Assert.AreEqual(0.5f, pose.ConstraintValues[0][0], 1e-6f);
            Assert.AreEqual(5f, pose.ConstraintValues[0][1], 1e-5f);
            // 참거짓 값(bend, compress, stretch)은 계단
            Assert.AreEqual(1f, pose.ConstraintValues[0][2]);
            Assert.AreEqual(0f, pose.ConstraintValues[0][3]);
            pose.ResetToSetup(m);
            Assert.AreEqual(1f, pose.ConstraintValues[0][0]);
        }

        const string RotAnim = @"""animations"":[{""name"":""r"",""bones"":{""a"":{""rotate"":[{""time"":0,""v"":[40]}],""translate"":[{""time"":0,""v"":[10,0]}]}},
            ""slots"":{""s"":{""alpha"":[{""time"":0,""v"":[0]}],""attachment"":[{""time"":0}]}}}],
            ""slots"":[{""name"":""s"",""bone"":""a"",""attachment"":""img""}]";

        [Test]
        public void m13_alpha_half_blends_bone_rotation()
        {
            var m = Load(RotAnim);
            var pose = new PoseState(m);
            AnimationApplier.Apply(m, m.FindAnimation("r"), 0, null, pose, 0.5f);
            Assert.AreEqual(20f, pose.Locals[1].Rotation, 1e-5f);
            Assert.AreEqual(5f, pose.Locals[1].X, 1e-5f);
            Assert.AreEqual(0.5f, pose.SlotColors[0].A, 1e-6f);
            // 이산 값은 alpha ≥ 0.5일 때만 바뀐다
            Assert.IsNull(pose.SlotAttachments[0]);
            pose.ResetToSetup(m);
            AnimationApplier.Apply(m, m.FindAnimation("r"), 0, null, pose, 0.4f);
            Assert.AreEqual("img", pose.SlotAttachments[0]);
        }

        [Test]
        public void m13_alpha_blend_takes_short_way_round()
        {
            // 현재 170, 목표 −170(= 셋업 0 + 키 −170): 짧은 쪽(+20)으로 절반 → 180
            var m = Load(@"""animations"":[{""name"":""r"",""bones"":{""a"":{""rotate"":[{""time"":0,""v"":[-170]}]}}}]");
            var pose = new PoseState(m);
            pose.Locals[1].Rotation = 170;
            AnimationApplier.Apply(m, m.FindAnimation("r"), 0, null, pose, 0.5f);
            Assert.AreEqual(180f, pose.Locals[1].Rotation, 1e-4f);
        }

        [Test]
        public void m13_additive_adds_on_current_pose()
        {
            // 가산: 현재 + (목표 − 셋업) × alpha. 현재 회전 15, 키 40 → 15 + 40 × 0.5 = 35
            var m = Load(RotAnim);
            var pose = new PoseState(m);
            pose.Locals[1].Rotation = 15;
            pose.SlotColors[0].A = 0.8f;
            AnimationApplier.Apply(m, m.FindAnimation("r"), 0, null, pose, 0.5f, true);
            Assert.AreEqual(35f, pose.Locals[1].Rotation, 1e-5f);
            // 색: 0.8 + (0 − 1) × 0.5 = 0.3
            Assert.AreEqual(0.3f, pose.SlotColors[0].A, 1e-6f);
        }

        [Test]
        public void m13_skip_constraint_keys()
        {
            var m = Load(@"""constraints"":[{""type"":""ik"",""name"":""k"",""bones"":[""a""],""target"":""t""}],
                ""animations"":[{""name"":""go"",""constraints"":{""k"":{""ik"":[{""time"":0,""v"":[0,0,1,0,0]}]}}}]");
            var pose = new PoseState(m);
            AnimationApplier.Apply(m, m.FindAnimation("go"), 0, null, pose, 1, false, 0);
            Assert.AreEqual(1f, pose.ConstraintValues[0][0]);
        }

        [Test]
        public void m13_skin_required_constraint_skipped_without_skin()
        {
            var m = KGBoneLoader.Load(@"{""formatVersion"":1,""skeleton"":{""bones"":[{""name"":""root""},{""name"":""a"",""parent"":""root""},
                {""name"":""t"",""parent"":""root""},{""name"":""only"",""parent"":""root"",""skinRequired"":true}],
                ""constraints"":[{""type"":""ik"",""name"":""k"",""skinRequired"":true,""bones"":[""a""],""target"":""t""},
                    {""type"":""ik"",""name"":""free"",""bones"":[""a""],""target"":""t""},
                    {""type"":""ik"",""name"":""toOnly"",""bones"":[""a""],""target"":""only""},
                    {""type"":""ik"",""name"":""missing"",""bones"":[""a""],""target"":""없음""}],
                ""skins"":[{""name"":""default""},{""name"":""big"",""bones"":[""only""],""constraints"":[""k""]}]}}");
            var active = new bool[m.Bones.Count];
            var on = new bool[m.Constraints.Count];
            ActiveBones.Compute(m, null, active);
            ConstraintSolvers.ActiveConstraints(m, active, null, on);
            CollectionAssert.AreEqual(new[] { false, true, false, false }, on);
            ActiveBones.Compute(m, "big", active);
            ConstraintSolvers.ActiveConstraints(m, active, "big", on);
            CollectionAssert.AreEqual(new[] { true, true, true, false }, on);
        }

        [Test]
        public void m13_world_to_local_round_trip()
        {
            var parent = BoneTransform.WorldOf(new LocalTransform { X = 3, Y = 4, Rotation = 30, ScaleX = 2, ScaleY = 1, ShearY = 10 }, Inherit.Normal, null);
            var local = new LocalTransform { X = 5, Y = -2, Rotation = 50, ScaleX = 1.5f, ScaleY = 0.5f };
            var w = BoneTransform.WorldOf(local, Inherit.Normal, parent);
            var back = BoneTransform.WorldToLocal(w, parent, Inherit.Normal);
            var w2 = BoneTransform.WorldOf(back, Inherit.Normal, parent);
            Assert.AreEqual(w.A, w2.A, 1e-4f);
            Assert.AreEqual(w.B, w2.B, 1e-4f);
            Assert.AreEqual(w.C, w2.C, 1e-4f);
            Assert.AreEqual(w.D, w2.D, 1e-4f);
            Assert.AreEqual(w.X, w2.X, 1e-4f);
            Assert.AreEqual(w.Y, w2.Y, 1e-4f);
            Assert.AreEqual(0f, back.ShearX);
        }

        [Test]
        public void m13_wrap_deg_and_world_parts()
        {
            Assert.AreEqual(-170f, WorldMath.WrapDeg(190));
            Assert.AreEqual(170f, WorldMath.WrapDeg(-190));
            Assert.AreEqual(180f, WorldMath.WrapDeg(180));
            var w = WorldMath.FromParts(30, 2, 0.5f, 10, 0, 0);
            Assert.AreEqual(30f, w.RotationX, 1e-4f);
            Assert.AreEqual(2f, w.ScaleX, 1e-5f);
            Assert.AreEqual(0.5f, w.ScaleY, 1e-5f);
            Assert.AreEqual(10f, WorldMath.WorldShearY(w), 1e-3f);
        }

        // ---------- M13 IK (핵심 pose/ik.rs 테스트와 같은 값) ----------
        static PoseResult Pose(string kgbone) => new SkeletonPoser(KGBoneLoader.Load(kgbone)).Compute(new PoseRequest());

        static string IkOne(string opts, float len, float tx, float ty) =>
            $@"{{""formatVersion"":1,""skeleton"":{{""bones"":[{{""name"":""root""}},{{""name"":""a"",""parent"":""root"",""length"":{F(len)}}},{{""name"":""t"",""parent"":""root"",""x"":{F(tx)},""y"":{F(ty)}}}],
                ""constraints"":[{{""type"":""ik"",""name"":""ik"",""bones"":[""a""],""target"":""t""{opts}}}]}}}}";

        static string IkTwo(string opts, float tx, float ty) =>
            $@"{{""formatVersion"":1,""skeleton"":{{""bones"":[{{""name"":""root""}},{{""name"":""p"",""parent"":""root"",""length"":10}},{{""name"":""c"",""parent"":""p"",""x"":10,""length"":10}},
                {{""name"":""t"",""parent"":""root"",""x"":{F(tx)},""y"":{F(ty)}}}],
                ""constraints"":[{{""type"":""ik"",""name"":""ik"",""bones"":[""p"",""c""],""target"":""t""{opts}}}]}}}}";

        static string F(float v) => v.ToString(System.Globalization.CultureInfo.InvariantCulture);

        [Test]
        public void m13_ik_f_f01_one_bone_ik()
        {
            var o = Pose(IkOne("", 2, 3, 4));
            Assert.AreEqual(53.1301f, o.Locals[1].Rotation, 1e-3f);
            Assert.AreEqual(1f, o.Locals[1].ScaleX, 1e-3f);
            Assert.AreEqual(26.5651f, Pose(IkOne(@",""mix"":0.5", 2, 3, 4)).Locals[1].Rotation, 1e-3f);
            o = Pose(IkOne(@",""stretch"":true,""uniform"":true", 2, 3, 4));
            Assert.AreEqual(2.5f, o.Locals[1].ScaleX, 1e-3f);
            Assert.AreEqual(2.5f, o.Locals[1].ScaleY, 1e-3f);
            Assert.AreEqual(0.5f, Pose(IkOne(@",""compress"":true", 10, 3, 4)).Locals[1].ScaleX, 1e-3f);
            Assert.AreEqual(1f, Pose(IkOne(@",""stretch"":true", 10, 3, 4)).Locals[1].ScaleX, 1e-3f);
        }

        [Test]
        public void m13_ik_f_f02_ik_volume()
        {
            var o = Pose(IkOne(@",""stretch"":true,""volume"":true", 10, 20, 0));
            Assert.AreEqual(2f, o.Locals[1].ScaleX, 1e-3f);
            Assert.AreEqual(0.5f, o.Locals[1].ScaleY, 1e-3f);
            var (x, y) = o.Bones[1].LocalToWorld(10, 0);
            Assert.AreEqual(20f, x, 1e-3f);
            Assert.AreEqual(0f, y, 1e-3f);
        }

        [Test]
        public void m13_ik_f_f01_two_bone_ik()
        {
            var o = Pose(IkTwo("", 10, 10));
            Assert.AreEqual(0f, o.Locals[1].Rotation, 1e-3f);
            Assert.AreEqual(90f, o.Locals[2].Rotation, 1e-3f);
            var (x, y) = o.Bones[2].LocalToWorld(10, 0);
            Assert.AreEqual(10f, x, 1e-3f);
            Assert.AreEqual(10f, y, 1e-3f);
            o = Pose(IkTwo(@",""bendPositive"":false", 10, 10));
            Assert.AreEqual(90f, o.Locals[1].Rotation, 1e-3f);
            Assert.AreEqual(-90f, o.Locals[2].Rotation, 1e-3f);
            Assert.AreEqual(0f, o.Bones[2].X, 1e-3f);
            Assert.AreEqual(10f, o.Bones[2].Y, 1e-3f);
            o = Pose(IkTwo("", 30, 0));
            Assert.AreEqual(0f, o.Locals[1].Rotation, 1e-3f);
            Assert.AreEqual(0f, o.Locals[2].Rotation, 1e-3f);
            o = Pose(IkTwo(@",""stretch"":true", 30, 0));
            Assert.AreEqual(1.5f, o.Locals[1].ScaleX, 1e-3f);
            Assert.AreEqual(30f, o.Bones[2].LocalToWorld(10, 0).x, 1e-3f);
            o = Pose(IkTwo(@",""softness"":5", 20, 0));
            Assert.AreEqual(-24.766f, o.Locals[1].Rotation, 1e-2f);
            (x, y) = o.Bones[2].LocalToWorld(10, 0);
            Assert.AreEqual(18.1606f, x, 1e-2f);
            Assert.AreEqual(0f, y, 1e-2f);
        }

        // ---------- M13 트랜스폼 (핵심 pose/transform.rs 테스트와 같은 값) ----------
        static string Tc(bool local, bool clamp, float dialX, string extra = @",""mixX"":0,""mixY"":0,""mixScaleX"":0,""mixScaleY"":0,""mixShearY"":0",
            string props = @",""properties"":[{""from"":""x"",""to"":[{""property"":""rotate"",""max"":90,""scale"":2}]}]") =>
            $@"{{""formatVersion"":1,""skeleton"":{{""bones"":[{{""name"":""root"",""x"":100}},{{""name"":""dial"",""parent"":""root"",""x"":{F(dialX)}}},{{""name"":""gem"",""parent"":""root"",""rotation"":10}}],
                ""constraints"":[{{""type"":""transform"",""name"":""tc"",""bones"":[""gem""],""source"":""dial"",""localSource"":{(local ? "true" : "false")},""localTarget"":{(local ? "true" : "false")},""clamp"":{(clamp ? "true" : "false")}{extra}{props}}}]}}}}";

        [Test]
        public void m13_transform_f_f03_translate_x_to_rotate()
        {
            Assert.AreEqual(60f, Pose(Tc(true, false, 30)).Locals[2].Rotation, 1e-3f);
            Assert.AreEqual(90f, Pose(Tc(true, true, 60)).Locals[2].Rotation, 1e-3f);
            Assert.AreEqual(90f, Pose(Tc(false, true, 0.2f)).Bones[2].RotationX, 1e-3f);
            var o = Pose(Tc(false, false, -80));
            Assert.AreEqual(40f, o.Bones[2].RotationX, 1e-3f);
            Assert.AreEqual(100f, o.Bones[2].X, 1e-3f);
            Assert.AreEqual(0f, o.Bones[2].Y, 1e-3f);
        }

        [Test]
        public void m13_transform_f_f03_mix_and_additive_and_default_mapping()
        {
            const string zero = @",""mixX"":0,""mixY"":0,""mixScaleX"":0,""mixScaleY"":0,""mixShearY"":0";
            Assert.AreEqual(35f, Pose(Tc(true, false, 30, zero + @",""mixRotate"":0.5")).Locals[2].Rotation, 1e-3f);
            Assert.AreEqual(40f, Pose(Tc(true, false, 30, zero + @",""mixRotate"":0.5,""additive"":true")).Locals[2].Rotation, 1e-3f);
            var o = Pose(Tc(false, false, 30, @",""mixScaleX"":0,""mixScaleY"":0,""mixShearY"":0", ""));
            Assert.AreEqual(130f, o.Bones[2].X, 1e-3f);
            Assert.AreEqual(0f, o.Bones[2].RotationX, 1e-3f);
        }

        // ---------- M13 슬라이더 (핵심 pose/slider.rs 테스트와 같은 값) ----------
        static string Sl(string opts, float dialY, string extraAnims = "") =>
            $@"{{""formatVersion"":1,""skeleton"":{{""bones"":[{{""name"":""root""}},{{""name"":""arm"",""parent"":""root"",""length"":10,""rotation"":10}},{{""name"":""dial"",""parent"":""root"",""y"":{F(dialY)}}}],
                ""constraints"":[{{""type"":""slider"",""name"":""sl"",""animation"":""swing"",""time"":0.5,""mix"":0.75,""property"":""y"",""propertyFrom"":0,""propertyTo"":100{opts}}}],
                ""animations"":[{{""name"":""swing"",""bones"":{{""arm"":{{""rotate"":[{{""time"":0,""v"":[0]}},{{""time"":2,""v"":[80]}}]}}}}}}{extraAnims}]}}}}";

        [Test]
        public void m13_slider_f_f06_slider_equals_animation_with_mix()
        {
            Assert.AreEqual(25f, Pose(Sl("", 0)).Locals[1].Rotation, 1e-3f);
            Assert.AreEqual(40f, Pose(Sl(@",""bone"":""dial""", 50)).Locals[1].Rotation, 1e-3f);
            Assert.AreEqual(70f, Pose(Sl(@",""bone"":""dial""", 300)).Locals[1].Rotation, 1e-3f);
            Assert.AreEqual(10f, Pose(Sl(@",""bone"":""dial"",""loop"":true", 300)).Locals[1].Rotation, 1e-3f);
            Assert.AreEqual(25f, Pose(Sl(@",""additive"":true", 0)).Locals[1].Rotation, 1e-3f);
            var poser = new SkeletonPoser(KGBoneLoader.Load(Sl("", 0, @",{""name"":""drive"",""constraints"":{""sl"":{""time"":[{""time"":0,""v"":[1.5]}]}}}")));
            Assert.AreEqual(55f, poser.Compute(new PoseRequest { Animation = "drive" }).Locals[1].Rotation, 1e-3f);
        }

        [Test]
        public void m13_slider_skips_own_keys()
        {
            // 적용하는 애니메이션 swing 안에 sl 자신의 time·mix 키가 있다: 순환 없이 건너뛰고 본 키만 적용된다
            var text = Sl("", 0).Replace(@"""rotate"":[{""time"":0,""v"":[0]},{""time"":2,""v"":[80]}]}}",
                @"""rotate"":[{""time"":0,""v"":[0]},{""time"":2,""v"":[80]}]}},""constraints"":{""sl"":{""time"":[{""time"":0,""v"":[1.9]}],""mix"":[{""time"":0,""v"":[0]}]}}");
            var m = KGBoneLoader.Load(text);
            Assert.AreEqual(2, m.FindAnimation("swing").Constraints.Count);
            var o = new SkeletonPoser(m).Compute(new PoseRequest());
            Assert.AreEqual(25f, o.Locals[1].Rotation, 1e-3f);
            // swing 자체를 재생하면 자기 키(time 1.9, mix 0)가 제약 값을 바꾼다: 그 뒤 슬라이더는 mix 0이라 아무것도 안 함
            o = new SkeletonPoser(m).Compute(new PoseRequest { Animation = "swing", Time = 1 });
            Assert.AreEqual(50f, o.Locals[1].Rotation, 1e-3f);
        }

        [Test]
        public void m10_constraint_warnings_only_unsupported_types()
        {
            var m = KGBoneLoader.Load(System.IO.File.ReadAllText(System.IO.Path.Combine(GoldenFile.RepoRoot(), "samples/kgbone/full-4.3.kgbone")));
            // 2단계: 제약 5종을 모두 계산하므로 제약 경고가 없다
            CollectionAssert.IsEmpty(m.ConstraintWarnings());
            var ik = KGBoneLoader.Load(System.IO.File.ReadAllText(System.IO.Path.Combine(GoldenFile.RepoRoot(), "unity/com.kgbone.runtime/Tests/Golden/spine43-ik.kgbone")));
            CollectionAssert.IsEmpty(ik.ConstraintWarnings());
        }

        // ---------- M14 패스 (핵심 pose/path.rs 테스트와 같은 값) ----------
        static string Pc(string spacing, string rotate, float position, float space, bool vertical) =>
            $@"{{""formatVersion"":1,""skeleton"":{{""bones"":[{{""name"":""root""}},{{""name"":""b0"",""parent"":""root"",""length"":20}},{{""name"":""b1"",""parent"":""b0"",""length"":20,""x"":20}},
                {{""name"":""b2"",""parent"":""b1"",""length"":20,""x"":20}},{{""name"":""pathbone"",""parent"":""root""{(vertical ? @",""rotation"":90" : "")}}}],
                ""slots"":[{{""name"":""path"",""bone"":""pathbone"",""attachment"":""p""}}],
                ""skins"":[{{""name"":""default"",""attachments"":{{""path"":{{""p"":{{""type"":""path"",""vertexCount"":6,""vertices"":[-10,0,0,0,33.333333333,0,66.666666667,0,100,0,110,0],""lengths"":[100]}}}}}}}}],
                ""constraints"":[{{""type"":""path"",""name"":""pc"",""bones"":[""b0"",""b1"",""b2""],""target"":""path"",""spacingMode"":""{spacing}"",""rotateMode"":""{rotate}"",""position"":{F(position)},""spacing"":{F(space)}}}]}}}}";

        [Test]
        public void m14_path_f_f04_straight_path_positions()
        {
            var o = Pose(Pc("length", "tangent", 0.1f, 5, false));
            float[] xs = { 10, 35, 60 };
            for (int k = 0; k < 3; k++)
            {
                Assert.AreEqual(xs[k], o.Bones[1 + k].X, 1e-2f, $"x{k}");
                Assert.AreEqual(0f, o.Bones[1 + k].Y, 1e-2f, $"y{k}");
                Assert.AreEqual(0f, o.Bones[1 + k].RotationX, 1e-2f, $"r{k}");
            }
            Assert.AreEqual(70f, Pose(Pc("fixed", "chain", 0.1f, 30, false)).Bones[3].X, 1e-2f);
            o = Pose(Pc("percent", "tangent", 0.1f, 0.2f, false));
            Assert.AreEqual(30f, o.Bones[2].X, 1e-2f);
            Assert.AreEqual(50f, o.Bones[3].X, 1e-2f);
            Assert.AreEqual(50f, Pose(Pc("proportional", "tangent", 0.1f, 0.6f, false)).Bones[3].X, 1e-2f);
            Assert.AreEqual(1.5f, Pose(Pc("fixed", "chainScale", 0.1f, 30, false)).Bones[1].ScaleX, 1e-2f);
            o = Pose(Pc("length", "tangent", 0.1f, 5, true));
            float[] ys = { 10, 35, 60 };
            for (int k = 0; k < 3; k++)
            {
                Assert.AreEqual(0f, o.Bones[1 + k].X, 1e-2f);
                Assert.AreEqual(ys[k], o.Bones[1 + k].Y, 1e-2f);
                Assert.AreEqual(90f, o.Bones[1 + k].RotationX, 1e-2f);
            }
        }

        [Test]
        public void m14_path_without_attachment_does_nothing()
        {
            // 대상 슬롯의 첨부물이 패스가 아니거나 없으면 본은 그대로(예외 없음)
            var o = Pose(Pc("length", "tangent", 0.1f, 5, false).Replace(@"""attachment"":""p""", @"""attachment"":""없음"""));
            Assert.AreEqual(0f, o.Bones[1].X, 1e-6f);
            Assert.AreEqual(20f, o.Bones[2].X, 1e-6f);
        }

        // ---------- M15 물리 (핵심 pose/physics.rs 테스트와 같은 값) ----------
        static string Phys(float gravity, string extra = "", string anims = "") =>
            $@"{{""formatVersion"":1,""skeleton"":{{""bones"":[{{""name"":""root""}},{{""name"":""tail"",""parent"":""root"",""length"":50}}],
                ""constraints"":[{{""type"":""physics"",""name"":""phys"",""bone"":""tail"",""x"":1,""y"":1,""rotate"":1,""scaleX"":0.5,""limit"":5000,""fps"":60,
                    ""inertia"":1,""strength"":100,""damping"":1,""mass"":1,""gravity"":{F(gravity)},""mix"":1,""volume"":true}}{extra}]{anims}}}}}";

        static WorldBone[][] Run(SkeletonPoser p, int steps, PoseRequest template = null)
        {
            var o = new WorldBone[steps][];
            for (int i = 0; i < steps; i++)
            {
                var r = template ?? new PoseRequest();
                r.Physics = PhysicsStep.Advance(1f / 60f);
                o[i] = (WorldBone[])p.Compute(r).Bones.Clone();
            }
            return o;
        }

        static void SameBones(WorldBone[] a, WorldBone[] b, float tol = 0f)
        {
            Assert.AreEqual(a.Length, b.Length);
            for (int i = 0; i < a.Length; i++)
            {
                Assert.AreEqual(a[i].A, b[i].A, tol); Assert.AreEqual(a[i].B, b[i].B, tol); Assert.AreEqual(a[i].C, b[i].C, tol);
                Assert.AreEqual(a[i].D, b[i].D, tol); Assert.AreEqual(a[i].X, b[i].X, tol); Assert.AreEqual(a[i].Y, b[i].Y, tol);
            }
        }

        [Test]
        public void m15_physics_f_f05_deterministic_and_still()
        {
            var m = KGBoneLoader.Load(Phys(30));
            var a = Run(new SkeletonPoser(m), 120);
            var b = Run(new SkeletonPoser(m), 120);
            for (int i = 0; i < 120; i++) SameBones(a[i], b[i]);
            Assert.Less(a[119][1].Y, -0.01f);
            var still = KGBoneLoader.Load(Phys(0));
            var outs = Run(new SkeletonPoser(still), 120);
            SameBones(outs[119], (WorldBone[])new SkeletonPoser(still).Compute(new PoseRequest()).Bones.Clone());
            var e = new SkeletonPoser(m);
            var first = Run(e, 120);
            e.Compute(new PoseRequest { Physics = PhysicsStep.Reset });
            var again = Run(e, 120);
            SameBones(first[119], again[119]);
            e.ResetPhysics();
            var again2 = Run(e, 120);
            SameBones(first[60], again2[60]);
            var e2 = new SkeletonPoser(m);
            e2.Compute(new PoseRequest());
            var big = (WorldBone[])e2.Compute(new PoseRequest { Physics = PhysicsStep.Advance(2f) }).Bones.Clone();
            SameBones(big, first[119], 1e-3f);
        }

        [Test]
        public void m15_physics_none_keeps_state()
        {
            // 진행하지 않는 요청(None)은 상태를 그대로 두고 마지막 처짐을 그대로 보여 준다
            var p = new SkeletonPoser(KGBoneLoader.Load(Phys(30)));
            var a = Run(p, 30);
            var held = (WorldBone[])p.Compute(new PoseRequest()).Bones.Clone();
            SameBones(a[29], held);
        }

        [Test]
        public void m15_reset_key_resets_named_constraint_only()
        {
            // 서로 다른 본의 물리 제약 두 개, phys에만 0.5초 reset 키. 구간 [0.4, 0.6)을 지나는 요청(진행 없음)에서
            // phys는 처음 상태(셋업 위치), other는 처진 그대로여야 한다
            var m = KGBoneLoader.Load(@"{""formatVersion"":1,""skeleton"":{""bones"":[{""name"":""root""},{""name"":""tail"",""parent"":""root"",""length"":50},{""name"":""tail2"",""parent"":""root"",""length"":50,""y"":100}],
                ""constraints"":[{""type"":""physics"",""name"":""phys"",""bone"":""tail"",""x"":1,""y"":1,""gravity"":30},{""type"":""physics"",""name"":""other"",""bone"":""tail2"",""x"":1,""y"":1,""gravity"":30}],
                ""animations"":[{""name"":""a"",""constraints"":{""phys"":{""reset"":[{""time"":0.5}]}},""bones"":{""root"":{""rotate"":[{""time"":1,""v"":[0]}]}}}]}}");
            var p = new SkeletonPoser(m);
            Run(p, 30, new PoseRequest { Animation = "a", Time = 0.2f });
            var held = (WorldBone[])p.Compute(new PoseRequest { Animation = "a", Time = 0.4f }).Bones.Clone();
            Assert.Less(held[1].Y, -0.01f);
            Assert.Less(held[2].Y, 100f - 0.01f);
            var after = p.Compute(new PoseRequest { Animation = "a", Time = 0.6f, LastTime = 0.4f }).Bones;
            Assert.AreEqual(0f, after[1].Y, 1e-6f, "phys는 초기화");
            Assert.AreEqual(held[2].Y, after[2].Y, 1e-6f, "other는 그대로");
        }

        [Test]
        public void m15_empty_name_reset_key_resets_all()
        {
            var m = KGBoneLoader.Load(@"{""formatVersion"":1,""skeleton"":{""bones"":[{""name"":""root""},{""name"":""tail"",""parent"":""root"",""length"":50},{""name"":""tail2"",""parent"":""root"",""length"":50,""y"":100}],
                ""constraints"":[{""type"":""physics"",""name"":""phys"",""bone"":""tail"",""y"":1,""gravity"":30},{""type"":""physics"",""name"":""other"",""bone"":""tail2"",""y"":1,""gravity"":30}],
                ""animations"":[{""name"":""a"",""constraints"":{"""":{""reset"":[{""time"":0.5}]}},""bones"":{""root"":{""rotate"":[{""time"":1,""v"":[0]}]}}}]}}");
            var p = new SkeletonPoser(m);
            Run(p, 30, new PoseRequest { Animation = "a", Time = 0.2f });
            var after = p.Compute(new PoseRequest { Animation = "a", Time = 0.6f, LastTime = 0.4f }).Bones;
            Assert.AreEqual(0f, after[1].Y, 1e-6f);
            Assert.AreEqual(100f, after[2].Y, 1e-6f);
        }

        // ---------- M16 월드 움직임 반영(계산부) ----------
        const string StillPhys = @"{""formatVersion"":1,""skeleton"":{""bones"":[{""name"":""root""},{""name"":""tail"",""parent"":""root"",""length"":50}],
            ""constraints"":[{""type"":""physics"",""name"":""p"",""bone"":""tail"",""x"":1,""y"":1,""rotate"":1,""strength"":100,""damping"":1}]}}";

        static PoseRequest Adv(float dt, float wx = 0, float wy = 0, float wr = 0) =>
            new PoseRequest { Physics = PhysicsStep.Advance(dt), WorldMotion = new WorldMotion { X = wx, Y = wy, Rotation = wr } };

        [Test]
        public void m16_zero_world_motion_matches_golden()
        {
            // WorldMotion을 명시적으로 0으로 넣어도 연속 정답과 같다
            foreach (var path in GoldenFile.All())
            {
                var root = JsonValue.Parse(System.IO.File.ReadAllText(path));
                if (root.Get("stateful")?.AsBool() != true) continue;
                var m = KGBoneLoader.Load(System.IO.File.ReadAllText(System.IO.Path.Combine(System.IO.Path.GetDirectoryName(path), root.Get("kgbone").AsString())));
                var a = new SkeletonPoser(m);
                var b = new SkeletonPoser(m);
                for (int i = 0; i < 120; i++)
                {
                    var ra = new PoseRequest { Animation = m.Animations.Count > 0 ? m.Animations[0].Name : null, Time = i / 60f, Looping = true, Physics = PhysicsStep.Advance(1 / 60f) };
                    var rb = new PoseRequest { Animation = ra.Animation, Time = ra.Time, Looping = true, Physics = ra.Physics, WorldMotion = new WorldMotion() };
                    SameBones((WorldBone[])a.Compute(ra).Bones.Clone(), (WorldBone[])b.Compute(rb).Bones.Clone());
                }
            }
        }

        [Test]
        public void m16_move_right_makes_tip_lag_left()
        {
            var p = new SkeletonPoser(KGBoneLoader.Load(StillPhys));
            p.Compute(Adv(1 / 60f));
            var o = p.Compute(Adv(1 / 60f, 10));
            Assert.Less(o.Bones[1].X, -0.01f, "원점 질점이 왼쪽으로 뒤처진다");
            for (int i = 0; i < 3; i++) o = p.Compute(Adv(1 / 60f));
            Assert.Less(o.Bones[1].X, -0.01f);
            // 아래로 처지는 쪽(꼬리가 왼쪽으로 남음 = 끝도 왼쪽): 오른쪽을 향한 꼬리는 크게 돌지 않지만 위치는 뒤처진다
            Assert.AreEqual(0f, o.Bones[1].Y, 1e-4f);
        }

        [Test]
        public void m16_half_factor_halves_first_offset()
        {
            var full = new SkeletonPoser(KGBoneLoader.Load(StillPhys));
            var half = new SkeletonPoser(KGBoneLoader.Load(StillPhys));
            full.Compute(Adv(1 / 60f));
            half.Compute(Adv(1 / 60f));
            float a = full.Compute(Adv(1 / 60f, 10)).Bones[1].X;
            float b = half.Compute(Adv(1 / 60f, 5)).Bones[1].X;
            Assert.AreEqual(0.5f, b / a, 1e-4f);
        }

        [Test]
        public void m16_rotation_motion_moves_tip()
        {
            // 본 원점 = 뼈대 원점이라 원점 질점은 회전으로 움직이지 않고, 끝 질점만 뒤처져 본이 반대로 돈다
            var p = new SkeletonPoser(KGBoneLoader.Load(StillPhys));
            p.Compute(Adv(1 / 60f));
            var o = p.Compute(Adv(1 / 60f, 0, 0, 10));
            Assert.AreEqual(0f, o.Bones[1].X, 1e-5f);
            Assert.AreEqual(0f, o.Bones[1].Y, 1e-5f);
            Assert.Less(o.Bones[1].RotationX, -0.01f);
        }

        [Test]
        public void m16_world_motion_waits_for_a_step()
        {
            // 진행 단계가 0인 프레임(dt 0)의 월드 움직임은 다음 단계로 미뤄진다: dt 0 + 이동 10, 그다음 1/60 = 한 번에 1/60 + 이동 10
            var a = new SkeletonPoser(KGBoneLoader.Load(StillPhys));
            var b = new SkeletonPoser(KGBoneLoader.Load(StillPhys));
            a.Compute(Adv(1 / 60f)); b.Compute(Adv(1 / 60f));
            a.Compute(Adv(0, 10));
            float x1 = a.Compute(Adv(1 / 60f)).Bones[1].X;
            float x2 = b.Compute(Adv(1 / 60f, 10)).Bones[1].X;
            Assert.Less(x2, -0.01f);
            Assert.AreEqual(x2, x1, 1e-5f);
        }

        [Test]
        public void m16_reset_clears_pending_world_motion()
        {
            var p = new SkeletonPoser(KGBoneLoader.Load(StillPhys));
            p.Compute(Adv(1 / 60f));
            p.Compute(Adv(0, 50));
            // 초기화하지 않으면 쌓인 움직임이 다음 단계에 나타난다
            var control = new SkeletonPoser(KGBoneLoader.Load(StillPhys));
            control.Compute(Adv(1 / 60f));
            control.Compute(Adv(0, 50));
            Assert.Less(control.Compute(Adv(1 / 60f)).Bones[1].X, -0.01f);
            p.ResetPhysics();
            var o = p.Compute(Adv(1 / 60f));
            Assert.AreEqual(0f, o.Bones[1].X, 1e-6f);
            Assert.AreEqual(0f, o.Bones[1].RotationX, 1e-6f);
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
