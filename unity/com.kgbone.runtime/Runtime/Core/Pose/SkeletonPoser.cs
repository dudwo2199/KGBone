// 바깥 입구: 모델 하나에 대해 요청마다 포즈를 계산한다. 순서는 핵심 pose/mod.rs의 compute에서 제약을 뺀 것:
// 셋업 → 애니메이션(반복이면 길이로 나눈 시점) → 활성 판정 → 월드 → 그리기 목록 → 이벤트(LastTime이 있을 때).
// 같은 PoseResult 인스턴스를 다시 채워 돌려준다(매 프레임 새로 할당하지 않음).
namespace KGBone.Core
{
    public sealed class SkeletonPoser
    {
        public SkeletonModel Model { get; }
        readonly PoseState state;
        readonly DrawListBuilder draw = new DrawListBuilder();
        readonly PoseResult result = new PoseResult();

        public SkeletonPoser(SkeletonModel model)
        {
            Model = model;
            state = new PoseState(model);
            result.Bones = state.World;
            result.Active = state.Active;
        }

        public int BoneIndex(string name) => Model.BoneIndex(name);

        public PoseResult Compute(PoseRequest r)
        {
            var m = Model;
            state.ResetToSetup(m);
            var anim = m.FindAnimation(r.Animation);
            if (anim != null)
                AnimationApplier.Apply(m, anim, AnimationApplier.AnimationTime(anim.Duration, r.Time, r.Looping), r.Skin, state);
            ActiveBones.Compute(m, r.Skin, state.Active);
            BoneTransform.ComputeAll(m, state.Locals, state.Inherits, state.World);
            draw.Build(m, state, r.Skin, result.Draw);
            result.Events.Clear();
            if (anim != null && r.LastTime.HasValue)
                EventFinder.Fired(m, anim, r.LastTime.Value, r.Time, r.Looping, result.Events);
            return result;
        }
    }
}
