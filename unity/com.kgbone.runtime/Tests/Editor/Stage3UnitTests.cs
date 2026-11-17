// 유니티 런타임 3단계 손 계산 테스트(시퀀스·클리핑·트랙·스킨 합치기·덮어쓰기). dotnet test와 유니티 EditMode 양쪽에서 돈다(순수 C#, NUnit).
using System.Collections.Generic;
using System.Linq;
using KGBone.Core;
using NUnit.Framework;

namespace KGBone.Tests
{
    public class Stage3UnitTests
    {
        static Key[] SeqKey(string mode, float start, float delay, float time = 0) =>
            new[] { new Key { Time = time, S = mode, V = new[] { start, delay } } };

        // ---------- M17 시퀀스 ----------
        [Test]
        public void m17_sequence_frame_modes()
        {
            int F(string mode, float t) => SequenceKeys.Frame(SeqKey(mode, 0, 0.1f), t, 4).Value;
            Assert.AreEqual(0, F("hold", 0.25f));
            Assert.AreEqual(2, F("once", 0.25f));
            Assert.AreEqual(2, F("loop", 0.25f));
            Assert.AreEqual(2, F("pingpong", 0.25f));
            Assert.AreEqual(1, F("onceReverse", 0.25f));
            Assert.AreEqual(1, F("loopReverse", 0.25f));
            Assert.AreEqual(1, F("pingpongReverse", 0.25f));
            Assert.AreEqual(3, F("once", 0.55f));
            Assert.AreEqual(1, F("loop", 0.55f));
            Assert.AreEqual(1, F("pingpong", 0.55f));
        }

        [Test]
        public void m17_sequence_delay_zero_holds_start()
        {
            Assert.AreEqual(2, SequenceKeys.Frame(SeqKey("loop", 2, 0), 0.7f, 4).Value);
            Assert.IsNull(SequenceKeys.Frame(new Key[0], 0.7f, 4));
        }

        [Test]
        public void m17_image_path_digits()
        {
            var r = new RegionAttachment { Path = "base", Sequence = new SequenceData { Count = 5, Start = 1, Digits = 2 } };
            Assert.AreEqual("base04", r.ImagePath("x", 3));
            r.Sequence.Digits = 0;
            Assert.AreEqual("base4", r.ImagePath("x", 3));
            Assert.AreEqual("base1", r.ImagePath("x", null));
        }

        [Test]
        public void m17_image_paths_include_all_frames()
        {
            var m = KGBoneLoader.Load(@"{""formatVersion"":1,""skeleton"":{""bones"":[{""name"":""root""}],""slots"":[{""name"":""s"",""bone"":""root"",""attachment"":""a""}],
                ""skins"":[{""name"":""default"",""attachments"":{""s"":{""a"":{""path"":""f"",""sequence"":{""count"":4,""start"":1}}}}}]}}");
            CollectionAssert.AreEqual(new[] { "f1", "f2", "f3", "f4" }, KGBoneLoader.ImagePaths(m).ToArray());
        }

        // ---------- M18 삼각분할·클리핑 ----------
        static readonly float[] LShape = { 0, 0, 20, 0, 20, 5, 5, 5, 5, 20, 0, 20 };
        static readonly float[] Square = { -10, -10, 10, -10, 10, 10, -10, 10 };
        static readonly float[] SquareUv = { 0, 1, 1, 1, 1, 0, 0, 0 };
        static readonly int[] SquareIdx = { 0, 1, 2, 2, 3, 0 };

        static float Area(IList<float> p, IList<int> idx)
        {
            float s = 0;
            for (int t = 0; t + 2 < idx.Count; t += 3)
            {
                float ax = p[2 * idx[t]], ay = p[2 * idx[t] + 1], bx = p[2 * idx[t + 1]], by = p[2 * idx[t + 1] + 1], cx = p[2 * idx[t + 2]], cy = p[2 * idx[t + 2] + 1];
                s += System.Math.Abs((bx - ax) * (cy - ay) - (by - ay) * (cx - ax)) * 0.5f;
            }
            return s;
        }

        static (List<float> p, List<float> u, List<int> i) ClipSquare(float[] poly, bool convex, bool inverse, float[] square = null)
        {
            var c = Clipper.Create(poly, convex, inverse);
            var p = new List<float>(square ?? Square);
            var u = new List<float>(SquareUv);
            var i = new List<int>(SquareIdx);
            c.Clip(p, u, i);
            return (p, u, i);
        }

        [Test]
        public void m18_triangulate_concave()
        {
            var t = Triangulator.Triangulate(LShape);
            Assert.AreEqual(12, t.Length);
            Assert.AreEqual(175f, Area(LShape, t), 1e-3f);
            var rev = new float[LShape.Length];
            for (int k = 0; k < LShape.Length / 2; k++) { rev[2 * k] = LShape[LShape.Length - 2 - 2 * k]; rev[2 * k + 1] = LShape[LShape.Length - 1 - 2 * k]; }
            Assert.AreEqual(175f, Area(rev, Triangulator.Triangulate(rev)), 1e-3f);
            // 일직선 점·겹친 점: 넓이 0 삼각형 없음
            var deg = new float[] { 0, 0, 5, 0, 10, 0, 10, 10, 10, 10, 0, 10 };
            var t3 = Triangulator.Triangulate(deg);
            Assert.AreEqual(100f, Area(deg, t3), 1e-3f);
            for (int k = 0; k < t3.Length; k += 3) Assert.Greater(Area(deg, new[] { t3[k], t3[k + 1], t3[k + 2] }), 0f);
        }

        [Test]
        public void m18_clip_square_by_concave_polygon()
        {
            // 핵심 clipping.rs의 clip_square_by_concave_polygon과 같은 입력·기대값
            var (p, u, i) = ClipSquare(LShape, false, false);
            Assert.AreEqual(75f, Area(p, i), 1e-3f);
            Assert.AreEqual(325f, Area(ClipSquare(LShape, false, true).p, ClipSquare(LShape, false, true).i), 1e-2f);
            var cv = ClipSquare(LShape, true, false);
            Assert.AreEqual(100f, Area(cv.p, cv.i), 1e-3f);
        }

        [Test]
        public void m18_uv_barycentric()
        {
            var (p, u, _) = ClipSquare(LShape, false, false);
            Assert.Greater(p.Count, 0);
            // uv: (x, y) → (x/20 + 0.5, 0.5 − y/20)
            for (int k = 0; k < p.Count / 2; k++)
            {
                Assert.AreEqual(p[2 * k] / 20f + 0.5f, u[2 * k], 1e-4f);
                Assert.AreEqual(0.5f - p[2 * k + 1] / 20f, u[2 * k + 1], 1e-4f);
            }
        }

        [Test]
        public void m18_inverse_removes_inside()
        {
            var sq = new float[] { 0, 0, 10, 0, 10, 10, 0, 10 };
            var hole = new float[] { 3, 3, 7, 3, 7, 7, 3, 7 };
            var r = ClipSquare(hole, false, true, sq);
            Assert.AreEqual(84f, Area(r.p, r.i), 1e-3f);
        }

        [Test]
        public void m18_degenerate_polygon_no_clip()
        {
            Assert.IsNull(Clipper.Create(new float[] { 0, 0, 1, 1 }, false, false));
            var flat = ClipSquare(new float[] { 0, 0, 5, 0, 10, 0 }, false, false);
            CollectionAssert.IsEmpty(flat.i);
            var flatConvex = ClipSquare(new float[] { 0, 0, 5, 0, 10, 0 }, true, false);
            CollectionAssert.IsEmpty(flatConvex.i);
        }

        // ---------- M19 트랙 계산부 ----------
        // b1·b2 회전(셋업 0°). 걷기: 0→90(1초), 달리기: 0→−90(1초), 팔: b1만 30, 짧은: b2만 10→20(0.1초)
        const string TrackModelJson = @"{""formatVersion"":1,""skeleton"":{""bones"":[{""name"":""root""},{""name"":""b1"",""parent"":""root""},{""name"":""b2"",""parent"":""root""}],
            ""events"":[{""name"":""e""}],
            ""animations"":[
              {""name"":""걷기"",""bones"":{""b1"":{""rotate"":[{""time"":0,""v"":[0]},{""time"":1,""v"":[90]}]},""b2"":{""rotate"":[{""time"":0,""v"":[0]},{""time"":1,""v"":[90]}]}},""events"":[{""time"":0.5,""name"":""e""}]},
              {""name"":""달리기"",""bones"":{""b1"":{""rotate"":[{""time"":0,""v"":[0]},{""time"":1,""v"":[-90]}]},""b2"":{""rotate"":[{""time"":0,""v"":[0]},{""time"":1,""v"":[-90]}]}},""events"":[{""time"":0.05,""name"":""e""}]},
              {""name"":""팔"",""bones"":{""b1"":{""rotate"":[{""time"":0,""v"":[30]},{""time"":1,""v"":[30]}]}}},
              {""name"":""짧은"",""bones"":{""b2"":{""rotate"":[{""time"":0,""v"":[10]},{""time"":0.1,""v"":[20]}]}}}
            ]}}";

        sealed class TrackRig
        {
            public readonly SkeletonModel M = KGBoneLoader.Load(TrackModelJson);
            public readonly AnimationTracks T;
            public readonly PoseState Pose;
            public readonly List<TrackCompletion> Done = new List<TrackCompletion>();
            public TrackRig() { T = new AnimationTracks(M); Pose = new PoseState(M); }
            /// <summary>dt 진행 후 포즈를 계산하고 (b1, b2) 회전을 돌려준다. dt 0이면 진행 없이 계산만.</summary>
            public (float b1, float b2) Step(float dt)
            {
                if (dt != 0f) T.Update(dt, Done);
                Pose.ResetToSetup(M);
                T.Apply(Pose, SkinSet.Single(null));
                return (Pose.Locals[1].Rotation, Pose.Locals[2].Rotation);
            }
        }

        [Test]
        public void m19_single_track_matches_plain_apply()
        {
            var r = new TrackRig();
            r.T.SetAnimation(0, "걷기", true, 0);
            var plain = new PoseState(r.M);
            var anim = r.M.FindAnimation("걷기");
            float time = 0;
            foreach (float dt in new[] { 0.13f, 0.4f, 0.47f, 0.33f, 0.016f })
            {
                time += dt;
                var (b1, b2) = r.Step(dt);
                plain.ResetToSetup(r.M);
                AnimationApplier.Apply(r.M, anim, AnimationApplier.AnimationTime(anim.Duration, time, true), null, plain);
                Assert.AreEqual(plain.Locals[1].Rotation, b1);
                Assert.AreEqual(plain.Locals[2].Rotation, b2);
            }
        }

        [Test]
        public void m19_mix_midpoint_is_between_poses()
        {
            var r = new TrackRig();
            r.T.SetAnimation(0, "걷기", false, 0);
            Assert.AreEqual(45f, r.Step(0.5f).b1, 1e-3f);
            r.T.SetAnimation(0, "달리기", false, 0.2f);
            var (b1, b2) = r.Step(0.1f);
            // 걷기 0.6초 = 54°, 달리기 0.1초 = −9°, 섞기 절반 → 22.5°(셋업 0°로 꺼지지 않음)
            Assert.AreEqual(22.5f, b1, 1e-3f);
            Assert.AreEqual(22.5f, b2, 1e-3f);
        }

        [Test]
        public void m19_mix_end_no_jump()
        {
            var r = new TrackRig();
            r.T.SetAnimation(1, "팔", false, 0);
            r.Step(0.1f);
            r.T.SetAnimation(1, "짧은", false, 0.2f);
            float prev = r.Step(0).b1, maxStep = 0;
            for (int i = 0; i < 15; i++)
            {
                float b1 = r.Step(0.02f).b1;
                maxStep = System.Math.Max(maxStep, System.Math.Abs(b1 - prev));
                prev = b1;
            }
            // 팔에만 있던 b1 키는 0.2초 동안 30°→0°로 고르게 빠진다(한 프레임 3°), 끝나도 튀지 않음
            Assert.LessOrEqual(maxStep, 3f + 1e-3f);
            Assert.AreEqual(0f, prev, 1e-4f);
        }

        [Test]
        public void m19_upper_track_only_keyed_values()
        {
            var r = new TrackRig();
            r.T.SetAnimation(0, "걷기", true, 0);
            r.T.SetAnimation(1, "팔", true, 0);
            var (b1, b2) = r.Step(0.25f);
            Assert.AreEqual(30f, b1, 1e-4f);
            Assert.AreEqual(22.5f, b2, 1e-3f);
        }

        [Test]
        public void m19_queue_starts_at_end_plus_delay()
        {
            var r = new TrackRig();
            r.T.SetAnimation(0, "짧은", false, 0);
            r.T.AddAnimation(0, "걷기", true, 0.25f, 0);
            r.Step(0.3f);
            Assert.AreEqual("짧은", r.T.Current(0).Name);
            r.Step(0.1f);
            Assert.AreEqual("걷기", r.T.Current(0).Name);
            Assert.AreEqual(0.05f, r.T.Current(0).Time, 1e-4f);
        }

        [Test]
        public void m19_clear_track_fades_out()
        {
            var r = new TrackRig();
            r.T.SetAnimation(0, "걷기", true, 0);
            r.T.SetAnimation(1, "팔", true, 0);
            Assert.AreEqual(30f, r.Step(0.2f).b1, 1e-4f);
            r.T.ClearTrack(1, 0.2f);
            Assert.AreEqual(28.5f, r.Step(0.1f).b1, 1e-3f); // (팔 30 + 걷기 27) / 2
            Assert.AreEqual(36f, r.Step(0.1f).b1, 1e-3f);
            Assert.IsNull(r.T.Current(1));
        }

        [Test]
        public void m19_completion_once()
        {
            var r = new TrackRig();
            r.T.SetAnimation(0, "짧은", false, 0);
            r.Step(0.05f);
            Assert.AreEqual(0, r.Done.Count);
            r.Step(0.1f);
            Assert.AreEqual(1, r.Done.Count);
            Assert.AreEqual(0, r.Done[0].Track);
            Assert.AreEqual("짧은", r.Done[0].Name);
            r.Step(0.1f);
            Assert.AreEqual(1, r.Done.Count);
        }

        [Test]
        public void m19_mix_longer_than_from_holds_last_frame()
        {
            var r = new TrackRig();
            r.T.SetAnimation(0, "짧은", false, 0);
            Assert.AreEqual(15f, r.Step(0.05f).b2, 1e-3f);
            r.T.SetAnimation(0, "걷기", false, 0.5f);
            float prev = r.Step(0).b2;
            for (int i = 0; i < 10; i++)
            {
                float b2 = r.Step(0.05f).b2;
                Assert.LessOrEqual(System.Math.Abs(b2 - prev), 7f, "프레임 " + i);
                if (i == 4) Assert.AreEqual(21.25f, b2, 1e-3f); // 짧은은 끝(20°)에서 멈춤, 걷기 0.25초 22.5°, 절반
                prev = b2;
            }
            Assert.AreEqual(45f, prev, 1e-3f);
        }

        [Test]
        public void m19_set_during_clear_no_jump()
        {
            var r = new TrackRig();
            r.T.SetAnimation(1, "팔", false, 0);
            r.Step(0.1f);
            r.T.ClearTrack(1, 0.2f);
            Assert.AreEqual(15f, r.Step(0.1f).b1, 1e-3f);
            r.T.SetAnimation(1, "팔", false, 0.2f);
            Assert.AreEqual(15f, r.Step(0).b1, 1e-3f);
            Assert.AreEqual(22.5f, r.Step(0.1f).b1, 1e-3f);
        }

        [Test]
        public void m19_set_during_mix_freezes()
        {
            var r = new TrackRig();
            r.T.SetAnimation(0, "걷기", false, 0);
            r.Step(0.5f);
            r.T.SetAnimation(0, "달리기", false, 0.2f);
            var mid = r.Step(0.1f);
            r.T.SetAnimation(0, "팔", false, 0.2f);
            var now = r.Step(0);
            Assert.AreEqual(mid.b1, now.b1, 1e-3f);
            Assert.AreEqual(mid.b2, now.b2, 1e-3f);
            // 팔에 없는 b2는 얼린 값에서 셋업(0°)으로 빠진다
            Assert.AreEqual(mid.b2 * 0.5f, r.Step(0.1f).b2, 1e-3f);
        }

        [Test]
        public void m19_alpha_half()
        {
            var r = new TrackRig();
            r.T.SetAnimation(0, "걷기", true, 0);
            r.T.SetAnimation(1, "팔", true, 0);
            r.T.SetAlpha(1, 0.5f);
            Assert.AreEqual(24f, r.Step(0.2f).b1, 1e-3f); // 걷기 18° + (30 − 18) × 0.5
        }

        [Test]
        public void m19_events_from_current_only()
        {
            var r = new TrackRig();
            var events = new List<FiredEvent>();
            r.T.SetAnimation(0, "걷기", false, 0);
            r.Step(0.4f);
            r.T.SetAnimation(0, "달리기", false, 0.3f);
            r.Step(0.2f); // 걷기는 0.5초 이벤트를 지나지만 빠지는 중이라 내지 않음, 달리기 0.05초 이벤트
            r.T.CollectEvents(events);
            Assert.AreEqual(1, events.Count);
            Assert.AreEqual(0, events[0].Track);
            Assert.AreEqual("달리기", events[0].Animation);
            r.T.SetAnimation(1, "걷기", false, 0);
            r.Step(0.6f);
            events.Clear();
            r.T.CollectEvents(events);
            Assert.AreEqual(1, events.Count);
            Assert.AreEqual(1, events[0].Track);
            Assert.AreEqual("걷기", events[0].Animation);
        }

        [Test]
        public void m19_poser_tracks_equal_plain_request()
        {
            var text = System.IO.File.ReadAllText(System.IO.Path.Combine(GoldenFile.GoldenDir(), "unity-stage1.kgbone"));
            var m = KGBoneLoader.Load(text);
            foreach (var anim in m.Animations)
            {
                var plain = new SkeletonPoser(m);
                var byTrack = new SkeletonPoser(m);
                var tracks = new AnimationTracks(m);
                tracks.SetAnimation(0, anim.Name, true, 0);
                float time = 0;
                foreach (float dt in new[] { 0.1f, 0.37f, 0.5f, 0.016f, 0.9f, 0.25f })
                {
                    float last = time;
                    time += dt;
                    var a = plain.Compute(new PoseRequest { Animation = anim.Name, Time = time, LastTime = last, Looping = true });
                    var aBones = (WorldBone[])a.Bones.Clone();
                    var aDraw = a.Draw.Select(d => d.Positions.ToArray()).ToList();
                    var aEvents = a.Events.Select(e => e.Name + "@" + e.Time).ToList();
                    tracks.Update(dt, null);
                    var b = byTrack.Compute(new PoseRequest { Tracks = tracks, LastTime = last });
                    string at = anim.Name + "@" + time;
                    for (int i = 0; i < aBones.Length; i++)
                    {
                        Assert.AreEqual(aBones[i].A, b.Bones[i].A, at);
                        Assert.AreEqual(aBones[i].X, b.Bones[i].X, at);
                        Assert.AreEqual(aBones[i].Y, b.Bones[i].Y, at);
                    }
                    Assert.AreEqual(aDraw.Count, b.Draw.Count, at);
                    for (int i = 0; i < aDraw.Count; i++) CollectionAssert.AreEqual(aDraw[i], b.Draw[i].Positions, at);
                    CollectionAssert.AreEqual(aEvents, b.Events.Select(e => e.Name + "@" + e.Time).ToList(), at);
                    foreach (var e in b.Events) Assert.AreEqual(anim.Name, e.Animation);
                }
            }
        }

        [Test]
        public void m19_poser_two_tracks_events_tagged()
        {
            var m = KGBoneLoader.Load(TrackModelJson);
            var poser = new SkeletonPoser(m);
            var tracks = new AnimationTracks(m);
            tracks.SetAnimation(0, "걷기", false, 0);
            tracks.SetAnimation(1, "달리기", false, 0);
            tracks.Update(0.6f, null);
            var r = poser.Compute(new PoseRequest { Tracks = tracks, LastTime = 0 });
            CollectionAssert.AreEqual(new[] { "0:걷기", "1:달리기" }, r.Events.Select(e => e.Track + ":" + e.Animation).ToArray());
            // 이벤트를 내지 않는 계산(LastTime 없음)
            Assert.AreEqual(0, poser.Compute(new PoseRequest { Tracks = tracks }).Events.Count);
        }

        // ---------- M21 스킨 합치기·첨부물 덮어쓰기 ----------
        const string SkinModelJson = @"{""formatVersion"":1,""skeleton"":{""bones"":[{""name"":""root""},{""name"":""x"",""parent"":""root"",""skinRequired"":true}],
            ""slots"":[{""name"":""무기"",""bone"":""root"",""attachment"":""칼""},{""name"":""s"",""bone"":""root"",""attachment"":""a""}],
            ""skins"":[
              {""name"":""default"",""attachments"":{""s"":{""a"":{""path"":""pa""},""b"":{""path"":""pb""}}}},
              {""name"":""A"",""attachments"":{""무기"":{""칼"":{""path"":""칼A""}}}},
              {""name"":""B"",""bones"":[""x""],""attachments"":{""무기"":{""칼"":{""path"":""칼B""}}}}],
            ""animations"":[{""name"":""바꾸기"",""slots"":{""s"":{""attachment"":[{""time"":0,""s"":""b""}]}}}]}}";

        static PoseResult Pose(SkeletonModel m, PoseRequest r) => new SkeletonPoser(m).Compute(r);

        static string ImageOf(PoseResult r, string slot)
        {
            foreach (var d in r.Draw) if (d.Slot == slot) return d.Image;
            return null;
        }

        [Test]
        public void m21_skinset_single_equals_string_path()
        {
            var m = KGBoneLoader.Load(System.IO.File.ReadAllText(System.IO.Path.Combine(GoldenFile.GoldenDir(), "spine43-skins.kgbone")));
            foreach (var s in m.Skins)
            {
                var a = Pose(m, new PoseRequest { Skin = s.Name });
                var aDraw = a.Draw.Select(d => d.Image + ":" + string.Join(",", d.Positions)).ToList();
                var aActive = (bool[])a.Active.Clone();
                var b = Pose(m, new PoseRequest { Skins = new[] { s.Name } });
                CollectionAssert.AreEqual(aDraw, b.Draw.Select(d => d.Image + ":" + string.Join(",", d.Positions)).ToList(), s.Name);
                CollectionAssert.AreEqual(aActive, b.Active, s.Name);
            }
        }

        [Test]
        public void m21_later_skin_wins()
        {
            var m = KGBoneLoader.Load(SkinModelJson);
            Assert.AreEqual("칼B", ImageOf(Pose(m, new PoseRequest { Skins = new[] { "A", "B" } }), "무기"));
            Assert.AreEqual("칼A", ImageOf(Pose(m, new PoseRequest { Skins = new[] { "B", "A" } }), "무기"));
            Assert.AreEqual("pa", ImageOf(Pose(m, new PoseRequest { Skins = new[] { "A", "B" } }), "s"), "기본 스킨도 쓴다");
        }

        [Test]
        public void m21_skin_only_bone_active_if_any_skin()
        {
            var m = KGBoneLoader.Load(SkinModelJson);
            Assert.IsTrue(Pose(m, new PoseRequest { Skins = new[] { "A", "B" } }).Active[1]);
            Assert.IsFalse(Pose(m, new PoseRequest { Skins = new[] { "A" } }).Active[1]);
        }

        [Test]
        public void m21_attachment_override_beats_key_and_clears()
        {
            var m = KGBoneLoader.Load(SkinModelJson);
            var poser = new SkeletonPoser(m);
            var o = new PoseOverrides();
            int s = m.SlotIndex("s");
            var r = new PoseRequest { Animation = "바꾸기", Overrides = o };
            Assert.AreEqual("pb", ImageOf(poser.Compute(r), "s"), "키");
            o.SetAttachment(s, "a");
            Assert.AreEqual("pa", ImageOf(poser.Compute(r), "s"), "덮어쓰기가 키보다 우선");
            o.SetAttachment(s, null);
            Assert.IsNull(ImageOf(poser.Compute(r), "s"), "null이면 숨김");
            o.ClearAttachment(s);
            Assert.AreEqual("pb", ImageOf(poser.Compute(r), "s"), "해제하면 키");
        }

        // ---------- M22 본 덮어쓰기 ----------
        [Test]
        public void m22_local_override_mix_half()
        {
            var m = KGBoneLoader.Load(@"{""formatVersion"":1,""skeleton"":{""bones"":[{""name"":""root""},{""name"":""b"",""parent"":""root""}],
                ""animations"":[{""name"":""돌기"",""bones"":{""b"":{""rotate"":[{""time"":0,""v"":[90]}]}}}]}}");
            var o = new PoseOverrides();
            o.SetBoneLocal(1, new BoneOverride { Rotation = 0, Mix = 0.5f });
            var r = Pose(m, new PoseRequest { Animation = "돌기", Overrides = o });
            Assert.AreEqual(45f, r.Locals[1].Rotation, 1e-4f);
            o.SetBoneLocal(1, new BoneOverride { X = 10 });
            r = Pose(m, new PoseRequest { Animation = "돌기", Overrides = o });
            Assert.AreEqual(90f, r.Locals[1].Rotation, 1e-4f, "정하지 않은 채널은 애니메이션 값");
            Assert.AreEqual(10f, r.Bones[1].X, 1e-4f);
        }

        [Test]
        public void m22_world_position_drives_ik()
        {
            var m = KGBoneLoader.Load(@"{""formatVersion"":1,""skeleton"":{""bones"":[{""name"":""root""},
                {""name"":""윗팔"",""parent"":""root"",""length"":50},{""name"":""아랫팔"",""parent"":""윗팔"",""x"":50,""length"":50},{""name"":""목표"",""parent"":""root"",""x"":100}],
                ""constraints"":[{""type"":""ik"",""name"":""팔"",""bones"":[""윗팔"",""아랫팔""],""target"":""목표""}]}}");
            var o = new PoseOverrides();
            o.SetBoneWorldPosition(3, 60, 40, 1);
            var r = Pose(m, new PoseRequest { Overrides = o });
            Assert.AreEqual(60f, r.Bones[3].X, 1e-3f);
            Assert.AreEqual(40f, r.Bones[3].Y, 1e-3f);
            var (tx, ty) = r.Bones[2].LocalToWorld(50, 0);
            Assert.AreEqual(60f, tx, 1e-2f);
            Assert.AreEqual(40f, ty, 1e-2f);
        }

        [Test]
        public void m22_world_rotation_with_parent_rotated()
        {
            var m = KGBoneLoader.Load(@"{""formatVersion"":1,""skeleton"":{""bones"":[{""name"":""root""},{""name"":""p"",""parent"":""root"",""rotation"":30},
                {""name"":""c"",""parent"":""p"",""x"":20,""rotation"":10},{""name"":""cc"",""parent"":""c"",""x"":10}]}}");
            var o = new PoseOverrides();
            o.SetBoneWorldRotation(2, 90, 1);
            var r = Pose(m, new PoseRequest { Overrides = o });
            Assert.AreEqual(90f, r.Bones[2].RotationX, 1e-3f);
            Assert.AreEqual(60f, r.Locals[2].Rotation, 1e-3f);
            Assert.AreEqual(90f, r.Bones[3].RotationX, 1e-3f, "자식도 다시 계산");
            o.SetBoneWorldRotation(2, 90, 0.5f);
            Assert.AreEqual(65f, Pose(m, new PoseRequest { Overrides = o }).Bones[2].RotationX, 1e-3f, "40°와 90°의 절반");
            o.ClearBone(2);
            Assert.AreEqual(40f, Pose(m, new PoseRequest { Overrides = o }).Bones[2].RotationX, 1e-3f);
        }

        [Test]
        public void m22_constraint_wins_on_constrained_bone()
        {
            var m = KGBoneLoader.Load(@"{""formatVersion"":1,""skeleton"":{""bones"":[{""name"":""root""},{""name"":""a"",""parent"":""root"",""length"":50},{""name"":""t"",""parent"":""root"",""x"":80,""rotation"":40}],
                ""constraints"":[{""type"":""transform"",""name"":""tr"",""bones"":[""a""],""source"":""t""}]}}");
            var plain = Pose(m, new PoseRequest()).Bones[1];
            var o = new PoseOverrides();
            o.SetBoneLocal(1, new BoneOverride { Rotation = -70 });
            var got = Pose(m, new PoseRequest { Overrides = o }).Bones[1];
            Assert.AreEqual(plain.RotationX, got.RotationX, 1e-3f);
        }

        [Test]
        public void m19_completed_entry_stays_put()
        {
            // 검토 I1: 끝난 반복 아닌 항목은 시간을 옮겨도 다시 흐르지 않는다(2단계: 끝나면 멈춤)
            var r = new TrackRig();
            var events = new List<FiredEvent>();
            r.T.SetAnimation(0, "짧은", false, 0);
            r.Step(0.2f);
            Assert.IsTrue(r.T.Current(0).IsComplete);
            r.T.Current(0).Time = 0.05f;
            Assert.AreEqual(15f, r.Step(0.1f).b2, 1e-3f);
            Assert.AreEqual(0.05f, r.T.Current(0).Time, 1e-6f);
            r.T.CollectEvents(events);
            Assert.AreEqual(0, events.Count);
            r.Step(-0.05f);
            Assert.AreEqual(0.05f, r.T.Current(0).Time, 1e-6f);
            Assert.AreEqual(1, r.Done.Count);
        }

#if !UNITY_5_3_OR_NEWER
        [Test]
        public void m18_clipping_no_per_frame_allocation()
        {
            // 검토 I4: 클리핑은 매 프레임 새로 할당하지 않는다(버퍼 재사용)
            var m = KGBoneLoader.Load(System.IO.File.ReadAllText(System.IO.Path.Combine(GoldenFile.GoldenDir(), "unity-stage3-clipping.kgbone")));
            var poser = new SkeletonPoser(m);
            var req = new PoseRequest { Animation = "흔들기", Looping = true };
            for (int i = 0; i < 5; i++) { req.Time = i * 0.21f; poser.Compute(req); }
            long before = System.GC.GetAllocatedBytesForCurrentThread();
            for (int i = 0; i < 20; i++) { req.Time = 0.05f + i * 0.13f; poser.Compute(req); }
            long bytes = System.GC.GetAllocatedBytesForCurrentThread() - before;
            Assert.Less(bytes, 4096, "20프레임 할당 바이트");
        }
#endif

        [Test]
        public void m19_seek_moves_queue_trigger()
        {
            // 작은 지적 1: 시간을 옮기면 예약 시작 시점도 그 시간 기준
            var r = new TrackRig();
            r.T.SetAnimation(0, "짧은", false, 0);
            r.T.AddAnimation(0, "걷기", true, 0.2f, 0);
            r.Step(0.05f);
            r.T.Current(0).Time = 0f;
            r.Step(0.25f);
            Assert.AreEqual("짧은", r.T.Current(0).Name, "끝(0.1) + 0.2 = 0.3초 전");
            r.Step(0.1f);
            Assert.AreEqual("걷기", r.T.Current(0).Name);
            Assert.AreEqual(0.05f, r.T.Current(0).Time, 1e-4f);
        }

        [Test]
        public void m19_old_entry_events_stop_at_trigger()
        {
            // 작은 지적 2: 예약 항목으로 넘어가는 프레임에 옛 항목은 넘어가는 지점까지만 이벤트를 낸다
            var r = new TrackRig();
            var events = new List<FiredEvent>();
            r.T.SetAnimation(0, "걷기", true, 0);
            r.Step(0.3f);
            r.T.AddAnimation(0, "달리기", true, 0, 0); // 이번 바퀴 끝(1초)에 시작
            r.Step(1.3f); // 걷기 0.3→1.0(0.5초 이벤트 하나), 달리기 0→0.6(0.05초 이벤트)
            r.T.CollectEvents(events);
            CollectionAssert.AreEqual(new[] { "걷기", "달리기" }, events.Select(e => e.Animation).ToArray());
            Assert.AreEqual(0.6f, r.T.Current(0).Time, 1e-4f);
        }

        [Test]
        public void m19_queued_delay_not_changed()
        {
            // 작은 지적 3: 빈 트랙에서 기다리는 동안 공개 Delay 값은 그대로
            var r = new TrackRig();
            var e = r.T.AddAnimation(1, "팔", true, 0.5f, 0);
            r.Step(0.2f);
            Assert.AreEqual(0.5f, e.Delay);
            Assert.IsNull(r.T.Current(1));
            r.Step(0.4f);
            Assert.AreEqual("팔", r.T.Current(1).Name);
        }

        [Test]
        public void m19_track_index_limits()
        {
            // 작은 지적 4: 음수·너무 큰 트랙 번호는 거절(트랙을 만들지 않음)
            var r = new TrackRig();
            Assert.IsNull(r.T.SetAnimation(-1, "걷기", true, 0));
            Assert.IsNull(r.T.SetAnimation(AnimationTracks.MaxTracks, "걷기", true, 0));
            Assert.IsNull(r.T.AddAnimation(1000000, "걷기", true, 0, 0));
            r.T.AddEmpty(1000000, 0, 0);
            r.T.SetAlpha(1000000, 0.5f);
            Assert.AreEqual(0, r.T.TrackCount);
            Assert.IsNotNull(r.T.SetAnimation(AnimationTracks.MaxTracks - 1, "걷기", true, 0));
        }

        [Test]
        public void m22_bone_override_copied()
        {
            // 작은 지적 7: 넘긴 BoneOverride를 나중에 고쳐도 다시 부르기 전에는 반영되지 않는다
            var m = KGBoneLoader.Load(@"{""formatVersion"":1,""skeleton"":{""bones"":[{""name"":""root""},{""name"":""b"",""parent"":""root""}]}}");
            var o = new PoseOverrides();
            var v = new BoneOverride { Rotation = 20 };
            o.SetBoneLocal(1, v);
            v.Rotation = 80;
            Assert.AreEqual(20f, Pose(m, new PoseRequest { Overrides = o }).Locals[1].Rotation, 1e-4f);
        }

        [Test]
        public void m19_unknown_name_returns_null()
        {
            var r = new TrackRig();
            r.T.SetAnimation(0, "걷기", true, 0);
            Assert.IsNull(r.T.SetAnimation(0, "없음", true, 0));
            Assert.IsNull(r.T.AddAnimation(0, "없음", true, 0, 0));
            Assert.AreEqual("걷기", r.T.Current(0).Name);
        }
    }
}
