// 바깥 입구: 모델 하나에 대해 요청마다 포즈를 계산한다. 순서는 핵심 pose/mod.rs의 compute와 같다:
// 셋업 → 애니메이션(반복이면 길이로 나눈 시점) → 활성 판정 → 월드 → 활성 제약을 배열 순서대로 → 그리기 목록 → 이벤트(LastTime이 있을 때).
// 같은 PoseResult 인스턴스를 다시 채워 돌려준다(매 프레임 새로 할당하지 않음).
namespace KGBone.Core
{
    public sealed class SkeletonPoser
    {
        public SkeletonModel Model { get; }
        readonly PoseState state;
        readonly DrawListBuilder draw = new DrawListBuilder();
        readonly PoseResult result = new PoseResult();
        readonly bool[] constraintOn;
        readonly IConstraintSolver[] solvers;
        readonly SolveContext ctx = new SolveContext();
        readonly PhysicsState physics = new PhysicsState();

        public SkeletonPoser(SkeletonModel model)
        {
            Model = model;
            state = new PoseState(model);
            result.Bones = state.World;
            result.Active = state.Active;
            result.Locals = state.Locals;
            constraintOn = new bool[model.Constraints.Count];
            solvers = new IConstraintSolver[model.Constraints.Count];
            for (int i = 0; i < solvers.Length; i++) solvers[i] = SolverFor(model.Constraints[i].Type, physics);
        }

        /// <summary>제약 종류별 계산기(아직 계산하지 않는 종류는 null).</summary>
        static IConstraintSolver SolverFor(ConstraintType t, PhysicsState physics)
        {
            switch (t)
            {
                case ConstraintType.Ik: return new IkSolver();
                case ConstraintType.Transform: return new TransformSolver();
                case ConstraintType.Slider: return new SliderSolver();
                case ConstraintType.Path: return new PathSolver();
                case ConstraintType.Physics: return new PhysicsSolver(physics);
                default: return null;
            }
        }

        public int BoneIndex(string name) => Model.BoneIndex(name);

        /// <summary>물리 상태를 모두 비운다(핵심 PoseEngine::reset_physics).</summary>
        public void ResetPhysics() => physics.ResetAll();

        public PoseResult Compute(PoseRequest r)
        {
            var m = Model;
            state.ResetToSetup(m);
            var anim = m.FindAnimation(r.Animation);
            if (anim != null)
                AnimationApplier.Apply(m, anim, AnimationApplier.AnimationTime(anim.Duration, r.Time, r.Looping), r.Skin, state);
            ActiveBones.Compute(m, r.Skin, state.Active);
            BoneTransform.ComputeAll(m, state.Locals, state.Inherits, state.World);
            // 물리 초기화: 요청이 Reset이거나, 애니메이션의 reset 키가 구간 [LastTime, Time)에 있을 때
            if (r.Physics.Kind == PhysicsStep.StepKind.Reset) physics.ResetAll();
            if (anim != null && r.LastTime.HasValue)
                foreach (var tl in anim.Constraints)
                    if (tl.Name == "reset" && EventFinder.AnyHit(tl.Keys, r.LastTime.Value, r.Time, r.Looping, anim.Duration))
                        physics.ResetNamed(tl.Constraint == ConstraintTimeline.AllPhysics ? "" : m.Constraints[tl.Constraint].Name);
            ConstraintSolvers.ActiveConstraints(m, state.Active, r.Skin, constraintOn);
            ctx.Skin = r.Skin;
            ctx.Physics = r.Physics;
            for (int i = 0; i < solvers.Length; i++)
                if (constraintOn[i] && solvers[i] != null) solvers[i].Apply(m, i, state, ctx);
            draw.Build(m, state, r.Skin, result.Draw);
            result.Events.Clear();
            if (anim != null && r.LastTime.HasValue)
                EventFinder.Fired(m, anim, r.LastTime.Value, r.Time, r.Looping, result.Events);
            return result;
        }
    }
}
