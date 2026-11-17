// 단순 다각형 삼각분할(귀 자르기). KGBone 핵심 crates/kgbone-core/src/mesh/triangulate.rs와 같은 계산.
// 부호 있는 넓이로 방향을 정해 반시계로 본다. 귀 = 볼록 꼭짓점이고 그 삼각형 안에 다른 점이 없는 것.
// 귀가 없으면 넓이 0 꼭짓점(겹친 점·일직선 점)을 삼각형 없이 지우고, 그래도 없으면(꼬인 다각형) 가장 볼록한 꼭짓점을 자른다.
// 매 프레임 쓰는 클리핑을 위해 결과·작업 목록을 받아 다시 쓰는 판(할당 없음)을 둔다.
using System;
using System.Collections.Generic;

namespace KGBone.Core
{
    public static class Triangulator
    {
        // 셈마다 (float)로 반올림한다(Mono의 높은 정밀도 셈에서도 핵심 f32와 같은 판정, Clipper.cs 머리말 참고)
        static float Cross(float ox, float oy, float ax, float ay, float bx, float by)
        {
            float dx1 = (float)(ax - ox), dy1 = (float)(ay - oy), dx2 = (float)(bx - ox), dy2 = (float)(by - oy);
            return (float)((float)(dx1 * dy2) - (float)(dy1 * dx2));
        }

        static float Cross3(IReadOnlyList<float> p, int a, int b, int c) =>
            Cross(p[2 * a], p[2 * a + 1], p[2 * b], p[2 * b + 1], p[2 * c], p[2 * c + 1]);

        static bool Same(IReadOnlyList<float> p, int a, int b) => p[2 * a] == p[2 * b] && p[2 * a + 1] == p[2 * b + 1];

        static float Corner(IReadOnlyList<float> p, List<int> idx, int i, int m) => Cross3(p, idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);

        static bool InTriangle(IReadOnlyList<float> p, int q, int a, int b, int c) =>
            Cross3(p, a, b, q) >= 0f && Cross3(p, b, c, q) >= 0f && Cross3(p, c, a, q) >= 0f;

        /// <summary>다각형 [x0, y0, x1, y1, …](시계·반시계 무관) → 삼각형 정점 번호 3개씩(반시계).</summary>
        public static int[] Triangulate(IReadOnlyList<float> polygon)
        {
            var output = new List<int>();
            Triangulate(polygon, output, new List<int>());
            return output.ToArray();
        }

        /// <summary>output에 삼각형 번호를 채운다(output·idx는 비우고 다시 씀).</summary>
        public static void Triangulate(IReadOnlyList<float> polygon, List<int> output, List<int> idx)
        {
            output.Clear();
            idx.Clear();
            int n = polygon.Count / 2;
            if (n < 3) return;
            float area = 0f;
            for (int i = 0; i < n; i++)
            {
                int j = (i + 1) % n;
                area = (float)(area + Cross(0, 0, polygon[2 * i], polygon[2 * i + 1], polygon[2 * j], polygon[2 * j + 1]));
            }
            for (int i = 0; i < n; i++) idx.Add(i);
            if (area < 0f) idx.Reverse();
            float scale = 0f;
            for (int i = 0; i < n; i++) scale = Math.Max(Math.Max(scale, MathF.Abs(polygon[2 * i])), MathF.Abs(polygon[2 * i + 1]));
            scale = Math.Max(scale, 1f);
            float eps = (float)((float)(1e-7f * scale) * scale);

            while (idx.Count > 3)
            {
                int m = idx.Count;
                int cut = -1;
                // 1) 귀 찾기
                for (int i = 0; i < m && cut < 0; i++)
                {
                    int ia = idx[(i + m - 1) % m], ib = idx[i], ic = idx[(i + 1) % m];
                    if (Cross3(polygon, ia, ib, ic) <= eps) continue;
                    bool blocked = false;
                    for (int k = 0; k < idx.Count; k++)
                    {
                        int j = idx[k];
                        if (j != ia && j != ib && j != ic && !Same(polygon, j, ia) && !Same(polygon, j, ib) && !Same(polygon, j, ic) && InTriangle(polygon, j, ia, ib, ic)) { blocked = true; break; }
                    }
                    if (!blocked) cut = i;
                }
                // 2) 귀가 없으면 넓이 0 꼭짓점을 삼각형 없이 제거
                if (cut < 0)
                {
                    int flat = -1;
                    for (int i = 0; i < m; i++)
                        if (MathF.Abs(Corner(polygon, idx, i, m)) <= eps) { flat = i; break; }
                    if (flat >= 0)
                    {
                        idx.RemoveAt(flat);
                        continue;
                    }
                }
                // 3) 그래도 없으면 가장 볼록한 꼭짓점(같으면 뒤의 것 — Rust max_by와 같음)
                if (cut < 0)
                {
                    cut = 0;
                    float best = Corner(polygon, idx, 0, m);
                    for (int i = 1; i < m; i++)
                    {
                        float f = Corner(polygon, idx, i, m);
                        if (!(f < best)) { best = f; cut = i; }
                    }
                }
                int pa = idx[(cut + m - 1) % m], pb = idx[cut], pc = idx[(cut + 1) % m];
                if (Cross3(polygon, pa, pb, pc) > eps)
                {
                    output.Add(pa);
                    output.Add(pb);
                    output.Add(pc);
                }
                idx.RemoveAt(cut);
            }
            if (idx.Count == 3 && Cross3(polygon, idx[0], idx[1], idx[2]) > eps)
                output.AddRange(idx);
        }
    }
}
