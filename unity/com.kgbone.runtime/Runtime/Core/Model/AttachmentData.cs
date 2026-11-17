// 첨부물(FORMAT.md 3.5). 1단계에서 그리는 것은 region, mesh, linkedmesh. 나머지는 OtherAttachment.
using System.Globalization;

namespace KGBone.Core
{
    public sealed class SequenceData
    {
        public int Count = 1, Start = 1, Digits, Setup;
    }

    public abstract class AttachmentData
    {
        /// <summary>이미지 경로(없으면 null = 키 이름).</summary>
        public string Path;
        public SequenceData Sequence;

        /// <summary>그릴 이미지 경로(확장자 없음). 시퀀스면 셋업 프레임 번호를 붙인다. 그림이 없는 종류는 null.</summary>
        public string ImagePath(string key) => ImagePath(key, null);

        /// <summary>시퀀스 프레임 frame(없으면 셋업 프레임)의 이미지 경로. 그림이 없는 종류는 null.</summary>
        public virtual string ImagePath(string key, int? frame)
        {
            string basePath = Path ?? key;
            if (Sequence == null) return basePath;
            int number = System.Math.Max(frame ?? Sequence.Setup, 0) + Sequence.Start;
            // 핵심 pose/draw.rs의 sequence_path: 번호를 digits 자리로 0 채움(digits 0이면 채우지 않음)
            return basePath + number.ToString(CultureInfo.InvariantCulture).PadLeft(Sequence.Digits, '0');
        }
    }

    public sealed class RegionAttachment : AttachmentData
    {
        public float X, Y, Rotation, ScaleX = 1, ScaleY = 1, Width = 32, Height = 32;
        public ColorF Color = ColorF.White;
    }

    public sealed class MeshAttachment : AttachmentData
    {
        public float Width = 32, Height = 32;
        public ColorF Color = ColorF.White;
        public float[] Uvs = new float[0];
        public int[] Triangles = new int[0];
        public float[] Vertices = new float[0];
        public bool Weighted;
    }

    public sealed class LinkedMeshAttachment : AttachmentData
    {
        public ColorF Color = ColorF.White;
        public string Parent;
        /// <summary>부모 메시가 있는 스킨(null = 기본 스킨).</summary>
        public string Skin;
        public bool Timelines = true;
    }

    /// <summary>패스 첨부물(패스 제약 계산용, 그리지 않음).</summary>
    public sealed class PathAttachmentData : AttachmentData
    {
        public bool Closed, ConstantSpeed = true, Weighted;
        public int VertexCount;
        public float[] Vertices = new float[0];
        /// <summary>곡선 구간별 누적 길이.</summary>
        public float[] Lengths = new float[0];
        public override string ImagePath(string key, int? frame) => null;
    }

    /// <summary>그리지 않는 종류(boundingbox, point, clipping).</summary>
    /// <summary>클리핑(그리지 않음): 이 슬롯부터 End 슬롯(포함, 없으면 끝까지)까지 그려지는 것을 다각형으로 자른다.</summary>
    public sealed class ClippingAttachmentData : AttachmentData
    {
        public string End;
        public int VertexCount;
        public bool Weighted, Inverse, Convex;
        public float[] Vertices = new float[0];
        public override string ImagePath(string key, int? frame) => null;
    }

    public sealed class OtherAttachment : AttachmentData
    {
        public string Type;
        public override string ImagePath(string key, int? frame) => null;
    }
}
