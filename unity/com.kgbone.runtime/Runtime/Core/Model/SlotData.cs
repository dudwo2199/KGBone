// 슬롯(FORMAT.md 3.2). 배열 순서 = 셋업 그리기 순서(앞이 뒤에 그려짐).
namespace KGBone.Core
{
    public sealed class SlotData
    {
        public string Name;
        public int Bone = -1;
        public ColorF Color = ColorF.White;
        /// <summary>어두운 색(tint black). 없으면 꺼짐.</summary>
        public ColorF? Dark;
        /// <summary>셋업 포즈에서 보이는 첨부물 이름(없으면 null).</summary>
        public string Attachment;
        public BlendMode Blend = BlendMode.Normal;
    }
}
