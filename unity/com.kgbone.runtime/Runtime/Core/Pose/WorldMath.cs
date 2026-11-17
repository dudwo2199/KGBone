// 제약 계산이 함께 쓰는 작은 수학 도우미. KGBone 핵심 crates/kgbone-core/src/pose/math.rs와 같다.
using System;

namespace KGBone.Core
{
    public static class WorldMath
    {
        public const float DegToRad = MathF.PI / 180f;
        public const float RadToDeg = 180f / MathF.PI;

        /// <summary>각도(도)를 (−180, 180] 범위로.</summary>
        public static float WrapDeg(float a)
        {
            float r = a % 360f;
            if (r > 180f) r -= 360f;
            else if (r <= -180f) r += 360f;
            return r;
        }

        /// <summary>월드 행렬 전체(두 축)를 deg도 돌린다(위치는 그대로).</summary>
        public static void RotateWorld(ref WorldBone w, float deg)
        {
            float s = MathF.Sin(deg * DegToRad), c = MathF.Cos(deg * DegToRad);
            float a = w.A, b = w.B, cc = w.C, d = w.D;
            w.A = c * a - s * cc;
            w.C = s * a + c * cc;
            w.B = c * b - s * d;
            w.D = s * b + c * d;
        }

        /// <summary>월드 X축(첫 열)만 deg도 돌린다(기울기 X).</summary>
        public static void RotateWorldXAxis(ref WorldBone w, float deg)
        {
            float s = MathF.Sin(deg * DegToRad), c = MathF.Cos(deg * DegToRad);
            float a = w.A, cc = w.C;
            w.A = c * a - s * cc;
            w.C = s * a + c * cc;
        }

        /// <summary>월드 기울기 Y: 두 축 사이 각 − 90(도).</summary>
        public static float WorldShearY(in WorldBone w) =>
            WrapDeg(MathF.Atan2(w.D, w.B) * RadToDeg - MathF.Atan2(w.C, w.A) * RadToDeg - 90f);

        /// <summary>월드 값(회전, 크기 X·Y, 기울기 Y)으로 행렬을 만든다.</summary>
        public static WorldBone FromParts(float rotation, float scaleX, float scaleY, float shearY, float x, float y)
        {
            float rx = rotation * DegToRad;
            float ry = (rotation + 90f + shearY) * DegToRad;
            return new WorldBone { A = MathF.Cos(rx) * scaleX, C = MathF.Sin(rx) * scaleX, B = MathF.Cos(ry) * scaleY, D = MathF.Sin(ry) * scaleY, X = x, Y = y };
        }

        /// <summary>부모 공간: 자식 본 하나가 놓이는 좌표계(부모의 상속 행렬 + 부모 원점). 부모가 없으면 단위.</summary>
        public static WorldBone ParentSpace(WorldBone? parent, Inherit inherit)
        {
            if (!parent.HasValue) return WorldBone.Identity;
            var p = parent.Value;
            BoneTransform.InheritedMatrix(p, inherit, out float a, out float b, out float c, out float d);
            return new WorldBone { A = a, B = b, C = c, D = d, X = p.X, Y = p.Y };
        }
    }
}
