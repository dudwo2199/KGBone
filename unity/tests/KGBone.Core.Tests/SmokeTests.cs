// M01: 계산부가 유니티 없이 컴파일·실행되는지 확인하는 가장 작은 테스트.
using KGBone.Core;
using NUnit.Framework;

namespace KGBone.Tests
{
    public class SmokeTests
    {
        [Test]
        public void m01_world_bone_identity_maps_point()
        {
            var (x, y) = WorldBone.Identity.LocalToWorld(3, 4);
            Assert.AreEqual(3f, x);
            Assert.AreEqual(4f, y);
        }
    }
}
