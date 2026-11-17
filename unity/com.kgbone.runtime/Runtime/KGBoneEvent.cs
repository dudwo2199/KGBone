// 재생 중 발생한 애니메이션 이벤트(게임 코드가 받는 값).
namespace KGBone
{
    public readonly struct KGBoneEvent
    {
        public readonly string Name;
        /// <summary>키에 적힌 시간(애니메이션 안의 시간, 반복 바퀴를 더하지 않음).</summary>
        public readonly float Time;
        public readonly int Int;
        public readonly float Float;
        public readonly string String;
        public readonly float Volume;
        public readonly float Balance;
        /// <summary>이벤트를 낸 트랙 번호(Play로 재생하면 0).</summary>
        public readonly int Track;
        /// <summary>이벤트를 낸 애니메이션 이름.</summary>
        public readonly string Animation;

        public KGBoneEvent(string name, float time, int i, float f, string s, float volume, float balance)
            : this(name, time, i, f, s, volume, balance, 0, null) { }

        public KGBoneEvent(string name, float time, int i, float f, string s, float volume, float balance, int track, string animation)
        {
            Track = track;
            Animation = animation;
            Name = name;
            Time = time;
            Int = i;
            Float = f;
            String = s;
            Volume = volume;
            Balance = balance;
        }
    }
}
