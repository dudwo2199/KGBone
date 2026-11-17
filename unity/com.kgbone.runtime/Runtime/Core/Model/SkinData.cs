// 스킨(FORMAT.md 3.4): 슬롯 이름 → 첨부물 이름 → 첨부물.
using System.Collections.Generic;

namespace KGBone.Core
{
    public sealed class SkinData
    {
        public const string DefaultName = "default";

        public string Name;
        public List<string> Bones = new List<string>();
        /// <summary>이 스킨이 켜는 스킨 전용 제약 이름.</summary>
        public List<string> Constraints = new List<string>();
        internal readonly Dictionary<string, Dictionary<string, AttachmentData>> Attachments =
            new Dictionary<string, Dictionary<string, AttachmentData>>(System.StringComparer.Ordinal);

        /// <summary>슬롯의 첨부물. 없으면 null.</summary>
        public AttachmentData Get(string slot, string name)
        {
            if (slot == null || name == null) return null;
            return Attachments.TryGetValue(slot, out var m) && m.TryGetValue(name, out var a) ? a : null;
        }

        /// <summary>모든 (슬롯, 첨부물 이름, 첨부물).</summary>
        public IEnumerable<(string slot, string name, AttachmentData attachment)> All()
        {
            foreach (var s in Attachments)
                foreach (var a in s.Value)
                    yield return (s.Key, a.Key, a.Value);
        }
    }
}
