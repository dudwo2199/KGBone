// 키 사이 보간(선형·계단·베지어). KGBone 핵심 crates/kgbone-core/src/anim/curve.rs와 같은 규칙:
// - 첫 키 앞은 첫 키 값, 마지막 키 뒤는 마지막 키 값.
// - 앞 키의 curve가 두 키 사이 모양을 정한다. 베지어는 값 채널마다 [cx1, cy1, cx2, cy2](절대 시간·값), 그 채널 곡선이 없으면 선형.
using System;

namespace KGBone.Core
{
    public static class Curve
    {
        /// <summary>time이 들어가는 구간. between이면 index와 index+1 사이, 아니면 index 키 값 그대로. 키가 없으면 -1.</summary>
        static int Segment(Key[] keys, float time, out bool between)
        {
            between = false;
            if (keys == null || keys.Length == 0) return -1;
            if (time <= keys[0].Time) return 0;
            int last = keys.Length - 1;
            if (time >= keys[last].Time) return last;
            int lo = 0, hi = last; // keys[lo].Time <= time < keys[hi].Time
            while (hi - lo > 1)
            {
                int mid = (lo + hi) / 2;
                if (keys[mid].Time <= time) lo = mid;
                else hi = mid;
            }
            between = true;
            return lo;
        }

        /// <summary>시점의 앞쪽(같거나 이전) 키 번호. 첫 키 앞이면 0, 키가 없으면 -1.</summary>
        public static int StepIndex(Key[] keys, float time) => Segment(keys, time, out _);

        static float Channel(Key k, int ch) => ch < k.V.Length ? k.V[ch] : 0f;

        public static float ValueAt(Key[] keys, int channel, float time)
        {
            int i = Segment(keys, time, out bool between);
            if (i < 0) return 0f;
            if (!between) return Channel(keys[i], channel);
            Key k0 = keys[i], k1 = keys[i + 1];
            float v0 = Channel(k0, channel), v1 = Channel(k1, channel);
            switch (k0.Curve)
            {
                case CurveType.Stepped:
                    return v0;
                case CurveType.Bezier:
                    var c = k0.Bezier;
                    int b = channel * 4;
                    if (c != null && b + 4 <= c.Length) return Bezier(k0.Time, v0, k1.Time, v1, c[b], c[b + 1], c[b + 2], c[b + 3], time);
                    return LerpTime(k0.Time, v0, k1.Time, v1, time);
                default:
                    return LerpTime(k0.Time, v0, k1.Time, v1, time);
            }
        }

        /// <summary>두 키 사이 진행 비율(0~1). 베지어는 첫 채널을 (t0, 0)–(t1, 1) 비율 곡선으로 본다. 키가 없으면 false.</summary>
        public static bool RatioAt(Key[] keys, float time, out int index, out float ratio)
        {
            index = Segment(keys, time, out bool between);
            ratio = 0f;
            if (index < 0) return false;
            if (!between) return true;
            Key k0 = keys[index], k1 = keys[index + 1];
            switch (k0.Curve)
            {
                case CurveType.Stepped:
                    ratio = 0f;
                    break;
                case CurveType.Bezier when k0.Bezier != null && k0.Bezier.Length >= 4:
                    ratio = Bezier(k0.Time, 0f, k1.Time, 1f, k0.Bezier[0], k0.Bezier[1], k0.Bezier[2], k0.Bezier[3], time);
                    break;
                default:
                    ratio = LerpTime(k0.Time, 0f, k1.Time, 1f, time);
                    break;
            }
            return true;
        }

        static float LerpTime(float t0, float v0, float t1, float v1, float time)
        {
            float span = t1 - t0;
            if (span <= 0f) return v1;
            float p = Math.Min(Math.Max((time - t0) / span, 0f), 1f);
            return v0 + (v1 - v0) * p;
        }

        static double Cubic(double p0, double p1, double p2, double p3, double s)
        {
            double u = 1.0 - s;
            return u * u * u * p0 + 3.0 * u * u * s * p1 + 3.0 * u * s * s * p2 + s * s * s * p3;
        }

        /// <summary>
        /// 베지어 [cx1, cy1, cx2, cy2](절대 좌표)로 (t0, v0)–(t1, v1) 사이 시점의 값. 시간 성분을 s에 대해 60회 이분법으로 풀고
        /// (제어점 시간은 [t0, t1]로 잘라 단조 증가하게) 값 성분을 돌려준다. 계산은 double(핵심과 같음).
        /// </summary>
        public static float Bezier(float t0, float v0, float t1, float v1, float cx1, float cy1, float cx2, float cy2, float time)
        {
            if (t1 <= t0) return v1;
            if (time <= t0) return v0;
            if (time >= t1) return v1;
            double t0d = t0, t1d = t1;
            double x1 = Math.Min(Math.Max((double)cx1, t0d), t1d);
            double x2 = Math.Min(Math.Max((double)cx2, t0d), t1d);
            double target = time;
            double lo = 0.0, hi = 1.0;
            for (int n = 0; n < 60; n++)
            {
                double mid = 0.5 * (lo + hi);
                if (Cubic(t0d, x1, x2, t1d, mid) < target) lo = mid;
                else hi = mid;
            }
            double s = 0.5 * (lo + hi);
            return (float)Cubic(v0, cy1, cy2, v1, s);
        }
    }
}
