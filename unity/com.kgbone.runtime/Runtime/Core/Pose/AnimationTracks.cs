// 번호 트랙 재생(유니티 런타임 전용, 핵심에는 없음 — 설계 docs/superpowers/specs/2026-11-18-unity-runtime-stage3-design.md 5.3).
// - 트랙 0부터 위로 차례로 적용한다. 트랙 n 적용 전 포즈를 R이라 하면:
//   섞는 중이 아니면 R에 지금 항목을 비율 Alpha로 한 번 적용(트랙 하나·섞기 없음이면 2단계와 같은 경로).
//   섞는 중이면 X = R + 이전 항목(또는 얼린 포즈, 또는 R), Y = R + 지금 항목(비우는 중이면 R), 결과 = X·Y를 t = 지난 섞기 / 섞는 시간으로 섞은 것.
// - 섞는 도중 또 바꾸면 그 순간의 트랙 결과를 "얼린 포즈"로 만들어 X로 쓴다(다음 Apply에서 계산). 얼린 포즈는 키가 있던 값만 쓰고 나머지는 R을 따른다.
// - 시간은 Update(dt)로 진행한다. 반복 아닌 항목은 끝에서 멈추고 끝남을 한 번 알린다. 예약은 지금 항목 끝(반복이면 이번 바퀴 끝) + delay에 시작한다.
// - 이벤트는 지금 항목만 [이전, 지금) 구간으로 내고(섞이며 빠지는 항목은 내지 않음), 반복 아닌 항목이 끝에 닿으면 끝 시간 키도 보탠다(G-27 규칙).
using System;
using System.Collections.Generic;

namespace KGBone.Core
{
    public sealed class TrackEntry
    {
        /// <summary>애니메이션 이름(비우기 항목이면 null).</summary>
        public string Name;
        public bool Loop;
        /// <summary>애니메이션 시간(초, 반복이면 바퀴를 더한 누적 시간).</summary>
        public float Time;
        /// <summary>이 항목으로 섞어 들어오는 시간(초).</summary>
        public float MixDuration;
        /// <summary>예약 항목: 앞 항목 끝에서 기다리는 시간(초).</summary>
        public float Delay;
        internal AnimationData Data;
        internal float LastTime, MixTime, Elapsed;
        internal bool Completed;
        internal bool IsEmpty => Data == null;
    }

    public readonly struct TrackCompletion
    {
        public readonly int Track;
        public readonly string Name;
        public TrackCompletion(int track, string name) { Track = track; Name = name; }
    }

    public sealed class AnimationTracks
    {
        sealed class Track
        {
            public TrackEntry Cur, From;
            public float Alpha = 1f;
            public readonly List<TrackEntry> Queue = new List<TrackEntry>();
            public float Trigger = float.NaN;
            // 얼린 포즈
            public bool FrozenOn;
            public PoseState Frozen;
            public KeyedMask FrozenMask;
            // 다음 Apply에서 얼릴 것(바꾼 순간의 이전 항목·지금 항목·섞기 비율)
            public bool PendingOn, PendingFrozen;
            public TrackEntry PendingFrom, PendingCur;
            public float PendingT;
        }

        readonly SkeletonModel model;
        readonly List<Track> tracks = new List<Track>();
        readonly List<FiredEvent> events = new List<FiredEvent>();
        readonly Dictionary<AnimationData, KeyedMask> masks = new Dictionary<AnimationData, KeyedMask>();
        PoseState x, y, z;
        KeyedMask tmpMask;
        string skin;

        public AnimationTracks(SkeletonModel model) { this.model = model; }

        public int TrackCount => tracks.Count;

        Track Get(int track)
        {
            while (tracks.Count <= track) tracks.Add(new Track());
            return tracks[track];
        }

        Track Find(int track) => track >= 0 && track < tracks.Count ? tracks[track] : null;

        static bool Mixing(Track tr) => tr.Cur != null && tr.Cur.MixDuration > 0f && tr.Cur.MixTime < tr.Cur.MixDuration;

        static float MixT(TrackEntry e) => e.MixDuration > 0f ? Math.Min(e.MixTime / e.MixDuration, 1f) : 1f;

        // ---------- 공개 조작 ----------

        /// <summary>트랙의 지금 항목을 바꾼다(mix초 동안 섞기, 0이면 바로). 예약은 비운다. 없는 애니메이션이면 null, 트랙 그대로.</summary>
        public TrackEntry SetAnimation(int track, string name, bool loop, float mix)
        {
            var data = track >= 0 ? model.FindAnimation(name) : null;
            if (data == null) return null;
            var tr = Get(track);
            tr.Queue.Clear();
            tr.Trigger = float.NaN;
            var e = new TrackEntry { Name = name, Loop = loop, Data = data };
            Transition(tr, e, mix);
            return e;
        }

        /// <summary>지금 항목이 끝나면(반복이면 이번 바퀴 끝) delay초 뒤 시작할 항목을 예약한다. 트랙이 비어 있으면 delay 뒤 바로 시작.</summary>
        public TrackEntry AddAnimation(int track, string name, bool loop, float delay, float mix)
        {
            var data = track >= 0 ? model.FindAnimation(name) : null;
            if (data == null) return null;
            var e = new TrackEntry { Name = name, Loop = loop, Data = data, Delay = delay, MixDuration = mix };
            Enqueue(Get(track), e);
            return e;
        }

        /// <summary>트랙을 mix초 동안 서서히 빼고 비운다(0이면 바로). 예약도 비운다.</summary>
        public void ClearTrack(int track, float mix)
        {
            var tr = Find(track);
            if (tr == null) return;
            tr.Queue.Clear();
            tr.Trigger = float.NaN;
            if (tr.Cur == null) return;
            Transition(tr, new TrackEntry(), mix);
        }

        /// <summary>트랙 비우기를 예약한다(지금 항목 끝 + delay에 mix초 동안 빠짐).</summary>
        public void AddEmpty(int track, float delay, float mix)
        {
            if (track < 0) return;
            Enqueue(Get(track), new TrackEntry { Delay = delay, MixDuration = mix });
        }

        /// <summary>모든 트랙을 바로 비운다.</summary>
        public void ClearAll() => tracks.Clear();

        public void SetAlpha(int track, float alpha)
        {
            if (track < 0) return;
            Get(track).Alpha = Math.Min(Math.Max(alpha, 0f), 1f);
        }

        /// <summary>트랙의 지금 항목(비어 있거나 비우는 중이면 null).</summary>
        public TrackEntry Current(int track)
        {
            var tr = Find(track);
            return tr?.Cur != null && !tr.Cur.IsEmpty ? tr.Cur : null;
        }

        void Enqueue(Track tr, TrackEntry e)
        {
            tr.Queue.Add(e);
            if (tr.Queue.Count == 1) tr.Trigger = float.NaN;
        }

        void Transition(Track tr, TrackEntry e, float mix)
        {
            var old = tr.Cur;
            if (e.IsEmpty && old == null) return;
            e.MixTime = 0f;
            if (mix <= 0f)
            {
                e.MixDuration = 0f;
                tr.From = null;
                tr.FrozenOn = false;
                tr.PendingOn = false;
                tr.Cur = e.IsEmpty ? null : e;
                return;
            }
            e.MixDuration = mix;
            if (old == null)
            {
                tr.From = null;
                tr.FrozenOn = false;
                tr.PendingOn = false;
            }
            else if (Mixing(tr) || tr.PendingOn)
            {
                // 섞는 도중: 그 순간의 결과를 다음 Apply에서 얼린다(이미 얼릴 것이 있으면 그것을 그대로 쓴다)
                if (!tr.PendingOn)
                {
                    tr.PendingOn = true;
                    tr.PendingFrom = tr.From;
                    tr.PendingCur = old;
                    tr.PendingFrozen = tr.FrozenOn;
                    tr.PendingT = MixT(old);
                }
                tr.From = null;
            }
            else
            {
                tr.From = old.IsEmpty ? null : old;
                tr.FrozenOn = false;
            }
            tr.Cur = e;
        }

        // ---------- 시간 ----------

        /// <summary>dt초(timeScale을 곱한 값) 진행한다. 반복 아닌 항목이 끝에 닿으면 completed에 더한다. 이번 진행의 이벤트는 CollectEvents로 받는다.</summary>
        public void Update(float dt, List<TrackCompletion> completed)
        {
            events.Clear();
            float adv = Math.Max(dt, 0f);
            for (int i = 0; i < tracks.Count; i++)
            {
                var tr = tracks[i];
                var cur = tr.Cur;
                if (cur == null)
                {
                    if (tr.Queue.Count == 0) continue;
                    var head = tr.Queue[0];
                    head.Delay -= adv;
                    if (head.Delay > 0f) continue;
                    tr.Queue.RemoveAt(0);
                    tr.Trigger = float.NaN;
                    Start(i, tr, head, -head.Delay, dt, completed);
                    continue;
                }
                if (tr.Queue.Count > 0 && float.IsNaN(tr.Trigger)) tr.Trigger = TriggerFor(cur, tr.Queue[0]);
                Advance(i, cur, dt, completed, true);
                if (tr.From != null) Advance(i, tr.From, dt, null, false);
                if (cur.MixDuration > 0f) cur.MixTime += adv;
                if (tr.Queue.Count > 0 && cur.Elapsed >= tr.Trigger)
                {
                    var head = tr.Queue[0];
                    tr.Queue.RemoveAt(0);
                    float overflow = cur.Elapsed - tr.Trigger;
                    tr.Trigger = float.NaN;
                    Start(i, tr, head, overflow, dt, completed);
                }
                cur = tr.Cur;
                if (cur != null && cur.MixDuration > 0f && cur.MixTime >= cur.MixDuration)
                {
                    tr.From = null;
                    tr.FrozenOn = false;
                    tr.PendingOn = false;
                    if (cur.IsEmpty) tr.Cur = null;
                }
            }
        }

        static float TriggerFor(TrackEntry cur, TrackEntry head)
        {
            if (cur.IsEmpty) return cur.Elapsed + head.Delay;
            float d = cur.Data.Duration;
            if (!cur.Loop) return d + head.Delay;
            if (d <= 0f) return cur.Elapsed + head.Delay;
            return ((float)Math.Floor(cur.Elapsed / d) + 1f) * d + head.Delay;
        }

        void Start(int track, Track tr, TrackEntry e, float overflow, float dt, List<TrackCompletion> completed)
        {
            e.Time = 0f;
            e.LastTime = 0f;
            e.Elapsed = 0f;
            e.Completed = false;
            Transition(tr, e, e.MixDuration);
            if (overflow > 0f && tr.Cur == e)
            {
                Advance(track, e, dt >= 0f ? overflow : -overflow, completed, true);
                if (e.MixDuration > 0f) e.MixTime += overflow;
            }
        }

        void Advance(int track, TrackEntry e, float dt, List<TrackCompletion> completed, bool fire)
        {
            float adv = Math.Max(dt, 0f);
            e.Elapsed += adv;
            if (e.IsEmpty) return;
            e.LastTime = e.Time;
            e.Time += dt;
            float d = e.Data.Duration;
            bool reached = false;
            if (!e.Loop)
            {
                if (e.Time >= d)
                {
                    e.Time = d;
                    if (!e.Completed)
                    {
                        e.Completed = true;
                        reached = true;
                    }
                }
                else if (e.Time < 0f) e.Time = 0f;
            }
            // 시간을 거꾸로 돌릴 때는 이벤트를 내지 않는다(구간 규칙이 앞으로 가는 시간만 전제)
            if (fire && dt >= 0f)
            {
                int start = events.Count;
                EventFinder.Fired(model, e.Data, e.LastTime, e.Time, e.Loop, events);
                if (reached) EventFinder.KeysAt(model, e.Data, d, events);
                for (int k = start; k < events.Count; k++)
                {
                    var ev = events[k];
                    ev.Track = track;
                    ev.Animation = e.Name;
                    events[k] = ev;
                }
            }
            if (reached) completed?.Add(new TrackCompletion(track, e.Name));
        }

        /// <summary>마지막 Update의 이벤트(트랙 순서)를 output에 더한다.</summary>
        public void CollectEvents(List<FiredEvent> output) => output.AddRange(events);

        /// <summary>지금 항목들의 물리 reset 키가 마지막 Update 구간 [이전, 지금)에 있으면 그 물리 제약을 초기화한다.</summary>
        public void ResetPhysicsKeys(PhysicsState physics)
        {
            foreach (var tr in tracks)
            {
                var e = tr.Cur;
                if (e == null || e.IsEmpty) continue;
                foreach (var tl in e.Data.Constraints)
                    if (tl.Name == "reset" && EventFinder.AnyHit(tl.Keys, e.LastTime, e.Time, e.Loop, e.Data.Duration))
                        physics.ResetNamed(tl.Constraint == ConstraintTimeline.AllPhysics ? "" : model.Constraints[tl.Constraint].Name);
            }
        }

        // ---------- 포즈 ----------

        /// <summary>트랙을 차례로 pose(셋업에서 시작한 포즈)에 적용한다.</summary>
        public void Apply(PoseState pose, SkinSet skins)
        {
            skin = skins?.Primary;
            for (int i = 0; i < tracks.Count; i++)
            {
                var tr = tracks[i];
                if (tr.PendingOn) Freeze(tr, pose);
                if (tr.Cur == null) continue;
                if (!Mixing(tr))
                {
                    if (!tr.Cur.IsEmpty) ApplyEntry(tr.Cur, pose, tr.Alpha);
                    continue;
                }
                EnsureScratch();
                if (tr.FrozenOn) PoseBlend.Masked(model, pose, tr.Frozen, tr.FrozenMask, x);
                else
                {
                    x.CopyFrom(pose);
                    if (tr.From != null && !tr.From.IsEmpty) ApplyEntry(tr.From, x, tr.Alpha);
                }
                y.CopyFrom(pose);
                if (!tr.Cur.IsEmpty) ApplyEntry(tr.Cur, y, tr.Alpha);
                PoseBlend.Lerp(model, x, y, MixT(tr.Cur), pose);
            }
        }

        void ApplyEntry(TrackEntry e, PoseState pose, float alpha) =>
            AnimationApplier.Apply(model, e.Data, AnimationApplier.AnimationTime(e.Data.Duration, e.Time, e.Loop), skin, pose, alpha);

        void EnsureScratch()
        {
            if (x != null) return;
            x = new PoseState(model);
            y = new PoseState(model);
            z = new PoseState(model);
            tmpMask = new KeyedMask(model);
        }

        KeyedMask MaskOf(AnimationData a)
        {
            if (!masks.TryGetValue(a, out var k))
            {
                k = KeyedMask.Of(model, a);
                masks[a] = k;
            }
            return k;
        }

        /// <summary>바꾼 순간의 트랙 결과(이전 섞기)를 계산해 얼린 포즈로 둔다.</summary>
        void Freeze(Track tr, PoseState r)
        {
            EnsureScratch();
            if (tr.PendingFrozen) PoseBlend.Masked(model, r, tr.Frozen, tr.FrozenMask, x);
            else
            {
                x.CopyFrom(r);
                if (tr.PendingFrom != null && !tr.PendingFrom.IsEmpty) ApplyEntry(tr.PendingFrom, x, tr.Alpha);
            }
            y.CopyFrom(r);
            if (!tr.PendingCur.IsEmpty) ApplyEntry(tr.PendingCur, y, tr.Alpha);
            PoseBlend.Lerp(model, x, y, tr.PendingT, z);
            tmpMask.Clear();
            if (tr.PendingFrozen) tmpMask.Union(tr.FrozenMask);
            if (tr.PendingFrom != null && !tr.PendingFrom.IsEmpty) tmpMask.Union(MaskOf(tr.PendingFrom.Data));
            if (!tr.PendingCur.IsEmpty) tmpMask.Union(MaskOf(tr.PendingCur.Data));
            var spare = tr.Frozen ?? new PoseState(model);
            tr.Frozen = z;
            z = spare;
            if (tr.FrozenMask == null) tr.FrozenMask = new KeyedMask(model);
            tr.FrozenMask.Clear();
            tr.FrozenMask.Union(tmpMask);
            tr.FrozenOn = true;
            tr.PendingOn = false;
            tr.PendingFrom = tr.PendingCur = null;
        }
    }
}
