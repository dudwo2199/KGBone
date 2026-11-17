// 패스 제약. KGBone 핵심 crates/kgbone-core/src/pose/path.rs와 같은 계산.
// 패스 첨부물(대상 슬롯의 현재 첨부물)의 월드 정점은 [앞 제어, 점, 뒤 제어]가 정점마다 이어진 3차 베지어 연속 구간이다.
// 구간 i: 점 i → 뒤 제어 i → 앞 제어 i+1 → 점 i+1. closed면 마지막 점 → 첫 점 구간이 더 있다.
// 길이는 구간마다 64등분한 꺾은선 길이 누적 표. constantSpeed면 거리 → (구간, 매개변수)를 표로 찾고, 아니면 거리 비율을 매개변수에 그대로 대응.
// 위치: percent = position × 전체 길이, fixed = position. 간격: length = 본 길이 × 월드 scaleX + spacing, fixed = spacing,
// percent = spacing × 전체 길이, proportional = spacing × 전체 길이 × (본 길이 / 본 길이 합). 열린 패스는 양 끝으로 자르고 닫힌 패스는 감는다.
// 회전: tangent = 접선, chain = 다음 본 자리를 향함, chainScale = chain + 다음 자리까지 닿도록 scaleX. 모두 offsetRotation을 더한다.
using System;
using System.Collections.Generic;

namespace KGBone.Core
{
    public sealed class PathSolver : IConstraintSolver
    {
        const int Samples = 64;

        // 매 프레임 할당하지 않도록 다시 쓰는 버퍼
        readonly List<float> world = new List<float>();
        readonly List<float> segs = new List<float>();   // 구간마다 8개(p0, c1, c2, p1의 x, y)
        readonly List<float> table = new List<float>();  // 구간마다 Samples+1개 누적 길이
        readonly List<float> lens = new List<float>(), dists = new List<float>();
        readonly List<int> bones = new List<int>();
        readonly List<(float x, float y, float tangent)> samples = new List<(float, float, float)>();
        int segCount;
        float total;
        bool closed, constantSpeed;

        void Bez(int s, float t, out float x, out float y)
        {
            int o = s * 8;
            float u = 1f - t;
            float a = u * u * u, b = 3f * u * u * t, c = 3f * u * t * t, d = t * t * t;
            x = a * segs[o] + b * segs[o + 2] + c * segs[o + 4] + d * segs[o + 6];
            y = a * segs[o + 1] + b * segs[o + 3] + c * segs[o + 5] + d * segs[o + 7];
        }

        void BezTangent(int s, float t, out float dx, out float dy)
        {
            int o = s * 8;
            float u = 1f - t;
            float a = 3f * u * u, b = 6f * u * t, c = 3f * t * t;
            dx = a * (segs[o + 2] - segs[o]) + b * (segs[o + 4] - segs[o + 2]) + c * (segs[o + 6] - segs[o + 4]);
            dy = a * (segs[o + 3] - segs[o + 1]) + b * (segs[o + 5] - segs[o + 3]) + c * (segs[o + 7] - segs[o + 5]);
            if (MathF.Abs(dx) < 1e-9f && MathF.Abs(dy) < 1e-9f)
            {
                // 제어점이 끝점과 겹치면 현(弦) 방향
                dx = segs[o + 6] - segs[o];
                dy = segs[o + 7] - segs[o + 1];
            }
        }

        void AddPoint(int p)
        {
            segs.Add(world[2 * p]);
            segs.Add(world[2 * p + 1]);
        }

        void AddSeg(int p0, int c1, int c2, int p1)
        {
            AddPoint(p0);
            AddPoint(c1);
            AddPoint(c2);
            AddPoint(p1);
        }

        /// <summary>월드 정점으로 곡선을 만든다(핵심 PathCurve::new). 구간이 없으면 false.</summary>
        bool BuildCurve(bool isClosed, bool isConstantSpeed)
        {
            int k = world.Count / 2 / 3;
            if (k < 2 && !(isClosed && k >= 1)) return false;
            segs.Clear();
            table.Clear();
            for (int i = 0; i < k - 1; i++) AddSeg(3 * i + 1, 3 * i + 2, 3 * i + 3, 3 * i + 4);
            if (isClosed) AddSeg(3 * (k - 1) + 1, 3 * (k - 1) + 2, 0, 1);
            segCount = segs.Count / 8;
            float acc = 0f;
            for (int s = 0; s < segCount; s++)
            {
                table.Add(acc);
                float px = segs[s * 8], py = segs[s * 8 + 1];
                for (int j = 1; j <= Samples; j++)
                {
                    Bez(s, j / (float)Samples, out float x, out float y);
                    acc += MathF.Sqrt((x - px) * (x - px) + (y - py) * (y - py));
                    table.Add(acc);
                    px = x;
                    py = y;
                }
            }
            total = acc;
            closed = isClosed;
            constantSpeed = isConstantSpeed;
            return true;
        }

        float Row(int s, int j) => table[s * (Samples + 1) + j];

        /// <summary>거리 → (구간, 매개변수).</summary>
        void Locate(float dist, out int seg, out float t)
        {
            float d = dist;
            if (closed && total > 0f)
            {
                d %= total;
                if (d < 0f) d += total;
            }
            else d = Math.Min(Math.Max(d, 0f), total);
            int n = segCount;
            if (!constantSpeed)
            {
                float g = total > 0f ? d / total * n : 0f;
                seg = Math.Min((int)MathF.Floor(g), n - 1);
                t = Math.Min(Math.Max(g - seg, 0f), 1f);
                return;
            }
            seg = n - 1;
            for (int s = 0; s < n; s++)
                if (d <= Row(s, Samples)) { seg = s; break; }
            int jj = Samples;
            for (int j = 0; j <= Samples; j++)
                if (Row(seg, j) >= d) { jj = j; break; }
            jj = Math.Max(jj, 1);
            float l0 = Row(seg, jj - 1), l1 = Row(seg, jj);
            float f = l1 > l0 ? (d - l0) / (l1 - l0) : 0f;
            t = (jj - 1 + f) / Samples;
        }

        (float x, float y, float tangent) Sample(float dist)
        {
            Locate(dist, out int s, out float t);
            BezTangent(s, t, out float tx, out float ty);
            Bez(s, t, out float x, out float y);
            return (x, y, MathF.Atan2(ty, tx) * WorldMath.RadToDeg);
        }

        /// <summary>패스 첨부물의 월드 정점(핵심 path_world). 첨부물이 패스가 아니면 null.</summary>
        PathAttachmentData PathWorld(SkeletonModel m, PoseState pose, SkinSet skin, int slot)
        {
            var s = m.Slots[slot];
            string name = pose.SlotAttachments[slot];
            if (name == null) return null;
            if (!(DrawListBuilder.FindAttachment(m, skin, s.Name, name) is PathAttachmentData p)) return null;
            var bone = s.Bone >= 0 ? pose.World[s.Bone] : WorldBone.Identity;
            pose.Deform.TryGetValue((s.Name, name), out var deform);
            world.Clear();
            MeshVertices.WorldPositions(p.Weighted, p.Vertices, bone, pose.World, deform, world);
            return p;
        }

        public void Apply(SkeletonModel m, int index, PoseState pose, SolveContext ctx)
        {
            var c = (PathConstraintData)m.Constraints[index];
            if (c.Target < 0) return;
            var att = PathWorld(m, pose, ctx.Skins, c.Target);
            if (att == null || !BuildCurve(att.Closed, att.ConstantSpeed)) return;
            var v = pose.ConstraintValues[index];
            float position = v[0], spacing = v[1], mixR = v[2], mixX = v[3], mixY = v[4];
            bones.Clear();
            foreach (int b in c.Bones) if (b >= 0) bones.Add(b);
            if (bones.Count == 0) return;
            lens.Clear();
            float sumLen = 0f;
            foreach (int b in bones)
            {
                float l = m.Bones[b].Length * pose.World[b].ScaleX;
                lens.Add(l);
                sumLen += l;
            }
            float start = c.PositionMode == PositionMode.Percent ? position * total : position;
            // 본 i의 거리, 그리고 마지막 본 다음 자리까지(n+1개)
            dists.Clear();
            dists.Add(start);
            foreach (float l in lens)
            {
                float gap;
                switch (c.SpacingMode)
                {
                    case SpacingMode.Length: gap = l + spacing; break;
                    case SpacingMode.Fixed: gap = spacing; break;
                    case SpacingMode.Percent: gap = spacing * total; break;
                    default: gap = sumLen > 0f ? spacing * total * l / sumLen : 0f; break;
                }
                dists.Add(dists[dists.Count - 1] + gap);
            }
            samples.Clear();
            foreach (float d in dists) samples.Add(Sample(d));
            for (int k = 0; k < bones.Count; k++)
            {
                int b = bones[k];
                var (px, py, tangent) = samples[k];
                var w = pose.World[b];
                var (nx, ny, _) = samples[k + 1];
                float chainAngle = MathF.Atan2(ny - py, nx - px) * WorldMath.RadToDeg;
                float targetRot = (c.RotateMode == RotateMode.Tangent ? tangent : chainAngle) + c.OffsetRotation;
                w.X += (px - w.X) * mixX;
                w.Y += (py - w.Y) * mixY;
                float turn = WorldMath.WrapDeg(targetRot - w.RotationX) * mixR;
                WorldMath.RotateWorld(ref w, turn);
                float curLen = m.Bones[b].Length * w.ScaleX;
                if (c.RotateMode == RotateMode.ChainScale && curLen > 1e-5f)
                {
                    float reach = MathF.Sqrt((nx - px) * (nx - px) + (ny - py) * (ny - py));
                    float f = 1f + (reach / curLen - 1f) * mixR;
                    w.A *= f;
                    w.C *= f;
                }
                BoneTransform.SetWorld(m, pose, b, w);
                BoneTransform.UpdateFrom(m, pose, b);
            }
        }
    }
}
