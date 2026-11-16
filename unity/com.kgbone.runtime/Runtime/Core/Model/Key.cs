// 키 하나(FORMAT.md 3.7): { time, v, s, offset, curve }. 베지어는 값 채널마다 [cx1, cy1, cx2, cy2](절대 좌표).
namespace KGBone.Core
{
    public sealed class Key
    {
        public float Time;
        public float[] V = new float[0];
        public string S;
        public int Offset;
        public CurveType Curve = CurveType.Linear;
        /// <summary>Curve가 Bezier일 때의 숫자들(그 밖에는 null).</summary>
        public float[] Bezier;
    }
}
