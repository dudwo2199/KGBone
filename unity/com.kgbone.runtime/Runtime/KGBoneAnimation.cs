// KGBone 캐릭터 재생 컴포넌트: 가져온 .kgbone(KGBoneSkeletonData)을 매 프레임 계산해 Mesh 하나로 그린다.
// 에디터에서 재생하지 않아도 고른 애니메이션의 0초(없으면 셋업 포즈)가 씬 뷰에 보인다.
using System;
using System.Collections.Generic;
using KGBone.Core;
using UnityEngine;

namespace KGBone
{
    [ExecuteAlways, DisallowMultipleComponent, RequireComponent(typeof(MeshFilter), typeof(MeshRenderer))]
    public sealed class KGBoneAnimation : MonoBehaviour
    {
        public KGBoneSkeletonData skeletonData;
        [Tooltip("켤 스킨(비우면 기본 스킨)")]
        public string skin;
        [Tooltip("시작할 애니메이션(비우면 셋업 포즈)")]
        public string animationName;
        public bool loop = true;
        public float timeScale = 1;
        public bool playOnAwake = true;
        public bool flipX, flipY;
        [Tooltip("MeshRenderer는 인스펙터에 Sorting Layer를 보여 주지 않으므로 여기서 정한다")]
        public string sortingLayerName = "Default";
        public int sortingOrder;

        /// <summary>애니메이션 이벤트가 날 때(키 하나에 한 번, 반복이면 바퀴마다).</summary>
        public event Action<KGBoneEvent> EventFired;
        /// <summary>반복하지 않는 애니메이션이 끝에 도달했을 때(한 번).</summary>
        public event Action Completed;

        /// <summary>현재 시간(초). 반복 재생이면 바퀴를 더한 누적 시간.</summary>
        public float Time { get; set; }
        public bool IsPlaying { get; private set; }
        public string CurrentAnimation { get; private set; }
        /// <summary>마지막으로 계산한 포즈(KGBone 단위, 뒤집기 전).</summary>
        public PoseResult LastPose { get; private set; }

        SkeletonPoser poser;
        KGBoneSkeletonData poserData;
        Mesh mesh;
        readonly MeshBuilder builder = new MeshBuilder();
        readonly List<Material> materials = new List<Material>();
        readonly PoseRequest request = new PoseRequest();
        Material[] materialArray = new Material[0];

        void Awake()
        {
            ApplySorting();
            if (Application.isPlaying && playOnAwake && !string.IsNullOrEmpty(animationName)) Play(animationName, loop);
            else Refresh();
        }

        void OnEnable() => Refresh();

        void OnValidate()
        {
            ApplySorting();
            poser = null;
            if (isActiveAndEnabled) Refresh();
        }

        void Update()
        {
            if (Application.isPlaying) Advance(UnityEngine.Time.deltaTime);
        }

        void OnDestroy()
        {
            if (mesh != null)
            {
                if (Application.isPlaying) Destroy(mesh);
                else DestroyImmediate(mesh);
            }
        }

        /// <summary>애니메이션을 처음부터 재생한다. null이면 Stop(). 없는 이름이면 경고만 하고 지금 상태를 유지한다.</summary>
        public void Play(string animation, bool loop)
        {
            if (animation == null)
            {
                Stop();
                return;
            }
            var p = EnsurePoser();
            if (p == null || p.Model.FindAnimation(animation) == null)
            {
                Debug.LogWarning($"애니메이션을 찾을 수 없습니다: {animation}", this);
                return;
            }
            CurrentAnimation = animation;
            this.loop = loop;
            Time = 0;
            IsPlaying = true;
            Refresh();
        }

        /// <summary>재생을 멈추고 셋업 포즈로.</summary>
        public void Stop()
        {
            CurrentAnimation = null;
            IsPlaying = false;
            Time = 0;
            Refresh();
        }

        /// <summary>dt초만큼 진행하고 다시 그린다(Update가 부른다. 테스트·직접 제어용으로 열어 둔다).</summary>
        public void Advance(float dt)
        {
            var p = EnsurePoser();
            if (p == null)
            {
                ClearMesh();
                return;
            }
            float last = Time;
            bool fireEvents = IsPlaying;
            bool reachedEnd = false;
            if (IsPlaying)
            {
                Time += dt * timeScale;
                var anim = p.Model.FindAnimation(CurrentAnimation);
                if (!loop && anim != null && Time >= anim.Duration)
                {
                    Time = anim.Duration;
                    IsPlaying = false;
                    reachedEnd = true;
                }
            }
            Compute(fireEvents ? last : (float?)null);
            if (fireEvents && EventFired != null)
                foreach (var e in LastPose.Events) EventFired(new KGBoneEvent(e.Name, e.Time, e.Int, e.Float, e.String, e.Volume, e.Balance));
            if (reachedEnd) Completed?.Invoke();
        }

        /// <summary>
        /// 본의 현재 위치(유니티 월드 좌표), 회전(도, 본 X축 방향), 크기. 데이터가 없거나 없는 본이면 false.
        /// </summary>
        public bool TryGetBone(string name, out Vector3 position, out float rotation, out Vector2 scale)
        {
            position = default;
            rotation = 0;
            scale = default;
            var p = EnsurePoser();
            if (p == null) return false;
            int i = p.BoneIndex(name);
            if (i < 0) return false;
            if (LastPose == null) Compute(null);
            var w = LastPose.Bones[i];
            float fx = flipX ? -1f : 1f, fy = flipY ? -1f : 1f, ppu = skeletonData.PixelsPerUnit;
            position = transform.TransformPoint(new Vector3(w.X * fx / ppu, w.Y * fy / ppu, 0));
            rotation = Mathf.Atan2(w.C * fy, w.A * fx) * Mathf.Rad2Deg + transform.eulerAngles.z;
            scale = new Vector2(Mathf.Sqrt(w.A * w.A + w.C * w.C), Mathf.Sqrt(w.B * w.B + w.D * w.D));
            return true;
        }

        SkeletonPoser EnsurePoser()
        {
            if (skeletonData == null)
            {
                poser = null;
                poserData = null;
                LastPose = null;
                return null;
            }
            if (poser == null || poserData != skeletonData)
            {
                poser = new SkeletonPoser(skeletonData.Model);
                poserData = skeletonData;
                LastPose = null;
            }
            return poser;
        }

        void Refresh()
        {
            if (EnsurePoser() == null)
            {
                ClearMesh();
                return;
            }
            Compute(null);
        }

        void Compute(float? lastTime)
        {
            // 에디터에서 재생 중이 아니면 고른 애니메이션의 0초를 미리 보여 준다
            string anim = CurrentAnimation ?? (Application.isPlaying || string.IsNullOrEmpty(animationName) ? null : animationName);
            request.Animation = anim;
            request.Time = Time;
            request.LastTime = lastTime;
            request.Skin = string.IsNullOrEmpty(skin) ? null : skin;
            request.Looping = loop;
            LastPose = poser.Compute(request);
            builder.Build(LastPose, skeletonData, flipX, flipY, EnsureMesh(), materials);
            if (materialArray.Length != materials.Count) materialArray = new Material[materials.Count];
            materials.CopyTo(materialArray);
            GetComponent<MeshRenderer>().sharedMaterials = materialArray;
        }

        Mesh EnsureMesh()
        {
            if (mesh == null)
            {
                mesh = new Mesh { name = "KGBone", hideFlags = HideFlags.DontSave };
                mesh.MarkDynamic();
                GetComponent<MeshFilter>().sharedMesh = mesh;
            }
            return mesh;
        }

        void ClearMesh()
        {
            if (mesh != null) mesh.Clear();
        }

        void ApplySorting()
        {
            var r = GetComponent<MeshRenderer>();
            if (r == null) return;
            r.sortingLayerName = sortingLayerName;
            r.sortingOrder = sortingOrder;
        }
    }
}
