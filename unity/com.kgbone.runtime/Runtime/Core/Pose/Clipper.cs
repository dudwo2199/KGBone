// 클리핑: 삼각형을 클립 다각형으로 자르기(반전·볼록 옵션). KGBone 핵심 crates/kgbone-core/src/pose/clipping.rs와 같은 계산.
// - 클립 다각형(월드)을 볼록 조각들로 나눈다: convex면 볼록 껍질 하나, 아니면 귀 자르기 삼각형들(서로 겹치지 않음).
// - 보통: 그려질 삼각형 × 각 조각을 반평면으로 차례로 잘라 교집합.
// - 반전(inverse): 삼각형에서 각 조각을 차례로 빼낸다(앞 변들의 안쪽이면서 그 변의 바깥쪽인 부분들 = 겹치지 않는 볼록 조각들).
// - 결과 다각형은 부채꼴로 삼각형화하고, uv는 원래 삼각형의 무게중심 좌표로 다시 구한다. 넓이 0인 삼각형은 버린다.
// 정밀도: 넓이 기준(1e-6) 근처의 가는 조각이 남을지 갈리므로, 셈마다 (float)로 반올림해 핵심(Rust f32)과 같게 한다.
// 유니티의 Mono는 float 셈을 더 높은 정밀도로 할 수 있어, 명시적 (float) 변환(conv.r4)이 없으면 결과가 달라진다.
// 할당: 매 프레임 쓰므로 한 인스턴스를 Reset으로 다시 쓰고, 다각형 목록은 모두 재사용한다(첫 몇 프레임 뒤 할당 없음).
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

        static readonly Comparison<P> ByXY = (a, b) => a.X != b.X ? a.X.CompareTo(b.X) : a.Y.CompareTo(b.Y);

        /// <summary>반시계 볼록 조각들(앞 pieceCount개가 유효).</summary>
        readonly List<List<P>> pieces = new List<List<P>>();
        int pieceCount;
        bool inverse;
        // 다시 쓰는 작업 목록
        readonly List<P> pts = new List<P>(), sorted = new List<P>(), lower = new List<P>(), upper = new List<P>();
        readonly List<int> tri = new List<int>(), triIdx = new List<int>();
        readonly List<List<P>> pool = new List<List<P>>();
        int poolUsed;
        readonly List<List<P>> polys = new List<List<P>>(), remA = new List<List<P>>(), remB = new List<List<P>>();
        readonly List<float> outPos = new List<float>(), outUv = new List<float>();
        readonly List<int> outIdx = new List<int>();

        static float Cross(P o, P a, P b)
        {
            float ax = (float)(a.X - o.X), ay = (float)(a.Y - o.Y), bx = (float)(b.X - o.X), by = (float)(b.Y - o.Y);
            return (float)((float)(ax * by) - (float)(ay * bx));
        }

        static float SignedArea(List<P> poly)
        {
            int n = poly.Count;
            float s = 0f;
            for (int i = 0; i < n; i++)
            {
                P a = poly[i], b = poly[(i + 1) % n];
                s = (float)(s + (float)((float)(a.X * b.Y) - (float)(b.X * a.Y)));
            }
            return (float)(s * 0.5f);
        }

        List<P> Rent()
        {
            if (poolUsed == pool.Count) pool.Add(new List<P>());
            var l = pool[poolUsed++];
            l.Clear();
            return l;
        }

        List<P> NextPiece()
        {
            if (pieceCount == pieces.Count) pieces.Add(new List<P>());
            var l = pieces[pieceCount];
            l.Clear();
            return l;
        }

        /// <summary>볼록 껍질(반시계, 단조 사슬)을 output에.</summary>
        void ConvexHull(List<P> points, List<P> output)
        {
            sorted.Clear();
            sorted.AddRange(points);
            sorted.Sort(ByXY);
            output.Clear();
            var uniq = output;
            foreach (var p in sorted) if (uniq.Count == 0 || !uniq[uniq.Count - 1].Equals(p)) uniq.Add(p); // 연속 중복 제거(dedup)
            if (uniq.Count < 3) return;
            lower.Clear();
            foreach (var p in uniq)
            {
                while (lower.Count >= 2 && Cross(lower[lower.Count - 2], lower[lower.Count - 1], p) <= 0f) lower.RemoveAt(lower.Count - 1);
                lower.Add(p);
            }
            upper.Clear();
            for (int i = uniq.Count - 1; i >= 0; i--)
            {
                var p = uniq[i];
                while (upper.Count >= 2 && Cross(upper[upper.Count - 2], upper[upper.Count - 1], p) <= 0f) upper.RemoveAt(upper.Count - 1);
                upper.Add(p);
            }
            lower.RemoveAt(lower.Count - 1);
            upper.RemoveAt(upper.Count - 1);
            output.Clear();
            output.AddRange(lower);
            output.AddRange(upper);
        }

        /// <summary>반평면 자르기: 변 a→b의 왼쪽(keepLeft) 또는 오른쪽만 output에 남긴다(output은 poly와 달라야 함).</summary>
        static void ClipHalf(List<P> poly, P a, P b, bool keepLeft, List<P> output)
        {
            output.Clear();
            int n = poly.Count;
            for (int i = 0; i < n; i++)
            {
                P cur = poly[i], nxt = poly[(i + 1) % n];
                float sc = Cross(a, b, cur), sn = Cross(a, b, nxt);
                if (!keepLeft) { sc = -sc; sn = -sn; }
                if (sc >= 0f) output.Add(cur);
                if ((sc >= 0f) != (sn >= 0f))
                {
                    float t = (float)(sc / (float)(sc - sn));
                    output.Add(new P((float)(cur.X + (float)((float)(nxt.X - cur.X) * t)), (float)(cur.Y + (float)((float)(nxt.Y - cur.Y) * t))));
                }
            }
        }

        List<P> IntersectConvex(List<P> poly, List<P> c)
        {
            var cur = poly;
            for (int i = 0; i < c.Count; i++)
            {
                if (cur.Count < 3) return Rent();
                var nxt = Rent();
                ClipHalf(cur, c[i], c[(i + 1) % c.Count], true, nxt);
                cur = nxt;
            }
            return cur;
        }

        /// <summary>볼록 다각형 poly에서 볼록 조각 c를 뺀 나머지(서로 겹치지 않는 볼록 조각들)를 output에 더한다.</summary>
        void SubtractConvex(List<P> poly, List<P> c, List<List<P>> output)
        {
            var rem = poly;
            for (int i = 0; i < c.Count; i++)
            {
                if (rem.Count < 3) break;
                P a = c[i], b = c[(i + 1) % c.Count];
                var outside = Rent();
                ClipHalf(rem, a, b, false, outside);
                if (outside.Count >= 3 && MathF.Abs(SignedArea(outside)) > AreaEps) output.Add(outside);
                var inside = Rent();
                ClipHalf(rem, a, b, true, inside);
                rem = inside;
            }
        }

        /// <summary>클립 다각형(월드 [x, y, …])으로 만든다. 점이 3개 미만이면 null.</summary>
        public static Clipper Create(IReadOnlyList<float> polygon, bool convex, bool inverse)
        {
            var c = new Clipper();
            return c.Reset(polygon, convex, inverse) ? c : null;
        }

        /// <summary>이 인스턴스를 새 클립 다각형으로 다시 쓴다. 점이 3개 미만이면 false(자르지 않음).</summary>
        public bool Reset(IReadOnlyList<float> polygon, bool convex, bool inverse)
        {
            this.inverse = inverse;
            pieceCount = 0;
            int n = polygon.Count / 2;
            if (n < 3) return false;
            pts.Clear();
            for (int i = 0; i < n; i++) pts.Add(new P(polygon[2 * i], polygon[2 * i + 1]));
            if (convex)
            {
                ConvexHull(pts, NextPiece());
                pieceCount = 1;
            }
            else
            {
                Triangulator.Triangulate(polygon, tri, triIdx);
                for (int t = 0; t + 2 < tri.Count; t += 3)
                {
                    var p = NextPiece();
                    p.Add(pts[tri[t]]);
                    p.Add(pts[tri[t + 1]]);
                    p.Add(pts[tri[t + 2]]);
                    if (SignedArea(p) < 0f) p.Reverse();
                    if (MathF.Abs(SignedArea(p)) > AreaEps) pieceCount++;
                }
            }
            return true;
        }

        /// <summary>삼각형 묶음을 제자리에서 자른다(위치·uv·번호를 잘린 결과로 바꿈).</summary>
        public void Clip(List<float> positions, List<float> uvs, List<int> indices)
        {
            outPos.Clear();
            outUv.Clear();
            outIdx.Clear();
            for (int t = 0; t + 2 < indices.Count; t += 3)
            {
                int i0 = indices[t], i1 = indices[t + 1], i2 = indices[t + 2];
                if (2 * i0 + 1 >= positions.Count || 2 * i1 + 1 >= positions.Count || 2 * i2 + 1 >= positions.Count) continue;
                P a = new P(positions[2 * i0], positions[2 * i0 + 1]), b = new P(positions[2 * i1], positions[2 * i1 + 1]), c = new P(positions[2 * i2], positions[2 * i2 + 1]);
                float area = Cross(a, b, c);
                if (MathF.Abs(area) <= AreaEps) continue;
                poolUsed = 0;
                var triangle = Rent();
                triangle.Add(a);
                triangle.Add(b);
                triangle.Add(c);
                if (area < 0f) triangle.Reverse();
                polys.Clear();
                if (inverse)
                {
                    var rem = remA;
                    var next = remB;
                    rem.Clear();
                    rem.Add(triangle);
                    for (int k = 0; k < pieceCount; k++)
                    {
                        next.Clear();
                        foreach (var r in rem) SubtractConvex(r, pieces[k], next);
                        var swap = rem;
                        rem = next;
                        next = swap;
                    }
                    polys.AddRange(rem);
                }
                else
                {
                    for (int k = 0; k < pieceCount; k++)
                    {
                        var p = IntersectConvex(triangle, pieces[k]);
                        if (p.Count >= 3) polys.Add(p);
                    }
                }
                float uax = U(uvs, 2 * i0), uay = U(uvs, 2 * i0 + 1), ubx = U(uvs, 2 * i1), uby = U(uvs, 2 * i1 + 1), ucx = U(uvs, 2 * i2), ucy = U(uvs, 2 * i2 + 1);
                foreach (var poly in polys)
                {
                    int bas = outPos.Count / 2;
                    foreach (var p in poly)
                    {
                        // 무게중심 좌표(원래 순서 a, b, c 기준)
                        float wa = (float)(Cross(p, b, c) / area);
                        float wb = (float)(Cross(a, p, c) / area);
                        float wc = (float)((float)(1f - wa) - wb);
                        outPos.Add(p.X);
                        outPos.Add(p.Y);
                        outUv.Add((float)((float)((float)(uax * wa) + (float)(ubx * wb)) + (float)(ucx * wc)));
                        outUv.Add((float)((float)((float)(uay * wa) + (float)(uby * wb)) + (float)(ucy * wc)));
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

        static float U(List<float> uvs, int k) => k < uvs.Count ? uvs[k] : 0f;
    }
}
