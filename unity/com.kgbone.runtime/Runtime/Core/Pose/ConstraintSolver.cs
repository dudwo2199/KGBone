// 제약 종류별 계산의 공통 틀과 활성 판정. KGBone 핵심 crates/kgbone-core/src/pose/mod.rs(ConstraintSolver, SolveContext)와 active.rs.
namespace KGBone.Core
{
    /// <summary>물리 진행 방법(핵심 PhysicsStep).</summary>
    public readonly struct PhysicsStep
    {
        public enum StepKind { None, Reset, Advance }
        public readonly StepKind Kind;
        /// <summary>Advance일 때 실제 시간(초).</summary>
        public readonly float Dt;
        PhysicsStep(StepKind k, float dt) { Kind = k; Dt = dt; }
        public static PhysicsStep None => new PhysicsStep(StepKind.None, 0);
        public static PhysicsStep Reset => new PhysicsStep(StepKind.Reset, 0);
        public static PhysicsStep Advance(float dt) => new PhysicsStep(StepKind.Advance, dt);
    }

    /// <summary>제약 계산에 필요한 공통 정보.</summary>
    public sealed class SolveContext
    {
        public string Skin;
        public PhysicsStep Physics;
        public WorldMotion WorldMotion;
    }

    public interface IConstraintSolver
    {
        /// <summary>pose.World가 앞선 계산까지 반영된 상태에서 index번 제약을 적용하고, 바뀐 본의 월드를 갱신한다.</summary>
        void Apply(SkeletonModel m, int index, PoseState pose, SolveContext ctx);
    }

    public static class ConstraintSolvers
    {
        /// <summary>
        /// 제약마다 활성 여부. skinRequired 제약은 켠 스킨의 constraints에 있어야 하고,
        /// 참조하는 본(받는 본·목표·원본·슬라이더 본) 중 하나라도 없거나 비활성이면 계산하지 않는다.
        /// </summary>
        public static void ActiveConstraints(SkeletonModel m, bool[] boneActive, string skin, bool[] output)
        {
            var s = m.FindSkin(skin);
            for (int i = 0; i < m.Constraints.Count; i++)
            {
                var c = m.Constraints[i];
                bool own = !c.SkinRequired || (s != null && s.Constraints.Contains(c.Name));
                output[i] = own && BonesOk(c, boneActive);
            }
        }

        static bool Ok(int b, bool[] active) => b >= 0 && b < active.Length && active[b];

        static bool AllOk(int[] bones, bool[] active)
        {
            foreach (int b in bones) if (!Ok(b, active)) return false;
            return true;
        }

        static bool BonesOk(ConstraintData c, bool[] active)
        {
            switch (c)
            {
                case IkConstraintData k: return AllOk(k.Bones, active) && Ok(k.Target, active);
                case TransformConstraintData t: return AllOk(t.Bones, active) && Ok(t.Source, active);
                case PathConstraintData p: return AllOk(p.Bones, active);
                case PhysicsConstraintData p: return Ok(p.Bone, active);
                case SliderConstraintData s: return s.Bone == -1 || Ok(s.Bone, active);
                default: return false;
            }
        }
    }
}
