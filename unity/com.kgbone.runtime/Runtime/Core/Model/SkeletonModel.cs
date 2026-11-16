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

        // 이름 → 번호 사전. 매 프레임 찾으므로 람다 검색(할당) 대신 사전을 쓴다. 목록 길이가 바뀌면(읽는 중) 다시 만든다.
        // 이름이 겹치면 처음 나온 것(List.FindIndex와 같음).
        sealed class NameIndex
        {
            readonly Dictionary<string, int> map = new Dictionary<string, int>(System.StringComparer.Ordinal);
            int builtCount = -1;

            public int Find<T>(List<T> list, System.Func<T, string> nameOf, string name)
            {
                if (name == null) return -1;
                if (builtCount != list.Count)
                {
                    map.Clear();
                    for (int i = 0; i < list.Count; i++)
                    {
                        string n = nameOf(list[i]);
                        if (n != null && !map.ContainsKey(n)) map[n] = i;
                    }
                    builtCount = list.Count;
                }
                return map.TryGetValue(name, out int k) ? k : -1;
            }
        }

        static readonly System.Func<BoneData, string> BoneName = b => b.Name;
        static readonly System.Func<SlotData, string> SlotName = s => s.Name;
        static readonly System.Func<SkinData, string> SkinName = s => s.Name;
        static readonly System.Func<AnimationData, string> AnimationName = a => a.Name;
        static readonly System.Func<EventData, string> EventName = e => e.Name;
        readonly NameIndex bones = new NameIndex(), slots = new NameIndex(), skins = new NameIndex(), animations = new NameIndex(), events = new NameIndex();

        public int BoneIndex(string name) => bones.Find(Bones, BoneName, name);
        public int SlotIndex(string name) => slots.Find(Slots, SlotName, name);

        public SkinData FindSkin(string name)
        {
            int i = skins.Find(Skins, SkinName, name);
            return i < 0 ? null : Skins[i];
        }

        public AnimationData FindAnimation(string name)
        {
            int i = animations.Find(Animations, AnimationName, name);
            return i < 0 ? null : Animations[i];
        }

        public EventData FindEvent(string name)
        {
            int i = events.Find(Events, EventName, name);
            return i < 0 ? null : Events[i];
        }
    }
}
