// 제약 데이터(FORMAT.md 3.3, 핵심 model/constraint.rs). 모델의 Constraints 한 배열에 적용 순서대로 들어간다.
// 본·슬롯은 이름 대신 인덱스로 풀어 둔다(못 찾으면 -1).
using System.Collections.Generic;

namespace KGBone.Core
{
    public enum ConstraintType { Ik, Transform, Path, Physics, Slider }

    /// <summary>트랜스폼·슬라이더가 읽고 쓰는 본 속성(핵심 BoneProperty와 같은 순서).</summary>
    public enum BoneProperty { Rotate, X, Y, ScaleX, ScaleY, ShearY }

    public enum PositionMode { Fixed, Percent }
    public enum SpacingMode { Length, Fixed, Percent, Proportional }
    public enum RotateMode { Tangent, Chain, ChainScale }

    public abstract class ConstraintData
    {
        public string Name;
        public bool SkinRequired;
        public abstract ConstraintType Type { get; }
    }

    public sealed class IkConstraintData : ConstraintData
    {
        public override ConstraintType Type => ConstraintType.Ik;
        public int[] Bones = new int[0];
        public int Target = -1;
        public float Mix = 1, Softness;
        public bool BendPositive = true, Compress, Stretch, Uniform, Volume;
    }

    public sealed class TransformTo
    {
        public BoneProperty Property = BoneProperty.X;
        public float Offset, Max, Scale = 1;
    }

    public sealed class TransformMapping
    {
        public BoneProperty From = BoneProperty.X;
        public float Offset;
        public List<TransformTo> To = new List<TransformTo>();
    }

    public sealed class TransformConstraintData : ConstraintData
    {
        public override ConstraintType Type => ConstraintType.Transform;
        public int[] Bones = new int[0];
        public int Source = -1;
        public bool LocalSource, LocalTarget, Additive, Clamp;
        public float MixRotate = 1, MixX = 1, MixY = 1, MixScaleX = 1, MixScaleY = 1, MixShearY = 1;
        /// <summary>실제로 쓰는 매핑(파일에 없으면 6개 속성의 같은 속성 기본 매핑, 핵심 effective_properties).</summary>
        public List<TransformMapping> Properties = new List<TransformMapping>();

        public float MixOf(BoneProperty p)
        {
            switch (p)
            {
                case BoneProperty.Rotate: return MixRotate;
                case BoneProperty.X: return MixX;
                case BoneProperty.Y: return MixY;
                case BoneProperty.ScaleX: return MixScaleX;
                case BoneProperty.ScaleY: return MixScaleY;
                default: return MixShearY;
            }
        }
    }

    public sealed class PathConstraintData : ConstraintData
    {
        public override ConstraintType Type => ConstraintType.Path;
        public int[] Bones = new int[0];
        /// <summary>패스 첨부물이 있는 슬롯.</summary>
        public int Target = -1;
        public PositionMode PositionMode = PositionMode.Percent;
        public SpacingMode SpacingMode = SpacingMode.Length;
        public RotateMode RotateMode = RotateMode.Tangent;
        public float OffsetRotation, Position, Spacing, MixRotate = 1, MixX = 1, MixY = 1;
    }

    public sealed class PhysicsConstraintData : ConstraintData
    {
        public override ConstraintType Type => ConstraintType.Physics;
        public int Bone = -1;
        public float X, Y, Rotate, ScaleX, ShearX;
        public float Limit = 5000, Fps = 60, Inertia = 1, Strength = 100, Damping = 1, Mass = 1, Wind, Gravity, Mix = 1;
        public bool Volume;
    }

    public sealed class SliderConstraintData : ConstraintData
    {
        public override ConstraintType Type => ConstraintType.Slider;
        public string Animation = "";
        public float Time, Mix = 1;
        public bool Loop, Additive, Local;
        /// <summary>본 이름이 있는데 뼈대에 없음(제약을 계산하지 않는다).</summary>
        public const int MissingBone = -2;
        /// <summary>시점을 읽어 올 본(-1이면 time 값을 쓴다, MissingBone이면 비활성).</summary>
        public int Bone = -1;
        public BoneProperty Property = BoneProperty.X;
        public float PropertyFrom, PropertyTo = 1;
    }
}
