// 본의 월드 변환: 2x2 행렬 [A B; C D]와 위치 (X, Y). 로컬 점 (lx, ly) → (A·lx + B·ly + X, C·lx + D·ly + Y).
// KGBone 핵심 crates/kgbone-core/src/pose/bone_transform.rs의 WorldBone과 같다.
namespace KGBone.Core
{
    public struct WorldBone
    {
        public float A, B, C, D, X, Y;

        public static readonly WorldBone Identity = new WorldBone { A = 1, B = 0, C = 0, D = 1, X = 0, Y = 0 };

        public (float x, float y) LocalToWorld(float lx, float ly)
        {
            return (A * lx + B * ly + X, C * lx + D * ly + Y);
        }
    }
}
