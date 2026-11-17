// 켠 스킨 목록. 같은 (슬롯, 이름) 첨부물은 뒤에 적은 스킨이 이기고, 어느 스킨에도 없으면 기본 스킨.
// 스킨 전용 본·제약은 켠 스킨 중 하나라도 가지면 켜진다. 스킨이 하나면 핵심의 "켠 스킨 → 기본 스킨"과 같다.
// null인 SkinSet은 기본 스킨만 켠 것으로 다룬다. 문자열 하나는 그 스킨 하나로 바뀐다(암시적 변환).
using System.Collections.Generic;

namespace KGBone.Core
{
    public sealed class SkinSet
    {
        static readonly string[] None = new string[0];
        readonly string[] names;

        SkinSet(string[] names) { this.names = names; }

        /// <summary>스킨 하나(null·빈 이름이면 기본 스킨만).</summary>
        public static SkinSet Single(string skin) => new SkinSet(string.IsNullOrEmpty(skin) ? None : new[] { skin });

        /// <summary>켠 스킨 목록(앞에서 뒤로, 뒤가 이김). 빈 이름은 건너뛴다.</summary>
        public static SkinSet Of(IReadOnlyList<string> skins)
        {
            var list = new List<string>();
            if (skins != null) foreach (var s in skins) if (!string.IsNullOrEmpty(s)) list.Add(s);
            return new SkinSet(list.ToArray());
        }

        public static implicit operator SkinSet(string skin) => Single(skin);

        public IReadOnlyList<string> Names => names;

        /// <summary>첫 스킨(없으면 null).</summary>
        public string Primary => names.Length > 0 ? names[0] : null;

        /// <summary>이름 목록이 같은가(순서 포함, 빈 이름은 건너뛰고 비교).</summary>
        public bool SameAs(IReadOnlyList<string> skins)
        {
            int k = 0;
            if (skins != null)
                foreach (var s in skins)
                {
                    if (string.IsNullOrEmpty(s)) continue;
                    if (k >= names.Length || names[k] != s) return false;
                    k++;
                }
            return k == names.Length;
        }

        /// <summary>첨부물 찾기: 뒤 스킨 → … → 앞 스킨 → 기본 스킨(핵심 find_attachment를 여러 스킨으로 넓힘).</summary>
        public static AttachmentData Find(SkinSet set, SkeletonModel m, string slot, string name)
        {
            if (set != null)
                for (int i = set.names.Length - 1; i >= 0; i--)
                {
                    var a = m.FindSkin(set.names[i])?.Get(slot, name);
                    if (a != null) return a;
                }
            return m.FindSkin(SkinData.DefaultName)?.Get(slot, name);
        }

        /// <summary>이 스킨의 키(deform·시퀀스)를 쓰는가: 기본 스킨이거나 켠 스킨.</summary>
        public static bool Includes(SkinSet set, string skin)
        {
            if (skin == SkinData.DefaultName) return true;
            if (set == null) return false;
            foreach (var n in set.names) if (n == skin) return true;
            return false;
        }

        public static bool HasBone(SkinSet set, SkeletonModel m, string bone)
        {
            if (set == null) return false;
            foreach (var n in set.names)
                if (m.FindSkin(n)?.Bones.Contains(bone) == true) return true;
            return false;
        }

        public static bool HasConstraint(SkinSet set, SkeletonModel m, string constraint)
        {
            if (set == null) return false;
            foreach (var n in set.names)
                if (m.FindSkin(n)?.Constraints.Contains(constraint) == true) return true;
            return false;
        }
    }
}
