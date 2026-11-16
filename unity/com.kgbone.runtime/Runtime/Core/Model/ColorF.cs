// 0~1 색. 파일 표기는 "RRGGBBAA"(어두운 색은 "RRGGBB").
using System.Globalization;

namespace KGBone.Core
{
    public struct ColorF
    {
        public float R, G, B, A;

        public ColorF(float r, float g, float b, float a) { R = r; G = g; B = b; A = a; }

        public static readonly ColorF White = new ColorF(1, 1, 1, 1);

        /// <summary>"RRGGBBAA" 또는 "RRGGBB"(알파 1). 형식이 틀리면 fallback.</summary>
        public static ColorF Parse(string hex, ColorF fallback)
        {
            if (hex == null || (hex.Length != 6 && hex.Length != 8)) return fallback;
            if (!uint.TryParse(hex, NumberStyles.HexNumber, CultureInfo.InvariantCulture, out uint v)) return fallback;
            if (hex.Length == 6) v = (v << 8) | 0xFF;
            return new ColorF(((v >> 24) & 0xFF) / 255f, ((v >> 16) & 0xFF) / 255f, ((v >> 8) & 0xFF) / 255f, (v & 0xFF) / 255f);
        }

        public static ColorF operator *(ColorF a, ColorF b) => new ColorF(a.R * b.R, a.G * b.G, a.B * b.B, a.A * b.A);
    }
}
