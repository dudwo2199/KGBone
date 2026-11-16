// 모델의 이름 있는 값들(FORMAT.md 3장). 문자열 → 값 변환은 모르는 값이면 기본값.
namespace KGBone.Core
{
    public enum Inherit { Normal, OnlyTranslation, NoRotationOrReflection, NoScale, NoScaleOrReflection }

    public enum BlendMode { Normal, Additive, Multiply, Screen }

    public enum CurveType { Linear, Stepped, Bezier }

    public static class ModelNames
    {
        /// <summary>상속 이름. 모르는 이름이면 null(핵심의 Inherit::parse와 같음).</summary>
        public static Inherit? ParseInherit(string s)
        {
            switch (s)
            {
                case "normal": return Inherit.Normal;
                case "onlyTranslation": return Inherit.OnlyTranslation;
                case "noRotationOrReflection": return Inherit.NoRotationOrReflection;
                case "noScale": return Inherit.NoScale;
                case "noScaleOrReflection": return Inherit.NoScaleOrReflection;
                default: return null;
            }
        }

        public static BlendMode ParseBlend(string s)
        {
            switch (s)
            {
                case "additive": return BlendMode.Additive;
                case "multiply": return BlendMode.Multiply;
                case "screen": return BlendMode.Screen;
                default: return BlendMode.Normal;
            }
        }

        public static string BlendName(BlendMode m)
        {
            switch (m)
            {
                case BlendMode.Additive: return "additive";
                case BlendMode.Multiply: return "multiply";
                case BlendMode.Screen: return "screen";
                default: return "normal";
            }
        }
    }
}
