// 본(FORMAT.md 3.1). Parent는 skeleton.bones 배열 번호, 없으면 -1(부모는 항상 자식보다 앞).
namespace KGBone.Core
{
    public sealed class BoneData
    {
        public string Name;
        public int Parent = -1;
        public float Length, X, Y, Rotation, ScaleX = 1, ScaleY = 1, ShearX, ShearY;
        public Inherit Inherit = Inherit.Normal;
        public bool SkinRequired;
    }
}
