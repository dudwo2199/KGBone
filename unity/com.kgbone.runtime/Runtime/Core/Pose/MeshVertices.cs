// 메시 정점 계산 도우미. KGBone 핵심 crates/kgbone-core/src/pose/mesh_deform.rs와 같은 규칙.
namespace KGBone.Core
{
    public static class MeshVertices
    {
        /// <summary>가중치 정점 배열의 본별 항목 수(= 변형 값 길이 / 2).</summary>
        public static int WeightedEntryCount(float[] vertices)
        {
            int i = 0, count = 0;
            while (i < vertices.Length)
            {
                int n = (int)System.Math.Max(vertices[i], 0f);
                count += n;
                i += 1 + 4 * n;
            }
            return count;
        }

        /// <summary>첨부물 정점 데이터의 변형 값 길이.</summary>
        public static int DeformLength(bool weighted, float[] vertices) => weighted ? WeightedEntryCount(vertices) * 2 : vertices.Length;

        /// <summary>
        /// 메시(또는 링크드 메시)의 실제 정점 데이터와 변형 키를 찾을 첨부물 이름. 링크드 메시는 부모 메시(Skin ?? 기본 스킨, 같은 슬롯)를 쓰고,
        /// Timelines면 부모 이름의 변형을 쓴다. 메시가 아니면 null.
        /// </summary>
        public static MeshAttachment ResolveMesh(SkeletonModel m, string slot, string name, AttachmentData att, out string deformName)
        {
            deformName = name;
            if (att is MeshAttachment mesh) return mesh;
            if (att is LinkedMeshAttachment l)
            {
                var skin = m.FindSkin(l.Skin ?? SkinData.DefaultName);
                if (skin?.Get(slot, l.Parent) is MeshAttachment parent)
                {
                    if (l.Timelines) deformName = l.Parent;
                    return parent;
                }
            }
            return null;
        }
    }
}
