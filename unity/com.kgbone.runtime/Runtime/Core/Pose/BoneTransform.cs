// 본 월드 변환(상속 종류별). KGBone 핵심 crates/kgbone-core/src/pose/bone_transform.rs와 같은 계산.
using System;

namespace KGBone.Core
{
    /// <summary>본의 로컬 값(부모 기준). 각도는 도.</summary>
    public struct LocalTransform
    {
        public float X, Y, Rotation, ScaleX, ScaleY, ShearX, ShearY;

        public static LocalTransform Of(BoneData b) => new LocalTransform
        {
            X = b.X, Y = b.Y, Rotation = b.Rotation, ScaleX = b.ScaleX, ScaleY = b.ScaleY, ShearX = b.ShearX, ShearY = b.ShearY,
        };
    }

    public static class BoneTransform
    {
        const float DegToRad = MathF.PI / 180f;

        /// <summary>로컬 값 하나의 행렬 [a, b, c, d](부모 없음).</summary>
        static void LocalMatrix(in LocalTransform t, out float a, out float b, out float c, out float d)
        {
            float rx = (t.Rotation + t.ShearX) * DegToRad;
            float ry = (t.Rotation + 90f + t.ShearY) * DegToRad;
            a = MathF.Cos(rx) * t.ScaleX;
            b = MathF.Cos(ry) * t.ScaleY;
            c = MathF.Sin(rx) * t.ScaleX;
            d = MathF.Sin(ry) * t.ScaleY;
        }

        /// <summary>상속 종류에 따라 자식이 물려받을 부모 행렬. θ = atan2(c, a), 반사 = det &lt; 0.</summary>
        internal static void InheritedMatrix(in WorldBone p, Inherit inherit, out float a, out float b, out float c, out float d)
        {
            float theta = MathF.Atan2(p.C, p.A);
            float cs = MathF.Cos(theta), sn = MathF.Sin(theta);
            bool reflect = p.A * p.D - p.B * p.C < 0f;
            switch (inherit)
            {
                case Inherit.OnlyTranslation:
                    a = 1; b = 0; c = 0; d = 1;
                    return;
                case Inherit.NoRotationOrReflection:
                    // R(−θ)·P, 반사면 둘째 열 부호를 뒤집는다
                    a = cs * p.A + sn * p.C;
                    b = cs * p.B + sn * p.D;
                    c = -sn * p.A + cs * p.C;
                    d = -sn * p.B + cs * p.D;
                    if (reflect) { b = -b; d = -d; }
                    return;
                case Inherit.NoScale:
                    if (reflect) { a = cs; b = sn; c = sn; d = -cs; }
                    else { a = cs; b = -sn; c = sn; d = cs; }
                    return;
                case Inherit.NoScaleOrReflection:
                    a = cs; b = -sn; c = sn; d = cs;
                    return;
                default:
                    a = p.A; b = p.B; c = p.C; d = p.D;
                    return;
            }
        }

        public static WorldBone WorldOf(in LocalTransform l, Inherit inherit, WorldBone? parent)
        {
            LocalMatrix(l, out float la, out float lb, out float lc, out float ld);
            if (!parent.HasValue) return new WorldBone { A = la, B = lb, C = lc, D = ld, X = l.X, Y = l.Y };
            var p = parent.Value;
            InheritedMatrix(p, inherit, out float pa, out float pb, out float pc, out float pd);
            var (x, y) = p.LocalToWorld(l.X, l.Y);
            return new WorldBone
            {
                A = pa * la + pb * lc,
                B = pa * lb + pb * ld,
                C = pc * la + pd * lc,
                D = pc * lb + pd * ld,
                X = x,
                Y = y,
            };
        }

        /// <summary>모든 본의 월드 변환. 부모가 앞에 있다(BoneData.Parent &lt; 자기 번호).</summary>
        public static void ComputeAll(SkeletonModel m, LocalTransform[] locals, Inherit[] inherits, WorldBone[] outWorld)
        {
            for (int i = 0; i < m.Bones.Count; i++)
            {
                int pi = m.Bones[i].Parent;
                WorldBone? parent = pi >= 0 && pi < i ? outWorld[pi] : (WorldBone?)null;
                outWorld[i] = WorldOf(locals[i], inherits[i], parent);
            }
        }

        /// <summary>본 i의 부모 월드(부모가 자기보다 앞에 있을 때만, 핵심 parent_indices).</summary>
        public static WorldBone? ParentWorld(SkeletonModel m, WorldBone[] world, int i)
        {
            int pi = m.Bones[i].Parent;
            return pi >= 0 && pi < i ? world[pi] : (WorldBone?)null;
        }

        /// <summary>from번 본부터 끝까지 월드를 다시 계산한다(핵심 update_world).</summary>
        public static void UpdateFrom(SkeletonModel m, PoseState pose, int from)
        {
            for (int i = from; i < m.Bones.Count; i++)
                pose.World[i] = WorldOf(pose.Locals[i], pose.Inherits[i], ParentWorld(m, pose.World, i));
        }

        /// <summary>본 i의 월드를 w로 만드는 로컬 값을 넣는다(핵심 set_world). 그 뒤 UpdateFrom을 불러야 한다.</summary>
        public static void SetWorld(SkeletonModel m, PoseState pose, int i, in WorldBone w)
        {
            pose.Locals[i] = WorldToLocal(w, ParentWorld(m, pose.World, i), pose.Inherits[i]);
        }

        /// <summary>월드 변환 w를 부모 아래 로컬 값으로 되돌린다(기울기는 shearY로, 핵심 world_to_local_transform).</summary>
        public static LocalTransform WorldToLocal(in WorldBone w, WorldBone? parent, Inherit inherit)
        {
            float pa = 1, pb = 0, pc = 0, pd = 1, x = w.X, y = w.Y;
            if (parent.HasValue)
            {
                InheritedMatrix(parent.Value, inherit, out pa, out pb, out pc, out pd);
                (x, y) = parent.Value.WorldToLocal(w.X, w.Y);
            }
            float det = pa * pd - pb * pc;
            float inv = MathF.Abs(det) < 1e-12f ? 0f : 1f / det;
            float ia = pd * inv, ib = -pb * inv, ic = -pc * inv, id = pa * inv;
            float l0 = ia * w.A + ib * w.C, l1 = ia * w.B + ib * w.D, l2 = ic * w.A + id * w.C, l3 = ic * w.B + id * w.D;
            float sx = MathF.Sqrt(l0 * l0 + l2 * l2);
            float rotation = MathF.Atan2(l2, l0) * WorldMath.RadToDeg;
            float sy = MathF.Sqrt(l1 * l1 + l3 * l3);
            float ry = MathF.Atan2(l3, l1) * WorldMath.RadToDeg;
            float shearY = ry - 90f - rotation;
            while (shearY > 180f) shearY -= 360f;
            while (shearY < -180f) shearY += 360f;
            return new LocalTransform { X = x, Y = y, Rotation = rotation, ScaleX = sx, ScaleY = sy, ShearX = 0, ShearY = MathF.Abs(shearY) < 1e-4f ? 0f : shearY };
        }
    }
}
