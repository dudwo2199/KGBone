// 활성 판정: 스킨 전용(skinRequired) 본은 켠 스킨의 bones에 있어야 하고, 부모가 비활성이면 자식도 비활성.
// KGBone 핵심 crates/kgbone-core/src/pose/active.rs의 active_bones와 같다.
namespace KGBone.Core
{
    public static class ActiveBones
    {
        public static void Compute(SkeletonModel m, string skin, bool[] outActive)
        {
            var s = m.FindSkin(skin);
            for (int i = 0; i < m.Bones.Count; i++)
            {
                var b = m.Bones[i];
                bool own = !b.SkinRequired || (s != null && s.Bones.Contains(b.Name));
                bool parentOk = b.Parent < 0 || b.Parent >= i || outActive[b.Parent];
                outActive[i] = own && parentOk;
            }
        }
    }
}
