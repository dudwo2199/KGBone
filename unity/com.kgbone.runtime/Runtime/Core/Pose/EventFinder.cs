// 구간 안에서 발생한 이벤트. KGBone 핵심 crates/kgbone-core/src/pose/events.rs와 같은 규칙:
// 구간은 [last, time)(끝 제외). 반복 재생이고 길이 d > 0이면
// - last ≤ time: 실제 시간축에서 바퀴 k마다 (키 시간 + k·d)가 [last, time)에 드는 것
// - last > time(이미 길이로 나눈 시점): [last, d] ∪ [0, time)
using System;
using System.Collections.Generic;

namespace KGBone.Core
{
    /// <summary>발생한 이벤트. Time은 키에 적힌 시간(애니메이션 안의 시간). 키에 없는 값은 이벤트 정의의 기본값.</summary>
    public struct FiredEvent
    {
        public string Name;
        public float Time;
        public int Int;
        public float Float;
        public string String;
        public float Volume;
        public float Balance;
        /// <summary>트랙 번호(트랙 재생이 아니면 0).</summary>
        public int Track;
        /// <summary>이벤트를 낸 애니메이션 이름.</summary>
        public string Animation;
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
                            if (at >= last && at < time) output.Add(Make(m, keys[i]));
                        }
                }
                else
                {
                    for (int i = 0; i < keys.Count; i++)
                        if (keys[i].Time >= last && keys[i].Time <= d) output.Add(Make(m, keys[i]));
                    for (int i = 0; i < keys.Count; i++)
                        if (keys[i].Time < time) output.Add(Make(m, keys[i]));
                }
            }
            else
            {
                for (int i = 0; i < keys.Count; i++)
                    if (keys[i].Time >= last && keys[i].Time < time) output.Add(Make(m, keys[i]));
            }
        }

        /// <summary>
        /// 키 시간이 정확히 time인 이벤트. 핵심의 구간 규칙([last, time), 끝 제외)으로는 반복하지 않는 애니메이션의
        /// 끝 시간 키와 길이 0 애니메이션의 키가 나지 않으므로, 재생 컴포넌트가 끝에 도달한 프레임에 이것으로 보탠다.
        /// </summary>
        /// <summary>시간 목록 중 하나라도 구간에 드는가(Fired와 같은 구간 규칙, 핵심 events.rs의 hits). 물리 reset 키가 쓴다.</summary>
        public static bool AnyHit(Key[] keys, float last, float time, bool looping, float d)
        {
            if (looping && d > 0f)
            {
                if (last <= time)
                {
                    long k0 = (long)Math.Floor(last / d), k1 = (long)Math.Floor(time / d);
                    for (long k = k0; k <= k1; k++)
                        foreach (var key in keys)
                        {
                            float at = key.Time + k * d;
                            if (at >= last && at < time) return true;
                        }
                    return false;
                }
                foreach (var key in keys) if (key.Time >= last && key.Time <= d) return true;
                foreach (var key in keys) if (key.Time < time) return true;
                return false;
            }
            foreach (var key in keys) if (key.Time >= last && key.Time < time) return true;
            return false;
        }

        public static void KeysAt(SkeletonModel m, AnimationData a, float time, List<FiredEvent> output)
        {
            for (int i = 0; i < a.Events.Count; i++)
                if (a.Events[i].Time == time) output.Add(Make(m, a.Events[i]));
        }

        static FiredEvent Make(SkeletonModel m, EventKey k)
        {
            var def = m.FindEvent(k.Name);
            return new FiredEvent
            {
                Name = k.Name,
                Time = k.Time, // 핵심처럼 키에 적힌 시간(반복 바퀴를 더한 실제 시간이 아님)
                Int = k.Int ?? def?.Int ?? 0,
                Float = k.Float ?? def?.Float ?? 0f,
                String = k.String ?? def?.String ?? "",
                Volume = k.Volume ?? def?.Volume ?? 1f,
                Balance = k.Balance ?? def?.Balance ?? 0f,
            };
        }
    }
}
