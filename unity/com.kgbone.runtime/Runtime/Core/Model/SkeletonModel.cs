// 뼈대 전체(FORMAT.md 2~3장). KGBoneLoader가 만든다.
using System.Collections.Generic;

namespace KGBone.Core
{
    public sealed class EventData
    {
        public string Name;
        public int Int;
        public float Float;
        public string String = "";
        public float Volume = 1, Balance;
    }

    public sealed class SkeletonModel
    {
        public string ImagesPath = "./images";
        public float Fps = 30;
        public List<BoneData> Bones = new List<BoneData>();
        public List<SlotData> Slots = new List<SlotData>();
        public List<SkinData> Skins = new List<SkinData>();
        public List<EventData> Events = new List<EventData>();
        public List<AnimationData> Animations = new List<AnimationData>();
        /// <summary>제약 개수(1단계는 계산하지 않고 경고만).</summary>
        public int ConstraintCount;
        public bool HasClipping;
        public bool HasSequence;

        public int BoneIndex(string name) => Bones.FindIndex(b => b.Name == name);
        public int SlotIndex(string name) => Slots.FindIndex(s => s.Name == name);
        public SkinData FindSkin(string name) => name == null ? null : Skins.Find(s => s.Name == name);
        public AnimationData FindAnimation(string name) => name == null ? null : Animations.Find(a => a.Name == name);
        public EventData FindEvent(string name) => Events.Find(e => e.Name == name);
    }
}
