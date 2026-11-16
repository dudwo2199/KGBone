// 그리기 목록 → 유니티 Mesh 하나. 연속된 같은 블렌드 모드끼리 서브메시 하나(서브메시 순서 = 그리기 순서).
// 좌표: KGBone 단위 ÷ pixelsPerUnit, 뒤집기는 좌표 부호 반전. UV: 묶은 텍스처 안으로 옮기고 v를 뒤집는다(KGBone은 v 아래로).
// 정점 색 = 그리기 항목 색, 어두운 색 = UV 채널 1(rgb, 없으면 0). 버퍼는 다시 쓴다.
using System.Collections.Generic;
using KGBone.Core;
using UnityEngine;

namespace KGBone
{
    sealed class MeshBuilder
    {
        readonly List<Vector3> vertices = new List<Vector3>();
        readonly List<Color> colors = new List<Color>();
        readonly List<Vector2> uvs = new List<Vector2>();
        readonly List<Vector4> darks = new List<Vector4>();
        readonly List<List<int>> triangles = new List<List<int>>();

        public void Build(PoseResult pose, KGBoneSkeletonData data, bool flipX, bool flipY, Mesh mesh, List<Material> materialsOut)
        {
            vertices.Clear();
            colors.Clear();
            uvs.Clear();
            darks.Clear();
            materialsOut.Clear();
            int groups = 0;
            Core.BlendMode? current = null;
            float sx = (flipX ? -1f : 1f) / data.PixelsPerUnit, sy = (flipY ? -1f : 1f) / data.PixelsPerUnit;

            foreach (var item in pose.Draw)
            {
                if (!data.TryGetImage(item.Image, out Rect rect, out _)) continue;
                var mat = data.MaterialFor(item.Blend);
                if (mat == null) continue;
                if (current != item.Blend)
                {
                    current = item.Blend;
                    if (triangles.Count <= groups) triangles.Add(new List<int>());
                    triangles[groups].Clear();
                    groups++;
                    materialsOut.Add(mat);
                }
                var tris = triangles[groups - 1];
                int baseIndex = vertices.Count;
                var color = new Color(item.Color.R, item.Color.G, item.Color.B, item.Color.A);
                var dark = item.Dark.HasValue ? new Vector4(item.Dark.Value.R, item.Dark.Value.G, item.Dark.Value.B, 0) : Vector4.zero;
                for (int i = 0; i + 1 < item.Positions.Count; i += 2)
                {
                    vertices.Add(new Vector3(item.Positions[i] * sx, item.Positions[i + 1] * sy, 0));
                    colors.Add(color);
                    darks.Add(dark);
                    float u = i < item.Uvs.Count ? item.Uvs[i] : 0f, v = i + 1 < item.Uvs.Count ? item.Uvs[i + 1] : 0f;
                    uvs.Add(new Vector2(rect.x + u * rect.width, rect.yMax - v * rect.height));
                }
                foreach (int idx in item.Indices) tris.Add(baseIndex + idx);
            }

            mesh.Clear();
            mesh.SetVertices(vertices);
            mesh.SetColors(colors);
            mesh.SetUVs(0, uvs);
            mesh.SetUVs(1, darks);
            mesh.subMeshCount = groups;
            for (int g = 0; g < groups; g++) mesh.SetTriangles(triangles[g], g, false);
            mesh.RecalculateBounds();
        }
    }
}
