// 게임 코드가 정하는 첨부물·본 값(유니티 런타임 전용, 핵심에는 없음 — 설계 2026-11-18 5.4). 해제할 때까지 매 계산 유지된다.
// 적용 순서(SkeletonPoser): 애니메이션·트랙 → 첨부물 → 로컬 본 → 활성 판정 → 월드 → 월드 본(본 순서대로, 자식 다시 계산) → 제약.
// 제약 앞에 들어가므로 IK 대상 본을 옮기면 IK가 따라오고, 제약이 걸린 본 자체를 덮어쓰면 제약 결과가 이긴다.
using System;
using System.Collections.Generic;

namespace KGBone.Core
{
    /// <summary>본의 로컬 값(부모 기준, 에디터 속성 패널과 같은 값) 중 정한 것만 바꾼다. Mix(0~1) 비율로 애니메이션 값과 섞는다.</summary>
    public sealed class BoneOverride
    {
        public float? Rotation, X, Y, ScaleX, ScaleY;
        public float Mix = 1f;
    }

    public sealed class PoseOverrides
    {
        struct WorldTarget
        {
            public bool HasPosition, HasRotation;
            public float X, Y, PositionMix, Rotation, RotationMix;
        }

        readonly Dictionary<int, string> attachments = new Dictionary<int, string>();
        readonly Dictionary<int, BoneOverride> locals = new Dictionary<int, BoneOverride>();
        readonly Dictionary<int, WorldTarget> worlds = new Dictionary<int, WorldTarget>();
        // 월드 덮어쓰기는 본 순서대로 적용한다(부모를 먼저)
        readonly List<int> worldOrder = new List<int>();

        public bool IsEmpty => attachments.Count == 0 && locals.Count == 0 && worlds.Count == 0;

        /// <summary>슬롯 첨부물을 정한다(애니메이션 키보다 우선). null이면 그 슬롯을 숨긴다.</summary>
        public void SetAttachment(int slot, string attachment)
        {
            if (slot >= 0) attachments[slot] = attachment;
        }

        public void ClearAttachment(int slot) => attachments.Remove(slot);

        public void SetBoneLocal(int bone, BoneOverride value)
        {
            if (bone < 0) return;
            if (value == null) locals.Remove(bone);
            else locals[bone] = value;
        }

        /// <summary>본 원점의 목표 위치(뼈대 좌표). mix(0~1)로 지금 위치와 섞는다.</summary>
        public void SetBoneWorldPosition(int bone, float x, float y, float mix)
        {
            if (bone < 0) return;
            worlds.TryGetValue(bone, out var w);
            w.HasPosition = true;
            w.X = x;
            w.Y = y;
            w.PositionMix = Math.Min(Math.Max(mix, 0f), 1f);
            SetWorld(bone, w);
        }

        /// <summary>본 X축의 목표 방향(뼈대 좌표, 도). mix(0~1)로 지금 방향과 짧은 쪽으로 섞는다.</summary>
        public void SetBoneWorldRotation(int bone, float degrees, float mix)
        {
            if (bone < 0) return;
            worlds.TryGetValue(bone, out var w);
            w.HasRotation = true;
            w.Rotation = degrees;
            w.RotationMix = Math.Min(Math.Max(mix, 0f), 1f);
            SetWorld(bone, w);
        }

        void SetWorld(int bone, WorldTarget w)
        {
            if (!worlds.ContainsKey(bone))
            {
                int at = worldOrder.BinarySearch(bone);
                worldOrder.Insert(at < 0 ? ~at : at, bone);
            }
            worlds[bone] = w;
        }

        public void ClearBone(int bone)
        {
            locals.Remove(bone);
            if (worlds.Remove(bone)) worldOrder.Remove(bone);
        }

        public void ClearAllBones()
        {
            locals.Clear();
            worlds.Clear();
            worldOrder.Clear();
        }

        // ---------- SkeletonPoser가 부른다 ----------

        internal void ApplyAttachments(PoseState pose)
        {
            foreach (var kv in attachments)
                if (kv.Key < pose.SlotAttachments.Length) pose.SlotAttachments[kv.Key] = kv.Value;
        }

        internal void ApplyLocals(PoseState pose)
        {
            foreach (var kv in locals)
            {
                if (kv.Key >= pose.Locals.Length) continue;
                var o = kv.Value;
                float t = Math.Min(Math.Max(o.Mix, 0f), 1f);
                var l = pose.Locals[kv.Key];
                if (o.Rotation.HasValue) l.Rotation = t >= 1f ? o.Rotation.Value : l.Rotation + WorldMath.WrapDeg(o.Rotation.Value - l.Rotation) * t;
                if (o.X.HasValue) l.X += (o.X.Value - l.X) * t;
                if (o.Y.HasValue) l.Y += (o.Y.Value - l.Y) * t;
                if (o.ScaleX.HasValue) l.ScaleX += (o.ScaleX.Value - l.ScaleX) * t;
                if (o.ScaleY.HasValue) l.ScaleY += (o.ScaleY.Value - l.ScaleY) * t;
                pose.Locals[kv.Key] = l;
            }
        }

        internal void ApplyWorlds(SkeletonModel m, PoseState pose)
        {
            foreach (int bi in worldOrder)
            {
                if (bi >= pose.World.Length || !pose.Active[bi]) continue;
                var w = worlds[bi];
                var nw = pose.World[bi];
                if (w.HasRotation)
                    WorldMath.RotateWorld(ref nw, WorldMath.WrapDeg(w.Rotation - nw.RotationX) * w.RotationMix);
                if (w.HasPosition)
                {
                    nw.X += (w.X - nw.X) * w.PositionMix;
                    nw.Y += (w.Y - nw.Y) * w.PositionMix;
                }
                if (w.HasRotation) BoneTransform.SetWorld(m, pose, bi, nw);
                else
                {
                    // 위치만: 회전·크기 로컬 값은 그대로 두고 위치만 부모 기준으로 바꾼다
                    var l = BoneTransform.WorldToLocal(nw, BoneTransform.ParentWorld(m, pose.World, bi), pose.Inherits[bi]);
                    pose.Locals[bi].X = l.X;
                    pose.Locals[bi].Y = l.Y;
                }
                BoneTransform.UpdateFrom(m, pose, bi);
            }
        }
    }
}
