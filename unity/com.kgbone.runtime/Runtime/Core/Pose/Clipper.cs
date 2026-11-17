// 클리핑: 삼각형을 클립 다각형으로 자르기(반전·볼록 옵션). KGBone 핵심 crates/kgbone-core/src/pose/clipping.rs와 같은 계산.
// - 클립 다각형(월드)을 볼록 조각들로 나눈다: convex면 볼록 껍질 하나, 아니면 귀 자르기 삼각형들(서로 겹치지 않음).
// - 보통: 그려질 삼각형 × 각 조각을 반평면으로 차례로 잘라 교집합.
// - 반전(inverse): 삼각형에서 각 조각을 차례로 빼낸다(앞 변들의 안쪽이면서 그 변의 바깥쪽인 부분들 = 겹치지 않는 볼록 조각들).
// - 결과 다각형은 부채꼴로 삼각형화하고, uv는 원래 삼각형의 무게중심 좌표로 다시 구한다. 넓이 0인 삼각형은 버린다.
using System;
using System.Collections.Generic;

namespace KGBone.Core
{
    public sealed class Clipper
    {
        const float AreaEps = 1e-6f;

        readonly struct P : IEquatable<P>
        {
            public readonly float X, Y;
            public P(float x, float y) { X = x; Y = y; }
            public bool Equals(P o) => X == o.X && Y == o.Y;
        }

        /// <summary>반시계 볼록 조각들.</summary>
        readonly List<P[]> pieces;
        readonly bool inverse;
        readonly List<float> outPos = new List<float>(), outUv = new List<float>();
        readonly List<int> outIdx = new List<int>();

        Clipper(List<P[]> pieces, bool inverse)
        {
            this.pieces = pieces;
            this.inverse = inverse;
        }

        static float Cross(P o, P a, P b) => (a.X - o.X) * (b.Y - o.Y) - (a.Y - o.Y) * (b.X - o.X);

        static float SignedArea(IReadOnlyList<P> poly)
        {
            int n = poly.Count;
            float s = 0f;
            for (int i = 0; i < n; i++)
            {
                P a = poly[i], b = poly[(i + 1) % n];
                s += a.X * b.Y - b.X * a.Y;
            }
            return s * 0.5f;
        }

        /// <summary>볼록 껍질(반시계, 단조 사슬).</summary>
        static List<P> ConvexHull(List<P> points)
        {
            var sorted = new List<P>(points);
            sorted.Sort((a, b) => a.X != b.X ? a.X.CompareTo(b.X) : a.Y.CompareTo(b.Y));
            var pts = new List<P>(sorted.Count);
            foreach (var p in sorted) if (pts.Count == 0 || !pts[pts.Count - 1].Equals(p)) pts.Add(p); // 연속 중복 제거(dedup)
            if (pts.Count < 3) return pts;
            var lower = new List<P>();
            foreach (var p in pts)
            {
                while (lower.Count >= 2 && Cross(lower[lower.Count - 2], lower[lower.Count - 1], p) <= 0f) lower.RemoveAt(lower.Count - 1);
                lower.Add(p);
            }
            var upper = new List<P>();
            for (int i = pts.Count - 1; i >= 0; i--)
            {
                var p = pts[i];
                while (upper.Count >= 2 && Cross(upper[upper.Count - 2], upper[upper.Count - 1], p) <= 0f) upper.RemoveAt(upper.Count - 1);
                upper.Add(p);
            }
            lower.RemoveAt(lower.Count - 1);
            upper.RemoveAt(upper.Count - 1);
            lower.AddRange(upper);
            return lower;
        }

        /// <summary>반평면 자르기: 변 a→b의 왼쪽(keepLeft) 또는 오른쪽만 남긴다.</summary>
        static List<P> ClipHalf(IReadOnlyList<P> poly, P a, P b, bool keepLeft)
        {
            int n = poly.Count;
            var output = new List<P>(n + 2);
            for (int i = 0; i < n; i++)
            {
                P cur = poly[i], nxt = poly[(i + 1) % n];
                float sc = Cross(a, b, cur), sn = Cross(a, b, nxt);
                if (!keepLeft) { sc = -sc; sn = -sn; }
                if (sc >= 0f) output.Add(cur);
                if ((sc >= 0f) != (sn >= 0f))
                {
                    float t = sc / (sc - sn);
                    output.Add(new P(cur.X + (nxt.X - cur.X) * t, cur.Y + (nxt.Y - cur.Y) * t));
                }
            }
            return output;
        }

        static List<P> IntersectConvex(List<P> poly, P[] c)
        {
            var cur = poly;
            for (int i = 0; i < c.Length; i++)
            {
                if (cur.Count < 3) return new List<P>();
                cur = ClipHalf(cur, c[i], c[(i + 1) % c.Length], true);
            }
            return cur;
        }

        /// <summary>볼록 다각형 poly에서 볼록 조각 c를 뺀 나머지(서로 겹치지 않는 볼록 조각들).</summary>
        static void SubtractConvex(List<P> poly, P[] c, List<List<P>> output)
        {
            var rem = poly;
            for (int i = 0; i < c.Length; i++)
            {
                if (rem.Count < 3) break;
                P a = c[i], b = c[(i + 1) % c.Length];
                var outside = ClipHalf(rem, a, b, false);
                if (outside.Count >= 3 && MathF.Abs(SignedArea(outside)) > AreaEps) output.Add(outside);
                rem = ClipHalf(rem, a, b, true);
            }
        }

        /// <summary>클립 다각형(월드 [x, y, …])으로 만든다. 점이 3개 미만이면 null.</summary>
        public static Clipper Create(IReadOnlyList<float> polygon, bool convex, bool inverse)
        {
            int n = polygon.Count / 2;
            if (n < 3) return null;
            var pts = new List<P>(n);
            for (int i = 0; i < n; i++) pts.Add(new P(polygon[2 * i], polygon[2 * i + 1]));
            var pieces = new List<P[]>();
            if (convex) pieces.Add(ConvexHull(pts).ToArray());
            else
            {
                var tri = Triangulator.Triangulate(polygon);
                for (int t = 0; t + 2 < tri.Length; t += 3)
                {
                    var p = new[] { pts[tri[t]], pts[tri[t + 1]], pts[tri[t + 2]] };
                    if (SignedArea(p) < 0f) Array.Reverse(p);
                    if (MathF.Abs(SignedArea(p)) > AreaEps) pieces.Add(p);
                }
            }
            return new Clipper(pieces, inverse);
        }

        /// <summary>삼각형 묶음을 제자리에서 자른다(위치·uv·번호를 잘린 결과로 바꿈).</summary>
        public void Clip(List<float> positions, List<float> uvs, List<int> indices)
        {
            outPos.Clear();
            outUv.Clear();
            outIdx.Clear();
            P Pt(int i) => new P(positions[2 * i], positions[2 * i + 1]);
            float U(int k) => k < uvs.Count ? uvs[k] : 0f;
            var polys = new List<List<P>>();
            var rem = new List<List<P>>();
            var next = new List<List<P>>();
            for (int t = 0; t + 2 < indices.Count; t += 3)
            {
                int i0 = indices[t], i1 = indices[t + 1], i2 = indices[t + 2];
                if (2 * i0 + 1 >= positions.Count || 2 * i1 + 1 >= positions.Count || 2 * i2 + 1 >= positions.Count) continue;
                P a = Pt(i0), b = Pt(i1), c = Pt(i2);
                float area = Cross(a, b, c);
                if (MathF.Abs(area) <= AreaEps) continue;
                var tri = new List<P> { a, b, c };
                if (area < 0f) tri.Reverse();
                polys.Clear();
                if (inverse)
                {
                    rem.Clear();
                    rem.Add(tri);
                    foreach (var piece in pieces)
                    {
                        next.Clear();
                        foreach (var r in rem) SubtractConvex(r, piece, next);
                        (rem, next) = (next, rem);
                    }
                    polys.AddRange(rem);
                }
                else
                {
                    foreach (var piece in pieces)
                    {
                        var p = IntersectConvex(tri, piece);
                        if (p.Count >= 3) polys.Add(p);
                    }
                }
                float uax = U(2 * i0), uay = U(2 * i0 + 1), ubx = U(2 * i1), uby = U(2 * i1 + 1), ucx = U(2 * i2), ucy = U(2 * i2 + 1);
                foreach (var poly in polys)
                {
                    int bas = outPos.Count / 2;
                    foreach (var p in poly)
                    {
                        // 무게중심 좌표(원래 순서 a, b, c 기준)
                        float wa = Cross(p, b, c) / area;
                        float wb = Cross(a, p, c) / area;
                        float wc = 1f - wa - wb;
                        outPos.Add(p.X);
                        outPos.Add(p.Y);
                        outUv.Add(uax * wa + ubx * wb + ucx * wc);
                        outUv.Add(uay * wa + uby * wb + ucy * wc);
                    }
                    for (int k = 1; k < poly.Count - 1; k++)
                        if (MathF.Abs(Cross(poly[0], poly[k], poly[k + 1])) > AreaEps)
                        {
                            outIdx.Add(bas);
                            outIdx.Add(bas + k);
                            outIdx.Add(bas + k + 1);
                        }
                }
            }
            positions.Clear();
            positions.AddRange(outPos);
            uvs.Clear();
            uvs.AddRange(outUv);
            indices.Clear();
            indices.AddRange(outIdx);
        }
    }
}
