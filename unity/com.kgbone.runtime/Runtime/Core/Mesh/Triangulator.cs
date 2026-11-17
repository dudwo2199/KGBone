// 단순 다각형 삼각분할(귀 자르기). KGBone 핵심 crates/kgbone-core/src/mesh/triangulate.rs와 같은 계산.
// 부호 있는 넓이로 방향을 정해 반시계로 본다. 귀 = 볼록 꼭짓점이고 그 삼각형 안에 다른 점이 없는 것.
// 귀가 없으면 넓이 0 꼭짓점(겹친 점·일직선 점)을 삼각형 없이 지우고, 그래도 없으면(꼬인 다각형) 가장 볼록한 꼭짓점을 자른다.
using System;
using System.Collections.Generic;

namespace KGBone.Core
{
    public static class Triangulator
    {
        static float Cross(float ox, float oy, float ax, float ay, float bx, float by) =>
            (ax - ox) * (by - oy) - (ay - oy) * (bx - ox);

        /// <summary>다각형 [x0, y0, x1, y1, …](시계·반시계 무관) → 삼각형 정점 번호 3개씩(반시계).</summary>
        public static int[] Triangulate(IReadOnlyList<float> polygon)
        {
            int n = polygon.Count / 2;
            if (n < 3) return new int[0];
            float X(int i) => polygon[2 * i];
            float Y(int i) => polygon[2 * i + 1];
            float area = 0f;
            for (int i = 0; i < n; i++) area += Cross(0, 0, X(i), Y(i), X((i + 1) % n), Y((i + 1) % n));
            var idx = new List<int>(n);
            for (int i = 0; i < n; i++) idx.Add(i);
            if (area < 0f) idx.Reverse();
            float scale = 0f;
            for (int i = 0; i < n; i++) scale = Math.Max(Math.Max(scale, MathF.Abs(X(i))), MathF.Abs(Y(i)));
            scale = Math.Max(scale, 1f);
            float eps = 1e-7f * scale * scale;
            bool Same(int a, int b) => X(a) == X(b) && Y(a) == Y(b);
            float Corner(int i, int m) => Cross(X(idx[(i + m - 1) % m]), Y(idx[(i + m - 1) % m]), X(idx[i]), Y(idx[i]), X(idx[(i + 1) % m]), Y(idx[(i + 1) % m]));
            bool InTriangle(int p, int a, int b, int c) =>
                Cross(X(a), Y(a), X(b), Y(b), X(p), Y(p)) >= 0f && Cross(X(b), Y(b), X(c), Y(c), X(p), Y(p)) >= 0f && Cross(X(c), Y(c), X(a), Y(a), X(p), Y(p)) >= 0f;

            var output = new List<int>((n - 2) * 3);
            while (idx.Count > 3)
            {
                int m = idx.Count;
                int cut = -1;
                // 1) 귀 찾기
                for (int i = 0; i < m && cut < 0; i++)
                {
                    int ia = idx[(i + m - 1) % m], ib = idx[i], ic = idx[(i + 1) % m];
                    if (Cross(X(ia), Y(ia), X(ib), Y(ib), X(ic), Y(ic)) <= eps) continue;
                    bool blocked = false;
                    foreach (int j in idx)
                        if (j != ia && j != ib && j != ic && !Same(j, ia) && !Same(j, ib) && !Same(j, ic) && InTriangle(j, ia, ib, ic)) { blocked = true; break; }
                    if (!blocked) cut = i;
                }
                // 2) 귀가 없으면 넓이 0 꼭짓점을 삼각형 없이 제거
                if (cut < 0)
                {
                    int flat = -1;
                    for (int i = 0; i < m; i++)
                        if (MathF.Abs(Corner(i, m)) <= eps) { flat = i; break; }
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
                    float best = Corner(0, m);
                    for (int i = 1; i < m; i++)
                    {
                        float f = Corner(i, m);
                        if (!(f < best)) { best = f; cut = i; }
                    }
                }
                int pa = idx[(cut + m - 1) % m], pb = idx[cut], pc = idx[(cut + 1) % m];
                if (Cross(X(pa), Y(pa), X(pb), Y(pb), X(pc), Y(pc)) > eps)
                {
                    output.Add(pa);
                    output.Add(pb);
                    output.Add(pc);
                }
                idx.RemoveAt(cut);
            }
            if (idx.Count == 3 && Cross(X(idx[0]), Y(idx[0]), X(idx[1]), Y(idx[1]), X(idx[2]), Y(idx[2])) > eps)
                output.AddRange(idx);
            return output.ToArray();
        }
    }
}
