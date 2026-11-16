// 이 GameObject가 KGBone 캐릭터의 본 하나를 따라가게 한다(이펙트·무기 붙이기). 매 프레임 LateUpdate에서 맞춘다.
using UnityEngine;

namespace KGBone
{
    [ExecuteAlways, DisallowMultipleComponent]
    public sealed class KGBoneBoneFollower : MonoBehaviour
    {
        public KGBoneAnimation target;
        public string boneName;
        [Tooltip("켜면 본의 회전도 따라간다(꺼 두면 위치만)")]
        public bool followRotation;

        void LateUpdate() => Apply();

        /// <summary>지금 본 위치로 옮긴다(LateUpdate가 부른다).</summary>
        public void Apply()
        {
            if (target == null || string.IsNullOrEmpty(boneName)) return;
            if (!target.TryGetBone(boneName, out Vector3 position, out float rotation, out _)) return;
            transform.position = position;
            if (followRotation) transform.rotation = Quaternion.Euler(0, 0, rotation);
        }
    }
}
