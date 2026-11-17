// 시퀀스 키(s = 모드, v = [시작 프레임, 프레임 간격]). KGBone 핵심 crates/kgbone-core/src/anim/attachment_keys.rs의 sequence_frame과 같은 계산.
// 지난 프레임 수 f = ⌊(t − 키 시간) / 간격 + 1e-4⌋(간격 0이면 0). 역방향 모드는 정방향 결과를 (count − 1 − 번호)로 뒤집는다.
using System;

namespace KGBone.Core
{
    public static class SequenceKeys
    {
        /// <summary>시점 t의 프레임 번호(0 ~ count−1). 키가 없으면 null.</summary>
        public static int? Frame(Key[] keys, float t, int count)
        {
            int ki = Curve.StepIndex(keys, t);
            if (ki < 0) return null;
            var k = keys[ki];
            int n = Math.Max(count, 1);
            int start = (int)(k.V.Length > 0 ? k.V[0] : 0f);
            float delay = k.V.Length > 1 ? k.V[1] : 0f;
            int f = delay > 0f ? (int)MathF.Floor(Math.Max(t - k.Time, 0f) / delay + 1e-4f) : 0;
            int p = start + f;
            int idx;
            switch (k.S ?? "hold")
            {
                case "once": idx = Math.Min(p, n - 1); break;
                case "loop": idx = Mod(p, n); break;
                case "pingpong": idx = PingPong(p, n); break;
                case "onceReverse": idx = n - 1 - Math.Min(p, n - 1); break;
                case "loopReverse": idx = n - 1 - Mod(p, n); break;
                case "pingpongReverse": idx = n - 1 - PingPong(p, n); break;
                default: idx = start; break;
            }
            return Math.Min(Math.Max(idx, 0), n - 1);
        }

        /// <summary>Rust rem_euclid(항상 0 이상).</summary>
        static int Mod(int a, int n)
        {
            int r = a % n;
            return r < 0 ? r + n : r;
        }

        static int PingPong(int p, int n)
        {
            int n2 = (n - 1) * 2;
            if (n2 <= 0) return 0;
            int q = Mod(p, n2);
            return q >= n ? n2 - q : q;
        }
    }
}
