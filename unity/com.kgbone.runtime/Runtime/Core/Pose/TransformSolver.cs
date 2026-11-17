// 트랜스폼 제약(4.3 속성 매핑). KGBone 핵심 crates/kgbone-core/src/pose/transform.rs와 같은 계산.
// d = source.from − from.offset, 값 = to.offset + d × to.scale, clamp면 [to.offset, to.max]로 자름,
// 비가산 대상 += (값 − 대상) × mix(회전은 가까운 쪽), 가산 대상 += 값 × mix.
// 원본 값: localSource면 로컬, 아니면 월드(회전 = X축 각, x·y = 위치, scaleX·Y = 축 길이, shearY = 축 사이 각 − 90).
// 받는 쪽: localTarget면 로컬 값을 바꾸고, 아니면 월드 값을 바꾼 뒤 부모 기준 로컬로 되돌린다.
using System;

namespace KGBone.Core
{
    public sealed class TransformSolver : IConstraintSolver
    {
        readonly float[] src = new float[6], tgt = new float[6];

        static void OfLocal(in LocalTransform t, float[] o)
        {
            o[0] = t.Rotation; o[1] = t.X; o[2] = t.Y; o[3] = t.ScaleX; o[4] = t.ScaleY; o[5] = t.ShearY;
        }

        static void OfWorld(in WorldBone w, float[] o)
        {
            o[0] = w.RotationX; o[1] = w.X; o[2] = w.Y; o[3] = w.ScaleX; o[4] = w.ScaleY; o[5] = WorldMath.WorldShearY(w);
        }

        /// <summary>본 하나의 속성 값(로컬 또는 월드). 슬라이더도 쓴다.</summary>
        public static float BoneProperty(PoseState pose, int bone, BoneProperty p, bool local)
        {
            if (local)
            {
                var t = pose.Locals[bone];
                switch (p)
                {
                    case Core.BoneProperty.Rotate: return t.Rotation;
                    case Core.BoneProperty.X: return t.X;
                    case Core.BoneProperty.Y: return t.Y;
                    case Core.BoneProperty.ScaleX: return t.ScaleX;
                    case Core.BoneProperty.ScaleY: return t.ScaleY;
                    default: return t.ShearY;
                }
            }
            var w = pose.World[bone];
            switch (p)
            {
                case Core.BoneProperty.Rotate: return w.RotationX;
                case Core.BoneProperty.X: return w.X;
                case Core.BoneProperty.Y: return w.Y;
                case Core.BoneProperty.ScaleX: return w.ScaleX;
                case Core.BoneProperty.ScaleY: return w.ScaleY;
                default: return WorldMath.WorldShearY(w);
            }
        }

        public void Apply(SkeletonModel m, int index, PoseState pose, SolveContext ctx)
        {
            var c = (TransformConstraintData)m.Constraints[index];
            int si = c.Source;
            if (si < 0) return;
            var mixes = pose.ConstraintValues[index];
            foreach (int bi in c.Bones)
            {
                if (bi < 0) continue;
                // 원본 값은 대상마다 다시 읽는다(앞 대상이 원본의 조상일 수 있음)
                if (c.LocalSource) OfLocal(pose.Locals[si], src); else OfWorld(pose.World[si], src);
                if (c.LocalTarget) OfLocal(pose.Locals[bi], tgt); else OfWorld(pose.World[bi], tgt);
                foreach (var map in c.Properties)
                {
                    float d = src[(int)map.From] - map.Offset;
                    foreach (var to in map.To)
                    {
                        float v = to.Offset + d * to.Scale;
                        if (c.Clamp)
                        {
                            float lo = to.Offset <= to.Max ? to.Offset : to.Max, hi = to.Offset <= to.Max ? to.Max : to.Offset;
                            v = Math.Min(Math.Max(v, lo), hi);
                        }
                        int k = (int)to.Property;
                        float mix = mixes[k];
                        if (c.Additive) tgt[k] += v * mix;
                        else if (to.Property == Core.BoneProperty.Rotate) tgt[k] += WorldMath.WrapDeg(v - tgt[k]) * mix;
                        else tgt[k] += (v - tgt[k]) * mix;
                    }
                }
                if (c.LocalTarget)
                {
                    ref var l = ref pose.Locals[bi];
                    l.Rotation = tgt[0]; l.X = tgt[1]; l.Y = tgt[2]; l.ScaleX = tgt[3]; l.ScaleY = tgt[4]; l.ShearY = tgt[5];
                }
                else
                {
                    var w = WorldMath.FromParts(tgt[0], tgt[3], tgt[4], tgt[5], tgt[1], tgt[2]);
                    BoneTransform.SetWorld(m, pose, bi, w);
                }
                BoneTransform.UpdateFrom(m, pose, bi);
            }
        }
    }
}
