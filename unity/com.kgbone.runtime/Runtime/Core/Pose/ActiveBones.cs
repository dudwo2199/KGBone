// 활성 판정: 스킨 전용(skinRequired) 본은 켠 스킨의 bones에 있어야 하고, 부모가 비활성이면 자식도 비활성.
// KGBone 핵심 crates/kgbone-core/src/pose/active.rs의 active_bones와 같다.
namespace KGBone.Core
{
    public static class ActiveBones
    {
        /// <summary>skins: 켠 스킨들(하나라도 그 본을 가지면 켜짐, null이면 기본 스킨만).</summary>
        public static void Compute(SkeletonModel m, SkinSet skins, bool[] outActive)
        {
            for (int i = 0; i < m.Bones.Count; i++)
            {
                var b = m.Bones[i];
                bool own = !b.SkinRequired || SkinSet.HasBone(skins, m, b.Name);
                bool parentOk = b.Parent < 0 || b.Parent >= i || outActive[b.Parent];
                outActive[i] = own && parentOk;
            }
        }
    }
}
