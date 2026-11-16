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

        /// <summary>
        /// 정점 배열 → 월드 좌표 [x0, y0, …].
        /// - 가중치 없음: [x, y, …]가 슬롯 본 기준. 변형 값 deform[2i], deform[2i+1]을 더한 뒤 슬롯 본으로 옮긴다.
        /// - 가중치 있음: 정점마다 [n, (본 번호, x, y, 가중치)×n]. 본별 항목마다 변형 값 2개(항목 순서)를 더하고 월드 = Σ 가중치 × 본월드(x + dx, y + dy).
        /// </summary>
        public static void WorldPositions(bool weighted, float[] vertices, WorldBone slotBone, WorldBone[] world, float[] deform, System.Collections.Generic.List<float> output)
        {
            float D(int k) => deform != null && k < deform.Length ? deform[k] : 0f;
            if (!weighted)
            {
                for (int i = 0; i + 1 < vertices.Length; i += 2)
                {
                    var (x, y) = slotBone.LocalToWorld(vertices[i] + D(i), vertices[i + 1] + D(i + 1));
                    output.Add(x);
                    output.Add(y);
                }
                return;
            }
            int p = 0, entry = 0;
            while (p < vertices.Length)
            {
                int n = (int)System.Math.Max(vertices[p], 0f);
                p++;
                float wx = 0f, wy = 0f;
                for (int j = 0; j < n; j++)
                {
                    if (p + 3 >= vertices.Length) break;
                    int bone = (int)vertices[p];
                    float lx = vertices[p + 1] + D(2 * entry), ly = vertices[p + 2] + D(2 * entry + 1), w = vertices[p + 3];
                    if (bone >= 0 && bone < world.Length)
                    {
                        var (x, y) = world[bone].LocalToWorld(lx, ly);
                        wx += x * w;
                        wy += y * w;
                    }
                    p += 4;
                    entry++;
                }
                output.Add(wx);
                output.Add(wy);
            }
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
