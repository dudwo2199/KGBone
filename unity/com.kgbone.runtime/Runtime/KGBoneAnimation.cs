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
        [Range(0, 1), Tooltip("물리 제약: GameObject가 이동한 만큼 흔들림에 반영할 비율(0이면 에디터와 같음)")]
        public float physicsMoveFactor;
        [Range(0, 1), Tooltip("물리 제약: GameObject가 회전한 만큼 흔들림에 반영할 비율(0이면 에디터와 같음)")]
        public float physicsRotateFactor;
        [Min(0), Tooltip("트랙 애니메이션을 바꿀 때 섞는 기본 시간(초). 0이면 바로 바뀜")]
        public float defaultMix;

        /// <summary>애니메이션 이벤트가 날 때(키 하나에 한 번, 반복이면 바퀴마다).</summary>
        public event Action<KGBoneEvent> EventFired;
        /// <summary>트랙 0에서 반복하지 않는 애니메이션이 끝에 도달했을 때(한 번).</summary>
        public event Action Completed;
        /// <summary>어느 트랙이든 반복하지 않는 애니메이션이 끝에 도달했을 때(트랙 번호, 애니메이션 이름).</summary>
        public event Action<int, string> TrackCompleted;

        /// <summary>트랙 0의 현재 시간(초). 반복 재생이면 바퀴를 더한 누적 시간.</summary>
        public float Time
        {
            get => Track0?.Time ?? idleTime;
            set
            {
                var e = Track0;
                if (e != null) e.Time = value;
                else idleTime = value;
            }
        }
        /// <summary>트랙 0이 재생 중인가(반복 아닌 애니메이션이 끝에 닿으면 false).</summary>
        public bool IsPlaying => Track0 != null && !Track0.IsComplete;
        /// <summary>트랙 0의 애니메이션 이름.</summary>
        public string CurrentAnimation => Track0?.Name;
        /// <summary>마지막으로 계산한 포즈(KGBone 단위, 뒤집기 전).</summary>
        public PoseResult LastPose { get; private set; }

        SkeletonPoser poser;
        AnimationTracks tracks;
        // 트랙·덮어쓰기를 만든 모델(OnValidate가 poser만 버릴 때는 그대로 둔다)
        SkeletonModel tracksModel;
        float idleTime;
        string observedName;
        readonly List<TrackCompletion> completions = new List<TrackCompletion>();
        readonly HashSet<string> warned = new HashSet<string>();
        PoseOverrides overrides = new PoseOverrides();
        readonly Dictionary<int, (Vector3 position, float mix)> worldPositions = new Dictionary<int, (Vector3, float)>();
        readonly Dictionary<int, (float degrees, float mix)> worldRotations = new Dictionary<int, (float, float)>();
        string[] activeSkins;
        string activeSkinsKey;
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
        // 물리: 지난 프레임의 GameObject 변환(월드 움직임 반영용)과 마지막으로 계산한 스킨(바뀌면 물리를 초기화)
        Matrix4x4 lastMatrix;
        bool hasLastMatrix;
        string physicsSkin;

        void Awake()
        {
            ApplySorting();
            Refresh();
        }

        // 자동 재생은 Start에서 한다. 코드로 만들 때(AddComponent 뒤에 데이터를 넣음) Awake는 데이터가 들어오기 전에 불리기 때문이다
        void Start()
        {
            if (Application.isPlaying && playOnAwake && !controlled && !string.IsNullOrEmpty(animationName)) Play(animationName, loop);
            observedName = animationName;
        }

        TrackEntry Track0 => tracks?.Current(0);

        void OnEnable()
        {
            ResetPhysics();
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
            this.loop = loop;
            tracks.SetAnimation(0, animation, loop, 0);
            observedName = animationName;
            Refresh();
        }

        /// <summary>재생을 멈추고(모든 트랙 비움) 셋업 포즈로.</summary>
        public void Stop()
        {
            controlled = true;
            tracks?.ClearAll();
            idleTime = 0;
            Refresh();
        }

        // ---------- 트랙 ----------

        /// <summary>
        /// 트랙에 애니메이션을 놓는다(처음부터). mix초 동안 지금 애니메이션과 섞어 넘어가며, mix &lt; 0이면 defaultMix.
        /// 윗 트랙은 아랫 트랙 결과 위에 자기 키가 있는 값만 덮는다. 같은 이름으로 다시 부르면 처음부터 다시 시작한다.
        /// 없는 이름이면 경고 한 번 뒤 null(트랙 그대로).
        /// </summary>
        public TrackEntry SetAnimation(int track, string name, bool loop, float mix = -1)
        {
            if (EnsurePoser() == null) return null;
            controlled = true;
            var e = tracks.SetAnimation(track, name, loop, mix < 0 ? defaultMix : mix);
            if (e == null)
            {
                WarnMissing(name);
                return null;
            }
            if (track == 0) this.loop = loop;
            Refresh();
            return e;
        }

        /// <summary>지금 애니메이션이 끝나면(반복이면 이번 바퀴 끝) delay초 뒤 시작하도록 예약한다. mix &lt; 0이면 defaultMix.</summary>
        public TrackEntry AddAnimation(int track, string name, bool loop, float delay = 0, float mix = -1)
        {
            if (EnsurePoser() == null) return null;
            controlled = true;
            var e = tracks.AddAnimation(track, name, loop, delay, mix < 0 ? defaultMix : mix);
            if (e == null) WarnMissing(name);
            return e;
        }

        /// <summary>트랙을 mix초 동안 서서히 빼고 비운다(mix &lt; 0이면 defaultMix, 0이면 바로).</summary>
        public void ClearTrack(int track, float mix = -1)
        {
            if (tracks == null) return;
            tracks.ClearTrack(track, mix < 0 ? defaultMix : mix);
            if (isActiveAndEnabled) Refresh();
        }

        /// <summary>지금 애니메이션이 끝나면 delay초 뒤 트랙을 mix초 동안 비우도록 예약한다.</summary>
        public void AddEmpty(int track, float delay = 0, float mix = -1)
        {
            if (EnsurePoser() == null) return;
            tracks.AddEmpty(track, delay, mix < 0 ? defaultMix : mix);
        }

        /// <summary>트랙 비율(0~1). 1보다 작으면 아랫 트랙 결과와 그 비율로 섞인다.</summary>
        public void SetTrackAlpha(int track, float alpha)
        {
            if (EnsurePoser() == null) return;
            tracks.SetAlpha(track, alpha);
        }

        /// <summary>트랙의 지금 애니메이션 이름(비어 있거나 비우는 중이면 null).</summary>
        public string GetTrackAnimation(int track) => tracks?.Current(track)?.Name;

        void WarnMissing(string name) => Warn($"애니메이션을 찾을 수 없습니다: {name}");

        // ---------- 첨부물·스킨·본 덮어쓰기 ----------

        /// <summary>슬롯 첨부물을 바꾼다(애니메이션 키보다 우선, ClearAttachment까지 유지). null이면 그 슬롯을 숨긴다.</summary>
        public void SetAttachment(string slot, string attachment)
        {
            var p = EnsurePoser();
            if (p == null) return;
            int si = p.Model.SlotIndex(slot);
            if (si < 0)
            {
                Warn("슬롯을 찾을 수 없습니다: " + slot);
                return;
            }
            if (attachment != null && !AttachmentExists(p.Model, slot, attachment))
            {
                Warn("첨부물을 찾을 수 없습니다: " + slot + "/" + attachment);
                return;
            }
            overrides.SetAttachment(si, attachment);
            Refresh();
        }

        /// <summary>SetAttachment를 풀고 애니메이션·셋업 첨부물로 돌아간다.</summary>
        public void ClearAttachment(string slot)
        {
            var p = EnsurePoser();
            if (p == null) return;
            int si = p.Model.SlotIndex(slot);
            if (si < 0) return;
            overrides.ClearAttachment(si);
            Refresh();
        }

        static bool AttachmentExists(SkeletonModel m, string slot, string name)
        {
            foreach (var s in m.Skins) if (s.Get(slot, name) != null) return true;
            return false;
        }

        /// <summary>
        /// 스킨 여러 개를 함께 켠다(옷 + 무기). 같은 슬롯·이름의 첨부물은 뒤에 적은 스킨이 이긴다. 기본 스킨은 항상 포함.
        /// skin 필드는 첫 값이 되고, 필드를 직접 다른 값으로 바꾸면 그 스킨 하나로 돌아간다. 스킨을 바꾸면 물리를 초기화한다.
        /// </summary>
        public void SetSkins(params string[] skins)
        {
            var p = EnsurePoser();
            if (p == null) return;
            skins = skins ?? new string[0];
            foreach (var n in skins)
                if (!string.IsNullOrEmpty(n) && p.Model.FindSkin(n) == null)
                {
                    Warn("스킨을 찾을 수 없습니다: " + n);
                    return;
                }
            activeSkins = (string[])skins.Clone();
            activeSkinsKey = string.Join("\n", activeSkins);
            skin = activeSkins.Length > 0 ? activeSkins[0] : null;
            Refresh();
        }

        /// <summary>본의 로컬 값(부모 기준, 에디터 속성 패널과 같은 값) 중 정한 것만 정한다. Mix로 애니메이션 값과 섞는다. ClearBone까지 유지.</summary>
        public void SetBoneLocal(string bone, BoneOverride value)
        {
            int i = BoneOrWarn(bone);
            if (i < 0) return;
            overrides.SetBoneLocal(i, value);
            Refresh();
        }

        /// <summary>본 원점을 유니티 월드 좌표의 점으로 옮긴다(예: IK 대상 본을 마우스 위치로). 제약보다 먼저 적용된다. 매 계산 이 월드 점을 따른다.</summary>
        public void SetBoneWorldPosition(string bone, Vector3 position, float mix = 1)
        {
            int i = BoneOrWarn(bone);
            if (i < 0) return;
            worldPositions[i] = (position, mix);
            Refresh();
        }

        /// <summary>본 X축이 유니티 월드에서 degrees(도) 방향을 보게 한다(예: 마우스 쪽으로 고개). TryGetBone의 회전과 같은 기준.</summary>
        public void SetBoneWorldRotation(string bone, float degrees, float mix = 1)
        {
            int i = BoneOrWarn(bone);
            if (i < 0) return;
            worldRotations[i] = (degrees, mix);
            Refresh();
        }

        public void ClearBone(string bone)
        {
            var p = EnsurePoser();
            if (p == null) return;
            int i = p.BoneIndex(bone);
            if (i < 0) return;
            overrides.ClearBone(i);
            worldPositions.Remove(i);
            worldRotations.Remove(i);
            Refresh();
        }

        public void ClearAllBones()
        {
            overrides.ClearAllBones();
            worldPositions.Clear();
            worldRotations.Clear();
            if (isActiveAndEnabled) Refresh();
        }

        int BoneOrWarn(string bone)
        {
            var p = EnsurePoser();
            if (p == null) return -1;
            int i = p.BoneIndex(bone);
            if (i < 0) Warn("본을 찾을 수 없습니다: " + bone);
            return i;
        }

        void Warn(string message)
        {
            if (warned.Add(message)) Debug.LogWarning(message, this);
        }

        /// <summary>유니티 월드 점 → 뼈대 좌표(TryGetBone의 반대: GameObject 변환, 뒤집기, pixelsPerUnit).</summary>
        Vector2 ToSkeleton(Vector3 world)
        {
            var l = transform.InverseTransformPoint(world);
            float fx = flipX ? -1f : 1f, fy = flipY ? -1f : 1f, ppu = skeletonData.PixelsPerUnit;
            return new Vector2(l.x * ppu * fx, l.y * ppu * fy);
        }

        /// <summary>유니티 월드 각도(도) → 뼈대 좌표 각도(TryGetBone 회전의 반대).</summary>
        float ToSkeletonAngle(float degrees)
        {
            float fx = flipX ? -1f : 1f, fy = flipY ? -1f : 1f;
            float phi = (degrees - transform.eulerAngles.z) * Mathf.Deg2Rad;
            return Mathf.Atan2(Mathf.Sin(phi) * fy, Mathf.Cos(phi) * fx) * Mathf.Rad2Deg;
        }


        /// <summary>물리 한 프레임 진행 시간: 0~0.1초(에디터 재생과 같은 상한). 컴포넌트 timeScale은 곱하지 않는다.</summary>
        internal static float PhysicsDt(float dt) => Mathf.Min(0.1f, Mathf.Max(dt, 0f));

        /// <summary>물리 흔들림을 처음 상태로(순간이동 뒤 튀지 않게). 지난 위치 기록도 지금 위치로 맞춘다.</summary>
        public void ResetPhysics()
        {
            poser?.ResetPhysics();
            hasLastMatrix = false;
        }

        /// <summary>지난 프레임 대비 GameObject 움직임을 뼈대 좌표로 바꿔 반영 비율을 곱한다(반영 0이면 0).</summary>
        WorldMotion TakeWorldMotion()
        {
            var cur = transform.localToWorldMatrix;
            var prev = hasLastMatrix ? lastMatrix : cur;
            lastMatrix = cur;
            hasLastMatrix = true;
            if (physicsMoveFactor <= 0f && physicsRotateFactor <= 0f) return default;
            // 지난 프레임 좌표계에서 본 이번 프레임 좌표계: 유니티 로컬 단위 → 뼈대 단위(픽셀, 뒤집기)
            var d = prev.inverse * cur;
            float fx = flipX ? -1f : 1f, fy = flipY ? -1f : 1f, ppu = skeletonData.PixelsPerUnit;
            // 뼈대 점 s → 로컬 S·s(S = 뒤집기/ppu) → d → S⁻¹. 회전은 그 2x2의 첫 열 각도, 평행 이동은 S⁻¹·d.t(크기 변화는 무시)
            float a = d.m00, c = d.m10 * fx / fy;
            float tx = d.m03 * ppu * fx, ty = d.m13 * ppu * fy;
            float rotation = Mathf.Atan2(c, a) * Mathf.Rad2Deg;
            return new WorldMotion { X = tx * physicsMoveFactor, Y = ty * physicsMoveFactor, Rotation = rotation * physicsRotateFactor };
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
            // 물리는 플레이 모드에서 컴포넌트가 켜져 있으면 재생 여부와 상관없이 진행한다(멈춰도 흔들림이 가라앉는다)
            if (Application.isPlaying)
            {
                request.Physics = PhysicsStep.Advance(PhysicsDt(dt));
                request.WorldMotion = TakeWorldMotion();
            }
            // 실행 중 animationName 필드를 바꾸면 트랙 0을 defaultMix로 바꾼다(재생 중일 때만)
            if (observedName != animationName)
            {
                observedName = animationName;
                if (Track0 != null && !string.IsNullOrEmpty(animationName)
                    && tracks.SetAnimation(0, animationName, loop, defaultMix) == null) WarnMissing(animationName);
            }
            var t0 = Track0;
            if (t0 != null && t0.Loop != loop) t0.Loop = loop;
            float delta = dt * timeScale;
            // 반복 아닌 애니메이션은 끝에서 멈추고, 끝 시간 키(길이 0이면 0초 키)도 끝에 도달할 때 낸다(AnimationTracks)
            completions.Clear();
            tracks.Update(delta, completions);
            // 시간을 거꾸로 돌릴 때는 이벤트를 내지 않는다(구간 규칙이 앞으로 가는 시간만 전제한다)
            Compute(delta >= 0 ? 0f : (float?)null);
            request.Physics = PhysicsStep.None;
            request.WorldMotion = default;

            // 처리기가 Play·Stop을 불러 포즈가 다시 계산돼도 안전하도록 이벤트를 따로 담아 두고 내보낸다
            pending.Clear();
            if (delta >= 0) pending.AddRange(LastPose.Events);
            for (int i = 0; i < pending.Count && EventFired != null; i++)
            {
                var e = pending[i];
                EventFired(new KGBoneEvent(e.Name, e.Time, e.Int, e.Float, e.String, e.Volume, e.Balance, e.Track, e.Animation));
            }
            for (int i = 0; i < completions.Count; i++)
            {
                var c = completions[i];
                TrackCompleted?.Invoke(c.Track, c.Name);
                if (c.Track == 0) Completed?.Invoke();
            }
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
            ResetPhysics();
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
                if (tracksModel != skeletonData.Model)
                {
                    // 모델이 바뀌면(다시 가져오기) 트랙은 새 모델로 다시 만들고 트랙 0의 애니메이션·시간만 이어 간다
                    var old0 = Track0;
                    tracks = new AnimationTracks(skeletonData.Model);
                    tracksModel = skeletonData.Model;
                    if (old0 != null)
                    {
                        var e = tracks.SetAnimation(0, old0.Name, old0.Loop, 0);
                        if (e != null) e.Time = old0.Time;
                    }
                    // 덮어쓰기는 본·슬롯 번호로 들고 있으므로 비운다
                    overrides = new PoseOverrides();
                    worldPositions.Clear();
                    worldRotations.Clear();
                }
                poser = new SkeletonPoser(skeletonData.Model);
                poserData = skeletonData;
                LastPose = null;
                hasLastMatrix = false;
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
            // 트랙을 한 번이라도 썼으면(Stop 전까지) 트랙 경로: 같은 프레임에 모든 트랙이 비어도 그 프레임 이벤트를 낸다
            if (tracks.TrackCount > 0)
            {
                request.Tracks = tracks;
                request.Animation = null;
                request.Time = 0;
            }
            else
            {
                // 에디터에서 재생 중이 아니면 고른 애니메이션의 0초(또는 Time)를 미리 보여 준다
                request.Tracks = null;
                request.Animation = Application.isPlaying || string.IsNullOrEmpty(animationName) ? null : animationName;
                request.Time = idleTime;
            }
            request.LastTime = lastTime;
            request.Skin = string.IsNullOrEmpty(skin) ? null : skin;
            // skin 필드를 직접 다른 값으로 바꾸면 SetSkins로 합친 상태를 풀고 그 스킨 하나로
            if (activeSkins != null && (activeSkins.Length > 0 && !string.IsNullOrEmpty(activeSkins[0]) ? activeSkins[0] : null) != request.Skin)
            {
                activeSkins = null;
                activeSkinsKey = null;
            }
            request.Skins = activeSkins;
            string skinKey = activeSkins != null ? activeSkinsKey : request.Skin;
            if (skinKey != physicsSkin)
            {
                // 스킨이 바뀌면 켜지는 본·제약이 달라지므로 물리를 처음부터
                poser.ResetPhysics();
                physicsSkin = skinKey;
            }
            // 월드 덮어쓰기는 유니티 월드 값으로 들고 있다가 매 계산 지금 GameObject 기준 뼈대 좌표로 바꾼다
            foreach (var kv in worldPositions)
            {
                var s = ToSkeleton(kv.Value.position);
                overrides.SetBoneWorldPosition(kv.Key, s.x, s.y, kv.Value.mix);
            }
            foreach (var kv in worldRotations) overrides.SetBoneWorldRotation(kv.Key, ToSkeletonAngle(kv.Value.degrees), kv.Value.mix);
            request.Overrides = overrides;
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
