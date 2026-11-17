// 애니메이션 한 개를 포즈에 적용. KGBone 핵심 crates/kgbone-core/src/anim/의 apply, bone_keys, slot_keys,
// constraint_keys, attachment_keys(deform)와 같은 규칙: 본 값은 셋업 대비(rotate·translate·shear는 더하고 scale은 곱함), 슬롯 색·제약 값은 절대값,
// 이산 값(첨부물·상속·그리기 순서·참거짓 제약 값)은 계단, deform은 셋업 정점에 더할 차이값.
// 섞기(Mix): 비가산 현재 + (목표 − 현재) × alpha(alpha < 1이면 회전은 가까운 쪽), 가산 현재 + (목표 − 셋업) × alpha, 이산 값은 alpha ≥ 0.5일 때만.
using System;

namespace KGBone.Core
{
    /// <summary>섞는 방법 하나(핵심 anim/apply.rs의 Mix).</summary>
    public readonly struct Mix
    {
        public readonly float Alpha;
        public readonly bool Additive;
        public Mix(float alpha, bool additive) { Alpha = alpha; Additive = additive; }

        public float Blend(float current, float target, float setup)
        {
            if (Additive) return current + (target - setup) * Alpha;
            if (Alpha >= 1f) return target;
            return current + (target - current) * Alpha;
        }

        public float BlendAngle(float current, float target, float setup)
        {
            if (Additive || Alpha >= 1f) return Blend(current, target, setup);
            return current + WorldMath.WrapDeg(target - current) * Alpha;
        }

        public bool Discrete => Alpha >= 0.5f;
    }

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

        /// <summary>
        /// time 시점의 값을 pose에 적용한다(핵심 apply_scoped). deform은 기본 스킨과 skin 것만.
        /// alpha는 0~1로 자르고 0 이하면 아무것도 하지 않는다. skipConstraint번 제약의 값 키는 건너뛴다(슬라이더 순환 방지).
        /// </summary>
        public static void Apply(SkeletonModel m, AnimationData a, float time, string skin, PoseState pose,
            float alpha = 1f, bool additive = false, int skipConstraint = -1)
        {
            var mix = new Mix(Math.Min(Math.Max(alpha, 0f), 1f), additive);
            if (mix.Alpha <= 0f) return;
            ApplyBones(m, a, time, pose, mix);
            ApplySlots(m, a, time, pose, mix);
            if (mix.Discrete) ApplyDrawOrder(m, a, time, pose);
            ConstraintKeys.Apply(m, a, time, pose, mix, skipConstraint);
            ApplyDeform(m, a, time, skin, pose, mix);
            if (mix.Discrete) ApplySequences(m, a, time, skin, pose);
        }

        /// <summary>시퀀스 키: 기본 스킨과 skin 것만(deform과 같은 범위). 핵심 attachment_keys.rs.</summary>
        static void ApplySequences(SkeletonModel m, AnimationData a, float t, string skin, PoseState pose)
        {
            foreach (var tl in a.Sequences)
            {
                if (tl.Skin != SkinData.DefaultName && tl.Skin != skin) continue;
                var s = m.FindSkin(tl.Skin);
                if (s == null) continue;
                int count = s.Get(tl.Slot, tl.Attachment)?.Sequence?.Count ?? 1;
                int? frame = SequenceKeys.Frame(tl.Keys, t, count);
                if (frame.HasValue) pose.SequenceFrames[(tl.Slot, tl.Attachment)] = frame.Value;
            }
        }

        static void ApplyBones(SkeletonModel m, AnimationData a, float t, PoseState pose, Mix mix)
        {
            foreach (var tl in a.Bones)
            {
                if (tl.Keys.Length == 0) continue;
                var s = m.Bones[tl.Bone];
                ref var cur = ref pose.Locals[tl.Bone];
                var k = tl.Keys;
                switch (tl.Name)
                {
                    case "rotate": cur.Rotation = mix.BlendAngle(cur.Rotation, s.Rotation + Curve.ValueAt(k, 0, t), s.Rotation); break;
                    case "translate":
                        cur.X = mix.Blend(cur.X, s.X + Curve.ValueAt(k, 0, t), s.X);
                        cur.Y = mix.Blend(cur.Y, s.Y + Curve.ValueAt(k, 1, t), s.Y);
                        break;
                    case "translatex": cur.X = mix.Blend(cur.X, s.X + Curve.ValueAt(k, 0, t), s.X); break;
                    case "translatey": cur.Y = mix.Blend(cur.Y, s.Y + Curve.ValueAt(k, 0, t), s.Y); break;
                    case "scale":
                        cur.ScaleX = mix.Blend(cur.ScaleX, s.ScaleX * Curve.ValueAt(k, 0, t), s.ScaleX);
                        cur.ScaleY = mix.Blend(cur.ScaleY, s.ScaleY * Curve.ValueAt(k, 1, t), s.ScaleY);
                        break;
                    case "scalex": cur.ScaleX = mix.Blend(cur.ScaleX, s.ScaleX * Curve.ValueAt(k, 0, t), s.ScaleX); break;
                    case "scaley": cur.ScaleY = mix.Blend(cur.ScaleY, s.ScaleY * Curve.ValueAt(k, 0, t), s.ScaleY); break;
                    case "shear":
                        cur.ShearX = mix.Blend(cur.ShearX, s.ShearX + Curve.ValueAt(k, 0, t), s.ShearX);
                        cur.ShearY = mix.Blend(cur.ShearY, s.ShearY + Curve.ValueAt(k, 1, t), s.ShearY);
                        break;
                    case "shearx": cur.ShearX = mix.Blend(cur.ShearX, s.ShearX + Curve.ValueAt(k, 0, t), s.ShearX); break;
                    case "sheary": cur.ShearY = mix.Blend(cur.ShearY, s.ShearY + Curve.ValueAt(k, 0, t), s.ShearY); break;
                    case "inherit":
                        if (!mix.Discrete) break;
                        int i = Curve.StepIndex(k, t);
                        var inh = i >= 0 ? ModelNames.ParseInherit(k[i].S) : null;
                        if (inh.HasValue) pose.Inherits[tl.Bone] = inh.Value;
                        break;
                }
            }
        }

        static float Clamp01(float v) => Math.Min(Math.Max(v, 0f), 1f);

        static void ApplySlots(SkeletonModel m, AnimationData a, float t, PoseState pose, Mix mix)
        {
            foreach (var tl in a.Slots)
            {
                if (tl.Keys.Length == 0) continue;
                int s = tl.Slot;
                var k = tl.Keys;
                var setup = m.Slots[s].Color;
                ref var c = ref pose.SlotColors[s];
                switch (tl.Name)
                {
                    case "rgba":
                        SetRgb(ref c, setup, k, t, mix);
                        c.A = Clamp01(mix.Blend(c.A, Curve.ValueAt(k, 3, t), setup.A));
                        break;
                    case "rgb":
                        SetRgb(ref c, setup, k, t, mix);
                        break;
                    case "alpha":
                        c.A = Clamp01(mix.Blend(c.A, Curve.ValueAt(k, 0, t), setup.A));
                        break;
                    case "rgba2":
                        SetRgb(ref c, setup, k, t, mix);
                        c.A = Clamp01(mix.Blend(c.A, Curve.ValueAt(k, 3, t), setup.A));
                        SetDark(m, pose, s, k, 4, t, mix);
                        break;
                    case "rgb2":
                        SetRgb(ref c, setup, k, t, mix);
                        SetDark(m, pose, s, k, 3, t, mix);
                        break;
                    case "attachment":
                        if (!mix.Discrete) break;
                        int i = Curve.StepIndex(k, t);
                        if (i >= 0) pose.SlotAttachments[s] = k[i].S;
                        break;
                }
            }
        }

        static void SetRgb(ref ColorF c, ColorF setup, Key[] k, float t, Mix mix)
        {
            c.R = Clamp01(mix.Blend(c.R, Curve.ValueAt(k, 0, t), setup.R));
            c.G = Clamp01(mix.Blend(c.G, Curve.ValueAt(k, 1, t), setup.G));
            c.B = Clamp01(mix.Blend(c.B, Curve.ValueAt(k, 2, t), setup.B));
        }

        /// <summary>어두운 색 섞기. 어두운 색이 없던 슬롯은 셋업 어두운 색(없으면 검정)에서 시작해 켜진다.</summary>
        static void SetDark(SkeletonModel m, PoseState pose, int s, Key[] k, int ch, float t, Mix mix)
        {
            var setup = m.Slots[s].Dark ?? new ColorF(0, 0, 0, 1);
            var d = pose.SlotDarks[s] ?? new ColorF(setup.R, setup.G, setup.B, 1);
            d.R = Clamp01(mix.Blend(d.R, Curve.ValueAt(k, ch, t), setup.R));
            d.G = Clamp01(mix.Blend(d.G, Curve.ValueAt(k, ch + 1, t), setup.G));
            d.B = Clamp01(mix.Blend(d.B, Curve.ValueAt(k, ch + 2, t), setup.B));
            d.A = 1;
            pose.SlotDarks[s] = d;
        }

        static void ApplyDrawOrder(SkeletonModel m, AnimationData a, float t, PoseState pose)
        {
            if (a.DrawOrder.Count == 0) return;
            int k = 0;
            for (int i = a.DrawOrder.Count - 1; i >= 0; i--)
                if (a.DrawOrder[i].Time <= t) { k = i; break; }
            DrawOrderOf(m, a.DrawOrder[k], pose);
        }

        /// <summary>
        /// 키 하나의 그리기 순서: 이름이 적힌 슬롯은 셋업 자리 + offset으로 옮기고 나머지는 셋업 순서대로 빈 자리를 채운다.
        /// 범위를 벗어나거나 자리가 겹치는 항목은 무시(핵심 anim/slot_keys.rs의 draw_order_of — commands/draw_order.rs의 decode가 아니다).
        /// </summary>
        static void DrawOrderOf(SkeletonModel m, DrawOrderKey key, PoseState pose)
        {
            int n = m.Slots.Count;
            var output = pose.DrawOrder;
            var placed = pose.DrawOrderPlaced;
            var moved = pose.DrawOrderMoved;
            for (int i = 0; i < n; i++) { placed[i] = -1; moved[i] = false; }
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

        static void ApplyDeform(SkeletonModel m, AnimationData a, float t, string skin, PoseState pose, Mix mix)
        {
            foreach (var tl in a.Deform)
            {
                if (tl.Skin != SkinData.DefaultName && tl.Skin != skin) continue;
                var att = m.FindSkin(tl.Skin)?.Get(tl.Slot, tl.Attachment);
                if (att == null) continue;
                // 정점 데이터가 있는 첨부물(패스) 또는 그리는 메시(메시·링크드 메시)
                int len;
                if (att is PathAttachmentData path) len = MeshVertices.DeformLength(path.Weighted, path.Vertices);
                else
                {
                    var mesh = MeshVertices.ResolveMesh(m, tl.Slot, tl.Attachment, att, out _);
                    if (mesh == null) continue;
                    len = MeshVertices.DeformLength(mesh.Weighted, mesh.Vertices);
                }
                if (!Curve.RatioAt(tl.Keys, t, out int i, out float r)) continue;
                var key = (tl.Slot, tl.Attachment);
                // 목표 값을 임시 배열에 만든 뒤 현재 값(이번 계산에 아직 없으면 0 = 셋업)과 섞는다
                var target = pose.DeformScratch(2 * len);
                Expand(tl.Keys[i], target, 0, len);
                if (r > 0f && i + 1 < tl.Keys.Length)
                {
                    Expand(tl.Keys[i + 1], target, len, len);
                    for (int j = 0; j < len; j++) target[j] += (target[len + j] - target[j]) * r;
                }
                if (!pose.Deform.TryGetValue(key, out var cur) || cur.Length != len)
                {
                    cur = pose.DeformBuffer(key, len);
                    Array.Clear(cur, 0, len);
                }
                for (int j = 0; j < len; j++) cur[j] = mix.Blend(cur[j], target[j], 0f);
                pose.Deform[key] = cur;
            }
        }

        /// <summary>키 하나를 output[start..start+len)에 전체 변형 배열로 편다(offset 위치부터 v, 나머지 0).</summary>
        static void Expand(Key k, float[] output, int start, int len)
        {
            Array.Clear(output, start, len);
            for (int j = 0; j < k.V.Length; j++)
            {
                int at = k.Offset + j;
                if (at >= 0 && at < len) output[start + at] = k.V[j];
            }
        }
    }
}
