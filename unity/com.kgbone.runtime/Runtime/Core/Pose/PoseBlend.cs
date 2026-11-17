// 두 포즈 섞기와 "애니메이션이 키로 바꾸는 값" 표시(트랙 섞기용, 핵심에는 없음 — 설계 2026-11-18 5.3).
// - 숫자 값은 선형, 각도(회전·shear)는 짧은 쪽, 이산 값(Inherit·첨부물·그리기 순서·시퀀스 프레임·IK 굽힘/줄이기/늘이기)은 t ≥ 0.5면 y.
// - t ≤ 0이면 x 그대로, t ≥ 1이면 y 그대로(끝에서 반올림 차이로 튀지 않게).
using System;
using System.Collections.Generic;

namespace KGBone.Core
{
    /// <summary>애니메이션이 키로 바꾸는 값의 표시(얼린 포즈에서 키 없는 값은 아랫 트랙을 따르게 할 때 쓴다).</summary>
    public sealed class KeyedMask
    {
        public const int BoneX = 1, BoneY = 2, BoneRotation = 4, BoneScaleX = 8, BoneScaleY = 16, BoneShearX = 32, BoneShearY = 64, BoneInherit = 128;
        public const int SlotColor = 1, SlotDark = 2, SlotAttachment = 4;

        public readonly int[] Bones;
        public readonly int[] Slots;
        public readonly bool[] Constraints;
        public bool DrawOrder;
        public readonly HashSet<(string, string)> Deform = new HashSet<(string, string)>();
        public readonly HashSet<(string, string)> Sequences = new HashSet<(string, string)>();

        public KeyedMask(SkeletonModel m)
        {
            Bones = new int[m.Bones.Count];
            Slots = new int[m.Slots.Count];
            Constraints = new bool[m.Constraints.Count];
        }

        public void Clear()
        {
            Array.Clear(Bones, 0, Bones.Length);
            Array.Clear(Slots, 0, Slots.Length);
            Array.Clear(Constraints, 0, Constraints.Length);
            DrawOrder = false;
            Deform.Clear();
            Sequences.Clear();
        }

        public void Union(KeyedMask o)
        {
            for (int i = 0; i < Bones.Length; i++) Bones[i] |= o.Bones[i];
            for (int i = 0; i < Slots.Length; i++) Slots[i] |= o.Slots[i];
            for (int i = 0; i < Constraints.Length; i++) Constraints[i] |= o.Constraints[i];
            DrawOrder |= o.DrawOrder;
            Deform.UnionWith(o.Deform);
            Sequences.UnionWith(o.Sequences);
        }

        public static KeyedMask Of(SkeletonModel m, AnimationData a)
        {
            var k = new KeyedMask(m);
            foreach (var tl in a.Bones)
            {
                if (tl.Bone < 0 || tl.Bone >= k.Bones.Length) continue;
                int f;
                switch (tl.Name)
                {
                    case "rotate": f = BoneRotation; break;
                    case "translate": f = BoneX | BoneY; break;
                    case "translatex": f = BoneX; break;
                    case "translatey": f = BoneY; break;
                    case "scale": f = BoneScaleX | BoneScaleY; break;
                    case "scalex": f = BoneScaleX; break;
                    case "scaley": f = BoneScaleY; break;
                    case "shear": f = BoneShearX | BoneShearY; break;
                    case "shearx": f = BoneShearX; break;
                    case "sheary": f = BoneShearY; break;
                    case "inherit": f = BoneInherit; break;
                    default: f = 0; break;
                }
                k.Bones[tl.Bone] |= f;
            }
            foreach (var tl in a.Slots)
            {
                if (tl.Slot < 0 || tl.Slot >= k.Slots.Length) continue;
                int f;
                switch (tl.Name)
                {
                    case "rgba": case "rgb": case "alpha": f = SlotColor; break;
                    case "rgba2": case "rgb2": f = SlotColor | SlotDark; break;
                    case "attachment": f = SlotAttachment; break;
                    default: f = 0; break;
                }
                k.Slots[tl.Slot] |= f;
            }
            k.DrawOrder = a.DrawOrder.Count > 0;
            foreach (var tl in a.Deform) k.Deform.Add((tl.Slot, tl.Attachment));
            foreach (var tl in a.Sequences) k.Sequences.Add((tl.Slot, tl.Attachment));
            foreach (var tl in a.Constraints)
            {
                if (tl.Constraint == ConstraintTimeline.AllPhysics)
                {
                    for (int i = 0; i < m.Constraints.Count; i++)
                        if (m.Constraints[i].Type == ConstraintType.Physics) k.Constraints[i] = true;
                }
                else if (tl.Constraint >= 0 && tl.Constraint < k.Constraints.Length) k.Constraints[tl.Constraint] = true;
            }
            return k;
        }
    }

    public static class PoseBlend
    {
        static float L(float a, float b, float t) => a + (b - a) * t;
        static float LA(float a, float b, float t) => a + WorldMath.WrapDeg(b - a) * t;

        static ColorF LC(ColorF a, ColorF b, float t) => new ColorF(L(a.R, b.R, t), L(a.G, b.G, t), L(a.B, b.B, t), L(a.A, b.A, t));

        /// <summary>x와 y를 t(0~1)로 섞어 output에 넣는다. 0 &lt; t &lt; 1일 때 output은 x·y와 다른 인스턴스여야 한다.</summary>
        public static void Lerp(SkeletonModel m, PoseState x, PoseState y, float t, PoseState output)
        {
            if (t <= 0f)
            {
                if (!ReferenceEquals(output, x)) output.CopyFrom(x);
                return;
            }
            if (t >= 1f)
            {
                if (!ReferenceEquals(output, y)) output.CopyFrom(y);
                return;
            }
            bool late = t >= 0.5f;
            for (int i = 0; i < output.Locals.Length; i++)
            {
                LocalTransform a = x.Locals[i], b = y.Locals[i];
                output.Locals[i] = new LocalTransform
                {
                    X = L(a.X, b.X, t), Y = L(a.Y, b.Y, t), Rotation = LA(a.Rotation, b.Rotation, t),
                    ScaleX = L(a.ScaleX, b.ScaleX, t), ScaleY = L(a.ScaleY, b.ScaleY, t),
                    ShearX = LA(a.ShearX, b.ShearX, t), ShearY = LA(a.ShearY, b.ShearY, t),
                };
                output.Inherits[i] = late ? y.Inherits[i] : x.Inherits[i];
            }
            for (int i = 0; i < output.SlotColors.Length; i++)
            {
                output.SlotColors[i] = LC(x.SlotColors[i], y.SlotColors[i], t);
                var dx = x.SlotDarks[i];
                var dy = y.SlotDarks[i];
                output.SlotDarks[i] = dx.HasValue && dy.HasValue ? LC(dx.Value, dy.Value, t) : (late ? dy : dx);
                output.SlotAttachments[i] = late ? y.SlotAttachments[i] : x.SlotAttachments[i];
            }
            var order = late ? y.DrawOrder : x.DrawOrder;
            if (!ReferenceEquals(order, output.DrawOrder)) Array.Copy(order, output.DrawOrder, order.Length);
            BlendDeform(x, y, t, late, output);
            var frames = late ? y.SequenceFrames : x.SequenceFrames;
            if (!ReferenceEquals(frames, output.SequenceFrames))
            {
                output.SequenceFrames.Clear();
                foreach (var kv in frames) output.SequenceFrames[kv.Key] = kv.Value;
            }
            for (int c = 0; c < output.ConstraintValues.Length; c++)
            {
                float[] a = x.ConstraintValues[c], b = y.ConstraintValues[c], o = output.ConstraintValues[c];
                bool ik = m.Constraints[c].Type == ConstraintType.Ik;
                for (int j = 0; j < o.Length; j++) o[j] = ik && j >= 2 ? (late ? b[j] : a[j]) : L(a[j], b[j], t);
            }
        }

        // 변형: 같은 키끼리 선형, 한쪽에만 있으면 없는 쪽을 셋업(0)으로 본다. 길이가 다르면 이산(output 버퍼를 다시 씀).
        static void BlendDeform(PoseState x, PoseState y, float t, bool late, PoseState output)
        {
            output.Deform.Clear();
            foreach (var kv in x.Deform) BlendOne(kv.Key, kv.Value, y.Deform.TryGetValue(kv.Key, out var b) ? b : null, t, late, output);
            foreach (var kv in y.Deform) if (!x.Deform.ContainsKey(kv.Key)) BlendOne(kv.Key, null, kv.Value, t, late, output);
        }

        static void BlendOne((string, string) key, float[] a, float[] b, float t, bool late, PoseState output)
        {
            if (a != null && b != null && a.Length != b.Length)
            {
                var pick = late ? b : a;
                var o = output.DeformBuffer(key, pick.Length);
                Array.Copy(pick, o, o.Length);
                output.Deform[key] = o;
                return;
            }
            int len = (a ?? b).Length;
            var r = output.DeformBuffer(key, len);
            for (int j = 0; j < len; j++) r[j] = L(a != null ? a[j] : 0f, b != null ? b[j] : 0f, t);
            output.Deform[key] = r;
        }

        /// <summary>output = r을 바탕으로, mask가 표시한 값만 frozen에서 가져온 포즈.</summary>
        public static void Masked(SkeletonModel m, PoseState r, PoseState frozen, KeyedMask mask, PoseState output)
        {
            if (!ReferenceEquals(output, r)) output.CopyFrom(r);
            for (int i = 0; i < output.Locals.Length; i++)
            {
                int f = mask.Bones[i];
                if (f == 0) continue;
                var o = output.Locals[i];
                var z = frozen.Locals[i];
                if ((f & KeyedMask.BoneX) != 0) o.X = z.X;
                if ((f & KeyedMask.BoneY) != 0) o.Y = z.Y;
                if ((f & KeyedMask.BoneRotation) != 0) o.Rotation = z.Rotation;
                if ((f & KeyedMask.BoneScaleX) != 0) o.ScaleX = z.ScaleX;
                if ((f & KeyedMask.BoneScaleY) != 0) o.ScaleY = z.ScaleY;
                if ((f & KeyedMask.BoneShearX) != 0) o.ShearX = z.ShearX;
                if ((f & KeyedMask.BoneShearY) != 0) o.ShearY = z.ShearY;
                output.Locals[i] = o;
                if ((f & KeyedMask.BoneInherit) != 0) output.Inherits[i] = frozen.Inherits[i];
            }
            for (int i = 0; i < output.SlotColors.Length; i++)
            {
                int f = mask.Slots[i];
                if ((f & KeyedMask.SlotColor) != 0) output.SlotColors[i] = frozen.SlotColors[i];
                if ((f & KeyedMask.SlotDark) != 0) output.SlotDarks[i] = frozen.SlotDarks[i];
                if ((f & KeyedMask.SlotAttachment) != 0) output.SlotAttachments[i] = frozen.SlotAttachments[i];
            }
            if (mask.DrawOrder) Array.Copy(frozen.DrawOrder, output.DrawOrder, output.DrawOrder.Length);
            foreach (var k in mask.Deform)
            {
                if (frozen.Deform.TryGetValue(k, out var z))
                {
                    var buf = output.DeformBuffer(k, z.Length);
                    Array.Copy(z, buf, buf.Length);
                    output.Deform[k] = buf;
                }
                else output.Deform.Remove(k);
            }
            foreach (var k in mask.Sequences)
            {
                if (frozen.SequenceFrames.TryGetValue(k, out int f)) output.SequenceFrames[k] = f;
                else output.SequenceFrames.Remove(k);
            }
            for (int c = 0; c < output.ConstraintValues.Length; c++)
                if (mask.Constraints[c]) Array.Copy(frozen.ConstraintValues[c], output.ConstraintValues[c], output.ConstraintValues[c].Length);
        }
    }
}
