// 제약 값 타임라인 → pose.ConstraintValues. KGBone 핵심 crates/kgbone-core/src/anim/constraint_keys.rs와 같다.
// 이름 ""의 타임라인은 모든 물리 제약에 적용된다. 물리 reset 키는 SkeletonPoser가 구간으로 처리한다.
namespace KGBone.Core
{
    public static class ConstraintKeys
    {
        static readonly string[] PhysicsChannels = { "inertia", "strength", "damping", "mass", "wind", "gravity", "mix" };

        public static void Apply(SkeletonModel m, AnimationData a, float t, PoseState pose, Mix mix, int skip)
        {
            foreach (var tl in a.Constraints)
            {
                if (tl.Keys.Length == 0) continue;
                if (tl.Constraint == ConstraintTimeline.AllPhysics)
                {
                    for (int ci = 0; ci < m.Constraints.Count; ci++)
                        if (m.Constraints[ci].Type == ConstraintType.Physics) ApplyOne(m, tl, ci, t, pose, mix, skip);
                }
                else ApplyOne(m, tl, tl.Constraint, t, pose, mix, skip);
            }
        }

        static void ApplyOne(SkeletonModel m, ConstraintTimeline tl, int ci, float t, PoseState pose, Mix mix, int skip)
        {
            if (ci == skip || ci < 0 || ci >= m.Constraints.Count) return;
            var c = m.Constraints[ci];
            var vals = pose.ConstraintValues[ci];
            var setup = pose.ConstraintSetup[ci];
            switch (c.Type)
            {
                case ConstraintType.Ik:
                    if (tl.Name != "ik") return;
                    Continuous(vals, setup, 0, tl.Keys, 0, t, mix);
                    Continuous(vals, setup, 1, tl.Keys, 1, t, mix);
                    for (int ch = 2; ch < 5; ch++) Stepped(vals, ch, tl.Keys, ch, t, mix);
                    return;
                case ConstraintType.Transform:
                    if (tl.Name != "transform") return;
                    for (int ch = 0; ch < 6; ch++) Continuous(vals, setup, ch, tl.Keys, ch, t, mix);
                    return;
                case ConstraintType.Path:
                    switch (tl.Name)
                    {
                        case "position": Continuous(vals, setup, 0, tl.Keys, 0, t, mix); return;
                        case "spacing": Continuous(vals, setup, 1, tl.Keys, 0, t, mix); return;
                        case "mix":
                            for (int ch = 0; ch < 3; ch++) Continuous(vals, setup, 2 + ch, tl.Keys, ch, t, mix);
                            return;
                    }
                    return;
                case ConstraintType.Physics:
                    int p = System.Array.IndexOf(PhysicsChannels, tl.Name);
                    if (p >= 0) Continuous(vals, setup, p, tl.Keys, 0, t, mix);
                    return;
                case ConstraintType.Slider:
                    if (tl.Name == "time") Continuous(vals, setup, 0, tl.Keys, 0, t, mix);
                    else if (tl.Name == "mix") Continuous(vals, setup, 1, tl.Keys, 0, t, mix);
                    return;
            }
        }

        static void Continuous(float[] vals, float[] setup, int dst, Key[] keys, int ch, float t, Mix mix)
        {
            vals[dst] = mix.Blend(vals[dst], Curve.ValueAt(keys, ch, t), setup[dst]);
        }

        static void Stepped(float[] vals, int dst, Key[] keys, int ch, float t, Mix mix)
        {
            if (!mix.Discrete) return;
            int k = Curve.StepIndex(keys, t);
            vals[dst] = k >= 0 && ch < keys[k].V.Length ? keys[k].V[ch] : 0f;
        }
    }
}
