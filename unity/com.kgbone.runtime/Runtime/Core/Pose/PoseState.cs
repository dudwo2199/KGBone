// 계산 중 상태(핵심 pose/mod.rs의 SkeletonPose에서 제약·시퀀스를 뺀 것). 배열은 한 번 만들고 다시 쓴다.
using System;
using System.Collections.Generic;

namespace KGBone.Core
{
    public sealed class PoseState
    {
        public readonly LocalTransform[] Locals;
        public readonly Inherit[] Inherits;
        public readonly WorldBone[] World;
        public readonly bool[] Active;
        public readonly ColorF[] SlotColors;
        public readonly ColorF?[] SlotDarks;
        public readonly string[] SlotAttachments;
        public readonly int[] DrawOrder;
        /// <summary>(슬롯, 첨부물) → 셋업 정점에 더할 변형 값(이번 계산에서 키가 있는 것만).</summary>
        public readonly Dictionary<(string slot, string attachment), float[]> Deform = new Dictionary<(string, string), float[]>();
        /// <summary>(슬롯, 첨부물) → 시퀀스 프레임 번호(이번 계산에서 키가 있는 것만, 없으면 셋업 프레임).</summary>
        public readonly Dictionary<(string slot, string attachment), int> SequenceFrames = new Dictionary<(string, string), int>();
        /// <summary>
        /// 제약마다 현재 값(제약 순서, 핵심 constraint_values와 같은 모양):
        /// ik [mix, softness, bendPositive(1/−1), compress(0/1), stretch(0/1)], transform [mixRotate, mixX, mixY, mixScaleX, mixScaleY, mixShearY],
        /// path [position, spacing, mixRotate, mixX, mixY], physics [inertia, strength, damping, mass, wind, gravity, mix], slider [time, mix].
        /// </summary>
        public readonly float[][] ConstraintValues;
        /// <summary>제약마다 셋업 값(바꾸지 않는다).</summary>
        public readonly float[][] ConstraintSetup;

        // 매 프레임 할당하지 않도록 다시 쓰는 임시 버퍼
        readonly Dictionary<(string, string), float[]> deformPool = new Dictionary<(string, string), float[]>();
        float[] deformScratch = new float[0];
        internal readonly int[] DrawOrderPlaced;
        internal readonly bool[] DrawOrderMoved;

        /// <summary>(슬롯, 첨부물)의 변형 배열(길이 len). 계산마다 같은 배열을 다시 쓴다.</summary>
        internal float[] DeformBuffer((string, string) key, int len)
        {
            if (!deformPool.TryGetValue(key, out var a) || a.Length != len)
            {
                a = new float[len];
                deformPool[key] = a;
            }
            return a;
        }

        /// <summary>길이 len 이상인 임시 배열.</summary>
        internal float[] DeformScratch(int len)
        {
            if (deformScratch.Length < len) deformScratch = new float[len];
            return deformScratch;
        }

        public PoseState(SkeletonModel m)
        {
            Locals = new LocalTransform[m.Bones.Count];
            Inherits = new Inherit[m.Bones.Count];
            World = new WorldBone[m.Bones.Count];
            Active = new bool[m.Bones.Count];
            SlotColors = new ColorF[m.Slots.Count];
            SlotDarks = new ColorF?[m.Slots.Count];
            SlotAttachments = new string[m.Slots.Count];
            DrawOrder = new int[m.Slots.Count];
            DrawOrderPlaced = new int[m.Slots.Count];
            DrawOrderMoved = new bool[m.Slots.Count];
            ConstraintValues = new float[m.Constraints.Count][];
            ConstraintSetup = new float[m.Constraints.Count][];
            for (int i = 0; i < ConstraintValues.Length; i++)
            {
                ConstraintValues[i] = SetupValues(m.Constraints[i]);
                ConstraintSetup[i] = SetupValues(m.Constraints[i]);
            }
            ResetToSetup(m);
        }

        /// <summary>셋업 포즈로: 셋업 로컬 값·상속, 월드, 슬롯 색·어두운 색·첨부물, 셋업 그리기 순서, 변형 없음.</summary>
        public void ResetToSetup(SkeletonModel m)
        {
            for (int i = 0; i < m.Bones.Count; i++)
            {
                Locals[i] = LocalTransform.Of(m.Bones[i]);
                Inherits[i] = m.Bones[i].Inherit;
                Active[i] = true;
            }
            BoneTransform.ComputeAll(m, Locals, Inherits, World);
            for (int i = 0; i < m.Slots.Count; i++)
            {
                SlotColors[i] = m.Slots[i].Color;
                SlotDarks[i] = m.Slots[i].Dark;
                SlotAttachments[i] = m.Slots[i].Attachment;
                DrawOrder[i] = i;
            }
            Deform.Clear();
            SequenceFrames.Clear();
            for (int i = 0; i < ConstraintValues.Length; i++) System.Array.Copy(ConstraintSetup[i], ConstraintValues[i], ConstraintSetup[i].Length);
        }

        /// <summary>other의 값을 모두 복사한다(같은 모델의 포즈). 변형 배열은 이 포즈의 버퍼를 다시 쓴다.</summary>
        public void CopyFrom(PoseState other)
        {
            Array.Copy(other.Locals, Locals, Locals.Length);
            Array.Copy(other.Inherits, Inherits, Inherits.Length);
            Array.Copy(other.World, World, World.Length);
            Array.Copy(other.Active, Active, Active.Length);
            Array.Copy(other.SlotColors, SlotColors, SlotColors.Length);
            Array.Copy(other.SlotDarks, SlotDarks, SlotDarks.Length);
            Array.Copy(other.SlotAttachments, SlotAttachments, SlotAttachments.Length);
            Array.Copy(other.DrawOrder, DrawOrder, DrawOrder.Length);
            Deform.Clear();
            foreach (var kv in other.Deform)
            {
                var buf = DeformBuffer(kv.Key, kv.Value.Length);
                Array.Copy(kv.Value, buf, buf.Length);
                Deform[kv.Key] = buf;
            }
            SequenceFrames.Clear();
            foreach (var kv in other.SequenceFrames) SequenceFrames[kv.Key] = kv.Value;
            for (int i = 0; i < ConstraintValues.Length; i++) Array.Copy(other.ConstraintValues[i], ConstraintValues[i], ConstraintValues[i].Length);
        }

        /// <summary>제약의 셋업 값(핵심 constraint_setup_values).</summary>
        public static float[] SetupValues(ConstraintData c)
        {
            var v = new float[ValueCount(c.Type)];
            SetupValues(c, v);
            return v;
        }

        public static int ValueCount(ConstraintType t)
        {
            switch (t)
            {
                case ConstraintType.Ik: return 5;
                case ConstraintType.Transform: return 6;
                case ConstraintType.Path: return 5;
                case ConstraintType.Physics: return 7;
                default: return 2;
            }
        }

        static float B(bool v) => v ? 1f : 0f;

        static void SetupValues(ConstraintData c, float[] v)
        {
            switch (c)
            {
                case IkConstraintData k:
                    v[0] = k.Mix; v[1] = k.Softness; v[2] = k.BendPositive ? 1f : -1f; v[3] = B(k.Compress); v[4] = B(k.Stretch);
                    break;
                case TransformConstraintData t:
                    v[0] = t.MixRotate; v[1] = t.MixX; v[2] = t.MixY; v[3] = t.MixScaleX; v[4] = t.MixScaleY; v[5] = t.MixShearY;
                    break;
                case PathConstraintData p:
                    v[0] = p.Position; v[1] = p.Spacing; v[2] = p.MixRotate; v[3] = p.MixX; v[4] = p.MixY;
                    break;
                case PhysicsConstraintData p:
                    v[0] = p.Inertia; v[1] = p.Strength; v[2] = p.Damping; v[3] = p.Mass; v[4] = p.Wind; v[5] = p.Gravity; v[6] = p.Mix;
                    break;
                case SliderConstraintData s:
                    v[0] = s.Time; v[1] = s.Mix;
                    break;
            }
        }
    }
}
