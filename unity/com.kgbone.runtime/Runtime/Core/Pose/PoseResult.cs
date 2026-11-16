// 포즈 요청과 계산 결과(핵심 pose/mod.rs의 PoseRequest·PoseOutput에서 제약·표시 도형을 뺀 것).
using System.Collections.Generic;

namespace KGBone.Core
{
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
        /// <summary>계산에 포함된 본인가(스킨 전용 본이 꺼져 있으면 false).</summary>
        public bool[] Active;
        /// <summary>그리기 순서대로.</summary>
        public readonly List<DrawItem> Draw = new List<DrawItem>();
        public readonly List<FiredEvent> Events = new List<FiredEvent>();
    }
}
