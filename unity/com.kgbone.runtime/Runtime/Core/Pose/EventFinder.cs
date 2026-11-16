// 구간 안에서 발생한 이벤트. KGBone 핵심 crates/kgbone-core/src/pose/events.rs와 같은 규칙:
// 구간은 [last, time)(끝 제외). 반복 재생이고 길이 d > 0이면
// - last ≤ time: 실제 시간축에서 바퀴 k마다 (키 시간 + k·d)가 [last, time)에 드는 것
// - last > time(이미 길이로 나눈 시점): [last, d] ∪ [0, time)
using System;
using System.Collections.Generic;

namespace KGBone.Core
{
    /// <summary>발생한 이벤트. 키에 없는 값은 이벤트 정의의 기본값.</summary>
    public struct FiredEvent
    {
        public string Name;
        public float Time;
        public int Int;
        public float Float;
        public string String;
        public float Volume;
        public float Balance;
    }

    public static class EventFinder
    {
        public static void Fired(SkeletonModel m, AnimationData a, float last, float time, bool looping, List<FiredEvent> output)
        {
            var keys = a.Events;
            float d = a.Duration;
            if (looping && d > 0f)
            {
                if (last <= time)
                {
                    long k0 = (long)Math.Floor(last / d), k1 = (long)Math.Floor(time / d);
                    for (long k = k0; k <= k1; k++)
                        for (int i = 0; i < keys.Count; i++)
                        {
                            float at = keys[i].Time + k * d;
                            // 길이와 같은 시간의 키는 다음 바퀴의 0초 키와 같은 순간에 함께 날 수 있다
                            if (at >= last && at < time) output.Add(Make(m, keys[i], at));
                        }
                }
                else
                {
                    for (int i = 0; i < keys.Count; i++)
                        if (keys[i].Time >= last && keys[i].Time <= d) output.Add(Make(m, keys[i], keys[i].Time));
                    for (int i = 0; i < keys.Count; i++)
                        if (keys[i].Time < time) output.Add(Make(m, keys[i], keys[i].Time));
                }
            }
            else
            {
                for (int i = 0; i < keys.Count; i++)
                    if (keys[i].Time >= last && keys[i].Time < time) output.Add(Make(m, keys[i], keys[i].Time));
            }
        }

        static FiredEvent Make(SkeletonModel m, EventKey k, float at)
        {
            var def = m.FindEvent(k.Name);
            return new FiredEvent
            {
                Name = k.Name,
                Time = at,
                Int = k.Int ?? def?.Int ?? 0,
                Float = k.Float ?? def?.Float ?? 0f,
                String = k.String ?? def?.String ?? "",
                Volume = k.Volume ?? def?.Volume ?? 1f,
                Balance = k.Balance ?? def?.Balance ?? 0f,
            };
        }
    }
}
