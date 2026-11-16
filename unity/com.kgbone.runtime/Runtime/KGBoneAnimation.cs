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
        readonly List<FiredEvent> pending = new List<FiredEvent>();
        Material[] materialArray = new Material[0];
        // Start 전에 Play·Stop을 직접 불렀으면 자동 재생하지 않는다
        bool controlled;
        // 마지막으로 그릴 때의 입력. 에디터 모드에서 스크립트가 바꾼 것을 알아채 다시 그리는 데 쓴다
        KGBoneSkeletonData drawnData;
        string drawnSkin, drawnAnimationName;
        bool drawnFlipX, drawnFlipY;

        void Awake()
        {
            ApplySorting();
            Refresh();
        }

        // 자동 재생은 Start에서 한다. 코드로 만들 때(AddComponent 뒤에 데이터를 넣음) Awake는 데이터가 들어오기 전에 불리기 때문이다
        void Start()
        {
            if (Application.isPlaying && playOnAwake && !controlled && !string.IsNullOrEmpty(animationName)) Play(animationName, loop);
        }

        void OnEnable()
        {
            Refresh();
#if UNITY_EDITOR
            UnityEditor.EditorApplication.update -= EditorTick;
            UnityEditor.EditorApplication.update += EditorTick;
#endif
        }

        // 꺼질 때(도메인 다시 불러오기·스크립트 컴파일 포함) 메시를 지운다. OnDestroy는 도메인 다시 불러오기 때 불리지 않아
        // DontSave 메시가 참조를 잃고 쌓이기 때문이다. 켜지면 OnEnable에서 다시 만든다.
        void OnDisable()
        {
#if UNITY_EDITOR
            UnityEditor.EditorApplication.update -= EditorTick;
#endif
            DestroyMesh();
        }

#if UNITY_EDITOR
        // 에디터 모드에서는 Update가 계산하지 않으므로, 스크립트가 데이터·스킨·애니메이션·뒤집기를 바꿨으면 여기서 다시 그린다
        void EditorTick()
        {
            if (this == null || Application.isPlaying || !IsStale()) return;
            Refresh();
            UnityEditor.SceneView.RepaintAll();
        }
#endif

        bool IsStale()
        {
            if (skeletonData != drawnData) return true;
            if (skeletonData == null) return false;
            return skin != drawnSkin || animationName != drawnAnimationName || flipX != drawnFlipX || flipY != drawnFlipY
                || (poser != null && poser.Model != skeletonData.Model);
        }

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

        void OnDestroy() => DestroyMesh();

        void DestroyMesh()
        {
            if (mesh == null) return;
            var filter = GetComponent<MeshFilter>();
            if (filter != null && filter.sharedMesh == mesh) filter.sharedMesh = null;
            if (Application.isPlaying) Destroy(mesh);
            else DestroyImmediate(mesh);
            mesh = null;
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
            controlled = true;
            CurrentAnimation = animation;
            this.loop = loop;
            Time = 0;
            IsPlaying = true;
            Refresh();
        }

        /// <summary>재생을 멈추고 셋업 포즈로.</summary>
        public void Stop()
        {
            controlled = true;
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
            float delta = dt * timeScale;
            // 시간을 거꾸로 돌릴 때는 이벤트를 내지 않는다(구간 규칙이 앞으로 가는 시간만 전제한다)
            bool fireEvents = IsPlaying && delta >= 0;
            bool reachedEnd = false;
            AnimationData anim = null;
            if (IsPlaying)
            {
                Time += delta;
                anim = p.Model.FindAnimation(CurrentAnimation);
                if (!loop && anim != null)
                {
                    if (Time >= anim.Duration)
                    {
                        Time = anim.Duration;
                        IsPlaying = false;
                        reachedEnd = true;
                    }
                    else if (Time < 0) Time = 0;
                }
            }
            Compute(fireEvents ? last : (float?)null);

            // 처리기가 Play·Stop을 불러 포즈가 다시 계산돼도 안전하도록 이벤트를 따로 담아 두고 내보낸다
            pending.Clear();
            if (fireEvents)
            {
                pending.AddRange(LastPose.Events);
                // 반복하지 않는 애니메이션의 끝 시간 키(길이 0이면 0초 키)는 [이전, 지금) 구간에 들지 않으므로 끝에 도달할 때 보탠다
                if (reachedEnd) EventFinder.KeysAt(p.Model, anim, anim.Duration, pending);
            }
            for (int i = 0; i < pending.Count && EventFired != null; i++)
            {
                var e = pending[i];
                EventFired(new KGBoneEvent(e.Name, e.Time, e.Int, e.Float, e.String, e.Volume, e.Balance));
            }
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

        /// <summary>데이터 에셋을 다시 가져왔을 때 가져오기 도구(에디터)가 부른다. 에디터 모드에서는 Update가 계산하지 않기 때문이다.</summary>
        internal void OnDataReimported()
        {
            if (isActiveAndEnabled) Refresh();
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
            // 같은 데이터 에셋이라도 다시 가져오면 모델이 바뀐다(객체는 그대로일 수 있음)
            if (poser == null || poserData != skeletonData || poser.Model != skeletonData.Model)
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
                drawnData = null;
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
            drawnData = skeletonData;
            drawnSkin = skin;
            drawnAnimationName = animationName;
            drawnFlipX = flipX;
            drawnFlipY = flipY;
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
