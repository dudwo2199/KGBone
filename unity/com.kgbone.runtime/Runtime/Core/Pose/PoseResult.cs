// 포즈 요청과 계산 결과(핵심 pose/mod.rs의 PoseRequest·PoseOutput에서 제약·표시 도형을 뺀 것).
using System.Collections.Generic;

namespace KGBone.Core
{
    /// <summary>
    /// 이번 프레임에 뼈대 좌표계 자체가 움직인 양: (X, Y)만큼 옮기고 뼈대 원점을 중심으로 Rotation도 돌린 것.
    /// 유니티 컴포넌트가 GameObject의 이동·회전 변화로 만든다(유니티 전용, 핵심에는 없음).
    /// </summary>
    public struct WorldMotion
    {
        public float X, Y, Rotation;
        public bool IsZero => X == 0f && Y == 0f && Rotation == 0f;
    }

    public sealed class PoseRequest
    {
        /// <summary>적용할 애니메이션(null이면 셋업 포즈).</summary>
        public string Animation;
        /// <summary>애니메이션 시점(초). 반복이면 실제 누적 시간을 그대로 줘도 된다.</summary>
        public float Time;
        /// <summary>이전 시점. 있으면 [LastTime, Time) 구간의 이벤트를 돌려준다.</summary>
        public float? LastTime;
        /// <summary>켤 스킨(null이면 기본 스킨만).</summary>
        public string Skin;
        public bool Looping;
        /// <summary>물리 진행 방법(기본: 진행하지 않음).</summary>
        public PhysicsStep Physics = PhysicsStep.None;
        /// <summary>
        /// 트랙 재생(유니티 전용). null이 아니면 Animation·Time·Looping 대신 트랙을 적용한다.
        /// 이벤트·물리 reset 키는 LastTime이 있을 때만(값은 쓰지 않고 "이번 계산에서 낼지" 표시로만) 트랙의 마지막 Update 구간으로 낸다.
        /// </summary>
        public AnimationTracks Tracks;
        /// <summary>이번 프레임 바깥(GameObject) 움직임(뼈대 좌표, 반영 비율을 곱한 값). 물리 질점의 관성으로 들어간다. 기본 0.</summary>
        public WorldMotion WorldMotion;
    }

    /// <summary>그릴 삼각형 묶음 하나(슬롯 하나의 이미지·메시).</summary>
    public sealed class DrawItem
    {
        public string Slot, Attachment;
        /// <summary>이미지 경로(확장자 없음, imagesPath 기준).</summary>
        public string Image;
        public BlendMode Blend;
        /// <summary>슬롯 색 × 첨부물 색.</summary>
        public ColorF Color;
        /// <summary>어두운 색(tint black, A는 쓰지 않음). 없으면 null.</summary>
        public ColorF? Dark;
        /// <summary>월드 좌표 [x0, y0, x1, y1, …](KGBone 단위, y 위).</summary>
        public readonly List<float> Positions = new List<float>();
        /// <summary>이미지 한 장 기준 0~1 텍스처 좌표(v는 아래로 증가).</summary>
        public readonly List<float> Uvs = new List<float>();
        public readonly List<int> Indices = new List<int>();
    }

    public sealed class PoseResult
    {
        /// <summary>skeleton.bones 순서의 월드 변환.</summary>
        public WorldBone[] Bones;
        /// <summary>본마다 제약까지 반영된 로컬 값.</summary>
        public LocalTransform[] Locals;
        /// <summary>계산에 포함된 본인가(스킨 전용 본이 꺼져 있으면 false).</summary>
        public bool[] Active;
        /// <summary>그리기 순서대로.</summary>
        public readonly List<DrawItem> Draw = new List<DrawItem>();
        public readonly List<FiredEvent> Events = new List<FiredEvent>();
    }
}
