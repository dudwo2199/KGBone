// 켠 스킨 목록. 같은 (슬롯, 이름) 첨부물은 뒤에 적은 스킨이 이긴다. 기본 스킨은 항상 맨 앞에 있는 것처럼 다룬다.
using System.Collections.Generic;

namespace KGBone.Core
{
    public sealed class SkinSet
    {
        static readonly string[] None = new string[0];
        readonly string[] names;

        SkinSet(string[] names) { this.names = names; }

        /// <summary>스킨 하나(null이면 기본 스킨만).</summary>
        public static SkinSet Single(string skin) => new SkinSet(string.IsNullOrEmpty(skin) ? None : new[] { skin });

        /// <summary>켠 스킨 목록(앞에서 뒤로, 뒤가 이김). 빈 이름은 건너뛴다.</summary>
        public static SkinSet Of(IReadOnlyList<string> skins)
        {
            var list = new List<string>();
            if (skins != null) foreach (var s in skins) if (!string.IsNullOrEmpty(s)) list.Add(s);
            return new SkinSet(list.ToArray());
        }

        public IReadOnlyList<string> Names => names;

        /// <summary>첫 스킨(없으면 null). 스킨 하나만 받는 옛 경로가 쓴다.</summary>
        public string Primary => names.Length > 0 ? names[0] : null;
    }
}
