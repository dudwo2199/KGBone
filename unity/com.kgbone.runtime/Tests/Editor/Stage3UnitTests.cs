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
    }
}
