// 슬라이더 제약(4.3): 애니메이션을 정해진 시점·혼합으로 적용. KGBone 핵심 crates/kgbone-core/src/pose/slider.rs와 같다.
// 시점: bone이 없으면 제약 값 time(키 반영). bone이 있으면 (본 property 값 − propertyFrom) / (propertyTo − propertyFrom) × 애니메이션 길이
// (local이면 로컬 값, 아니면 월드 값). loop면 길이로 나눈 나머지, 아니면 [0, 길이]로 자른다.
// 그 애니메이션을 alpha = mix, additive로 현재 포즈에 적용하고 모든 본의 월드를 다시 계산한다. 자기 자신 제약 키는 건너뛴다(순환 방지).
using System;

namespace KGBone.Core
{
    public sealed class SliderSolver : IConstraintSolver
    {
        public void Apply(SkeletonModel m, int index, PoseState pose, SolveContext ctx)
        {
            var c = (SliderConstraintData)m.Constraints[index];
            var anim = m.FindAnimation(c.Animation);
            if (anim == null) return;
            float duration = anim.Duration;
            float timeValue = pose.ConstraintValues[index][0], mix = pose.ConstraintValues[index][1];
            float t;
            if (c.Bone >= 0)
            {
                float v = TransformSolver.BoneProperty(pose, c.Bone, c.Property, c.Local);
                float span = c.PropertyTo - c.PropertyFrom;
                t = MathF.Abs(span) < 1e-9f ? 0f : (v - c.PropertyFrom) / span * duration;
            }
            else t = timeValue;
            if (c.Loop) t = AnimationApplier.AnimationTime(duration, t, true);
            else t = Math.Min(Math.Max(t, 0f), Math.Max(duration, 0f));
            AnimationApplier.Apply(m, anim, t, ctx.Skin, pose, mix, c.Additive, index);
            BoneTransform.UpdateFrom(m, pose, 0);
        }
    }
}
