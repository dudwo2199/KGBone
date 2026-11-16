// 애니메이션(FORMAT.md 3.7). 타임라인은 파일 순서대로 둔다(적용 순서가 같아야 결과가 같다).
using System.Collections.Generic;

namespace KGBone.Core
{
    public sealed class BoneTimeline
    {
        public int Bone;
        public string Name;
        public Key[] Keys;
    }

    public sealed class SlotTimeline
    {
        public int Slot;
        public string Name;
        public Key[] Keys;
    }

    public sealed class DeformTimeline
    {
        public string Skin, Slot, Attachment;
        public Key[] Keys;
    }

    public sealed class DrawOrderKey
    {
        public float Time;
        public List<(string slot, int offset)> Offsets = new List<(string, int)>();
    }

    public sealed class EventKey
    {
        public float Time;
        public string Name;
        public int? Int;
        public float? Float;
        public string String;
        public float? Volume, Balance;
    }

    public sealed class AnimationData
    {
        public string Name;
        public List<BoneTimeline> Bones = new List<BoneTimeline>();
        public List<SlotTimeline> Slots = new List<SlotTimeline>();
        public List<DeformTimeline> Deform = new List<DeformTimeline>();
        public List<DrawOrderKey> DrawOrder = new List<DrawOrderKey>();
        public List<EventKey> Events = new List<EventKey>();
        /// <summary>모든 타임라인의 마지막 키 중 가장 늦은 시간(핵심 model/animation.rs의 duration).</summary>
        public float Duration;
    }
}
