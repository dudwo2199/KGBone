// 본의 월드 변환: 2x2 행렬 [A B; C D]와 위치 (X, Y). 로컬 점 (lx, ly) → (A·lx + B·ly + X, C·lx + D·ly + Y).
// KGBone 핵심 crates/kgbone-core/src/pose/bone_transform.rs의 WorldBone과 같다.
using System;

namespace KGBone.Core
{
    public struct WorldBone
    {
        public float A, B, C, D, X, Y;

        public static readonly WorldBone Identity = new WorldBone { A = 1, B = 0, C = 0, D = 1, X = 0, Y = 0 };

        public (float x, float y) LocalToWorld(float lx, float ly)
        {
            return (A * lx + B * ly + X, C * lx + D * ly + Y);
        }

        public (float x, float y) WorldToLocal(float wx, float wy)
        {
            float det = A * D - B * C;
            float inv = MathF.Abs(det) < 1e-12f ? 0f : 1f / det;
            float dx = wx - X, dy = wy - Y;
            return ((dx * D - dy * B) * inv, (dy * A - dx * C) * inv);
        }

        /// <summary>월드 X축 방향 각도(도).</summary>
        public float RotationX => MathF.Atan2(C, A) * WorldMath.RadToDeg;
        public float ScaleX => MathF.Sqrt(A * A + C * C);
        public float ScaleY => MathF.Sqrt(B * B + D * D);
    }
}
