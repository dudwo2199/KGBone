// 그리기 목록 만들기(그리기 순서대로). KGBone 핵심 crates/kgbone-core/src/pose/draw.rs에서 1단계 범위만:
// - region: 슬롯 본 기준 (x, y, rotation, scaleX, scaleY)로 놓인 width × height 사각형. 모서리 순서 왼아래, 오른아래, 오른위, 왼위,
//   uv (0,1)(1,1)(1,0)(0,0), 삼각형 [0,1,2, 2,3,0].
// - mesh: 가중치/비가중치 정점 + deform. linkedmesh: 부모 메시의 uv·삼각형·정점(Timelines면 부모의 deform).
// - 비활성 본에 붙은 슬롯, 찾지 못한 첨부물, 그림이 없는 종류는 그리지 않는다. 시퀀스는 키로 정한 프레임(없으면 셋업 프레임) 이미지.
// - clipping: 그 슬롯부터 end 슬롯(포함)까지 그려지는 것을 자른다(Clipper). 클리핑 슬롯 자체는 그리지 않고, 다음 클리핑이 오면 바뀐다.
//   end 검사는 첨부물이 없거나 비활성인 슬롯에서도 하고, 클리핑 슬롯 자신에서는 하지 않는다(핵심과 같음).
using System;
using System.Collections.Generic;

namespace KGBone.Core
{
    sealed class DrawListBuilder
    {
        const float DegToRad = MathF.PI / 180f;
        static readonly float[] RegionUvs = { 0, 1, 1, 1, 1, 0, 0, 0 };
        static readonly int[] RegionIndices = { 0, 1, 2, 2, 3, 0 };

        readonly List<DrawItem> pool = new List<DrawItem>();
        readonly List<float> clipPolygon = new List<float>();

        /// <summary>첨부물 찾기: 켠 스킨(뒤가 이김) → 기본 스킨(핵심 find_attachment).</summary>
        public static AttachmentData FindAttachment(SkeletonModel m, SkinSet skins, string slot, string name) => SkinSet.Find(skins, m, slot, name);

        public void Build(SkeletonModel m, PoseState pose, SkinSet skin, List<DrawItem> output)
        {
            output.Clear();
            int used = 0;
            Clipper clip = null;
            string clipEnd = null;
            bool clipping = false;
            foreach (int si in pose.DrawOrder)
            {
                if (si < 0 || si >= m.Slots.Count) continue;
                var slot = m.Slots[si];
                if (AddSlot(m, pose, skin, si, output, ref used, ref clip, ref clipEnd, ref clipping)) continue;
                if (clipping && clipEnd == slot.Name)
                {
                    clip = null;
                    clipEnd = null;
                    clipping = false;
                }
            }
        }

        /// <summary>슬롯 하나를 그리기 목록에 더한다. 클리핑 슬롯이면 자르기를 시작하고 true(그 슬롯은 end 검사를 하지 않음).</summary>
        bool AddSlot(SkeletonModel m, PoseState pose, SkinSet skin, int si, List<DrawItem> output, ref int used,
            ref Clipper clip, ref string clipEnd, ref bool clipping)
        {
            {
                var slot = m.Slots[si];
                int bi = slot.Bone;
                bool active = bi >= 0 && pose.Active[bi];
                if (!active) return false;
                string name = pose.SlotAttachments[si];
                if (name == null) return false;
                var att = FindAttachment(m, skin, slot.Name, name);
                if (att == null) return false;
                var bone = pose.World[bi];
                if (att is ClippingAttachmentData ca)
                {
                    pose.Deform.TryGetValue((slot.Name, name), out float[] cdeform);
                    clipPolygon.Clear();
                    MeshVertices.WorldPositions(ca.Weighted, ca.Vertices, bone, pose.World, cdeform, clipPolygon);
                    // 점이 3개 미만이면 자르기를 끈다(핵심 Clipper::new가 None)
                    clip = Clipper.Create(clipPolygon, ca.Convex, ca.Inverse);
                    clipEnd = ca.End;
                    clipping = clip != null;
                    return true;
                }

                DrawItem item = Take(ref used);
                item.Slot = slot.Name;
                item.Attachment = name;
                item.Blend = slot.Blend;
                item.Dark = pose.SlotDarks[si];
                var slotColor = pose.SlotColors[si];

                if (att is RegionAttachment r)
                {
                    item.Image = att.ImagePath(name, Frame(pose, slot.Name, name));
                    item.Color = slotColor * r.Color;
                    RegionCorners(r, bone, item.Positions);
                    item.Uvs.AddRange(RegionUvs);
                    item.Indices.AddRange(RegionIndices);
                }
                else
                {
                    var mesh = MeshVertices.ResolveMesh(m, slot.Name, name, att, out string deformName);
                    if (mesh == null) { used--; return false; }
                    item.Image = att.ImagePath(name, Frame(pose, slot.Name, name));
                    item.Color = slotColor * (att is LinkedMeshAttachment l ? l.Color : mesh.Color);
                    pose.Deform.TryGetValue((slot.Name, deformName), out float[] deform);
                    MeshVertices.WorldPositions(mesh.Weighted, mesh.Vertices, bone, pose.World, deform, item.Positions);
                    item.Uvs.AddRange(mesh.Uvs);
                    item.Indices.AddRange(mesh.Triangles);
                }
                if (clip != null) clip.Clip(item.Positions, item.Uvs, item.Indices);
                if (item.Indices.Count == 0) { used--; return false; }
                output.Add(item);
            }
            return false;
        }

        static int? Frame(PoseState pose, string slot, string name) =>
            pose.SequenceFrames.TryGetValue((slot, name), out int f) ? f : (int?)null;

        DrawItem Take(ref int used)
        {
            if (used == pool.Count) pool.Add(new DrawItem());
            var item = pool[used++];
            item.Positions.Clear();
            item.Uvs.Clear();
            item.Indices.Clear();
            return item;
        }

        /// <summary>region의 네 모서리 월드 좌표.</summary>
        public static void RegionCorners(RegionAttachment r, WorldBone bone, List<float> output)
        {
            float hw = r.Width * 0.5f, hh = r.Height * 0.5f;
            float rad = r.Rotation * DegToRad;
            float s = MathF.Sin(rad), c = MathF.Cos(rad);
            AddCorner(r, bone, -hw, -hh, s, c, output);
            AddCorner(r, bone, hw, -hh, s, c, output);
            AddCorner(r, bone, hw, hh, s, c, output);
            AddCorner(r, bone, -hw, hh, s, c, output);
        }

        static void AddCorner(RegionAttachment r, WorldBone bone, float lx, float ly, float s, float c, List<float> output)
        {
            float sx = lx * r.ScaleX, sy = ly * r.ScaleY;
            float x = r.X + c * sx - s * sy, y = r.Y + s * sx + c * sy;
            var (wx, wy) = bone.LocalToWorld(x, y);
            output.Add(wx);
            output.Add(wy);
        }
    }
}
