// 계산 중 상태(핵심 pose/mod.rs의 SkeletonPose에서 제약·시퀀스를 뺀 것). 배열은 한 번 만들고 다시 쓴다.
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
        /// <summary>(슬롯, 첨부물) → 셋업 정점에 더할 변형 값.</summary>
        public readonly Dictionary<(string slot, string attachment), float[]> Deform = new Dictionary<(string, string), float[]>();

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
        }
    }
}
