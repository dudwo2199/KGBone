// 애니메이션 한 개를 포즈에 적용(섞기 비율 1). KGBone 핵심 crates/kgbone-core/src/anim/의 bone_keys, slot_keys,
// attachment_keys(deform)와 같은 규칙: 본 값은 셋업 대비(rotate·translate·shear는 더하고 scale은 곱함), 슬롯 색은 절대값,
// 이산 값(첨부물·상속·그리기 순서)은 계단, deform은 셋업 정점에 더할 차이값.
using System;

namespace KGBone.Core
{
    public static class AnimationApplier
    {
        /// <summary>애니메이션 시점: 반복이면 길이로 나눈 양수 나머지(길이 0이면 0). 핵심 pose/mod.rs의 animation_time.</summary>
        public static float AnimationTime(float duration, float time, bool looping)
        {
            if (!looping) return time;
            if (duration <= 0f) return 0f;
            float r = time % duration;
            return r < 0f ? r + duration : r;
        }

        /// <summary>time 시점의 값을 pose에 적용한다. deform은 기본 스킨과 skin 것만.</summary>
        public static void Apply(SkeletonModel m, AnimationData a, float time, string skin, PoseState pose)
        {
            ApplyBones(m, a, time, pose);
            ApplySlots(a, time, pose);
            ApplyDrawOrder(m, a, time, pose);
            ApplyDeform(m, a, time, skin, pose);
        }

        static void ApplyBones(SkeletonModel m, AnimationData a, float t, PoseState pose)
        {
            foreach (var tl in a.Bones)
            {
                if (tl.Keys.Length == 0) continue;
                var setup = m.Bones[tl.Bone];
                ref var cur = ref pose.Locals[tl.Bone];
                var k = tl.Keys;
                switch (tl.Name)
                {
                    case "rotate": cur.Rotation = setup.Rotation + Curve.ValueAt(k, 0, t); break;
                    case "translate": cur.X = setup.X + Curve.ValueAt(k, 0, t); cur.Y = setup.Y + Curve.ValueAt(k, 1, t); break;
                    case "translatex": cur.X = setup.X + Curve.ValueAt(k, 0, t); break;
                    case "translatey": cur.Y = setup.Y + Curve.ValueAt(k, 0, t); break;
                    case "scale": cur.ScaleX = setup.ScaleX * Curve.ValueAt(k, 0, t); cur.ScaleY = setup.ScaleY * Curve.ValueAt(k, 1, t); break;
                    case "scalex": cur.ScaleX = setup.ScaleX * Curve.ValueAt(k, 0, t); break;
                    case "scaley": cur.ScaleY = setup.ScaleY * Curve.ValueAt(k, 0, t); break;
                    case "shear": cur.ShearX = setup.ShearX + Curve.ValueAt(k, 0, t); cur.ShearY = setup.ShearY + Curve.ValueAt(k, 1, t); break;
                    case "shearx": cur.ShearX = setup.ShearX + Curve.ValueAt(k, 0, t); break;
                    case "sheary": cur.ShearY = setup.ShearY + Curve.ValueAt(k, 0, t); break;
                    case "inherit":
                        int i = Curve.StepIndex(k, t);
                        var inh = i >= 0 ? ModelNames.ParseInherit(k[i].S) : null;
                        if (inh.HasValue) pose.Inherits[tl.Bone] = inh.Value;
                        break;
                }
            }
        }

        static float Clamp01(float v) => Math.Min(Math.Max(v, 0f), 1f);

        static void ApplySlots(AnimationData a, float t, PoseState pose)
        {
            foreach (var tl in a.Slots)
            {
                if (tl.Keys.Length == 0) continue;
                int s = tl.Slot;
                var k = tl.Keys;
                ref var c = ref pose.SlotColors[s];
                switch (tl.Name)
                {
                    case "rgba":
                        c.R = Clamp01(Curve.ValueAt(k, 0, t)); c.G = Clamp01(Curve.ValueAt(k, 1, t)); c.B = Clamp01(Curve.ValueAt(k, 2, t)); c.A = Clamp01(Curve.ValueAt(k, 3, t));
                        break;
                    case "rgb":
                        c.R = Clamp01(Curve.ValueAt(k, 0, t)); c.G = Clamp01(Curve.ValueAt(k, 1, t)); c.B = Clamp01(Curve.ValueAt(k, 2, t));
                        break;
                    case "alpha":
                        c.A = Clamp01(Curve.ValueAt(k, 0, t));
                        break;
                    case "rgba2":
                        c.R = Clamp01(Curve.ValueAt(k, 0, t)); c.G = Clamp01(Curve.ValueAt(k, 1, t)); c.B = Clamp01(Curve.ValueAt(k, 2, t)); c.A = Clamp01(Curve.ValueAt(k, 3, t));
                        SetDark(pose, s, Curve.ValueAt(k, 4, t), Curve.ValueAt(k, 5, t), Curve.ValueAt(k, 6, t));
                        break;
                    case "rgb2":
                        c.R = Clamp01(Curve.ValueAt(k, 0, t)); c.G = Clamp01(Curve.ValueAt(k, 1, t)); c.B = Clamp01(Curve.ValueAt(k, 2, t));
                        SetDark(pose, s, Curve.ValueAt(k, 3, t), Curve.ValueAt(k, 4, t), Curve.ValueAt(k, 5, t));
                        break;
                    case "attachment":
                        int i = Curve.StepIndex(k, t);
                        if (i >= 0) pose.SlotAttachments[s] = k[i].S;
                        break;
                }
            }
        }

        /// <summary>어두운 색 설정(어두운 색이 없던 슬롯도 켜진다). 섞기 비율 1이라 시작 값과 상관없이 목표 값이 된다.</summary>
        static void SetDark(PoseState pose, int s, float r, float g, float b)
        {
            pose.SlotDarks[s] = new ColorF(Clamp01(r), Clamp01(g), Clamp01(b), 1f);
        }

        static void ApplyDrawOrder(SkeletonModel m, AnimationData a, float t, PoseState pose)
        {
            if (a.DrawOrder.Count == 0) return;
            int k = 0;
            for (int i = a.DrawOrder.Count - 1; i >= 0; i--)
                if (a.DrawOrder[i].Time <= t) { k = i; break; }
            DrawOrderOf(m, a.DrawOrder[k], pose.DrawOrder);
        }

        /// <summary>
        /// 키 하나의 그리기 순서: 이름이 적힌 슬롯은 셋업 자리 + offset으로 옮기고 나머지는 셋업 순서대로 빈 자리를 채운다.
        /// 범위를 벗어나거나 자리가 겹치는 항목은 무시(핵심 anim/slot_keys.rs의 draw_order_of — commands/draw_order.rs의 decode가 아니다).
        /// </summary>
        static void DrawOrderOf(SkeletonModel m, DrawOrderKey key, int[] output)
        {
            int n = m.Slots.Count;
            var placed = new int[n];
            var moved = new bool[n];
            for (int i = 0; i < n; i++) placed[i] = -1;
            foreach (var (slot, offset) in key.Offsets)
            {
                int i = m.SlotIndex(slot);
                if (i < 0) continue;
                long target = (long)i + offset;
                if (target < 0 || target >= n || placed[target] >= 0 || moved[i]) continue;
                placed[target] = i;
                moved[i] = true;
            }
            int rest = 0;
            for (int p = 0; p < n; p++)
            {
                if (placed[p] >= 0) { output[p] = placed[p]; continue; }
                while (rest < n && moved[rest]) rest++;
                output[p] = rest < n ? rest++ : 0;
            }
        }

        static void ApplyDeform(SkeletonModel m, AnimationData a, float t, string skin, PoseState pose)
        {
            foreach (var tl in a.Deform)
            {
                if (tl.Skin != SkinData.DefaultName && tl.Skin != skin) continue;
                var att = m.FindSkin(tl.Skin)?.Get(tl.Slot, tl.Attachment);
                if (att == null) continue;
                // 1단계는 그리는 메시(메시·링크드 메시)의 변형만 쓴다
                var mesh = MeshVertices.ResolveMesh(m, tl.Slot, tl.Attachment, att, out _);
                if (mesh == null) continue;
                int len = MeshVertices.DeformLength(mesh.Weighted, mesh.Vertices);
                if (!Curve.RatioAt(tl.Keys, t, out int i, out float r)) continue;
                var target = Expand(tl.Keys[i], len);
                if (r > 0f && i + 1 < tl.Keys.Length)
                {
                    var b = Expand(tl.Keys[i + 1], len);
                    for (int j = 0; j < len; j++) target[j] += (b[j] - target[j]) * r;
                }
                pose.Deform[(tl.Slot, tl.Attachment)] = target;
            }
        }

        /// <summary>키 하나를 길이 len의 전체 변형 배열로 편다(offset 위치부터 v).</summary>
        static float[] Expand(Key k, int len)
        {
            var output = new float[len];
            for (int j = 0; j < k.V.Length; j++)
            {
                int at = k.Offset + j;
                if (at >= 0 && at < len) output[at] = k.V[j];
            }
            return output;
        }
    }
}
