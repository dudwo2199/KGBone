// 물리 제약(고정 시간 간격). KGBone 핵심 crates/kgbone-core/src/pose/physics.rs와 같은 계산.
// - 질점 두 개: 본 원점(이동 x·y에 씀)과 본 끝(회전·scaleX·shearX에 씀). 길이 0인 본은 본 X축 방향 referenceScale 거리의 점.
// - 상태는 질점마다 "본 위치에서 벗어난 양"(offset)과 속도. 본이 움직이면 offset −= 이동량 × inertia.
// - 한 단계(dt = 1/fps): 가속 = (−strength × offset − c × 속도) / mass + (wind, −gravity) × referenceScale/100,
//   c = damping × √(strength × mass). 속도 += 가속 × dt, |속도| ≤ limit, offset += 속도 × dt.
// - Advance(dt)는 누적 시간을 고정 간격으로 나눈 단계 수만큼 진행하고 남은 시간은 다음으로 넘긴다(double).
//   그 프레임의 본 이동량은 단계 수로 고르게 나눈다. 단계 수가 0이면 이동량을 다음 프레임으로 미룬다.
using System;
using System.Collections.Generic;

namespace KGBone.Core
{
    /// <summary>물리 제약별 속도·위치 상태(제약 이름 → 상태). SkeletonPoser가 들고 있다.</summary>
    public sealed class PhysicsState
    {
        internal struct Point
        {
            /// <summary>지난 단계의 본 위치(월드).</summary>
            public float LastX, LastY;
            public float OffX, OffY;
            public float VelX, VelY;
        }

        internal sealed class ConstraintState
        {
            public Point Origin, Tip;
            /// <summary>고정 간격으로 나누고 남은 시간(초).</summary>
            public double Remaining;
        }

        internal readonly Dictionary<string, ConstraintState> Map = new Dictionary<string, ConstraintState>(StringComparer.Ordinal);

        public void ResetAll() => Map.Clear();

        /// <summary>이름 ""은 모든 물리 제약.</summary>
        public void ResetNamed(string name)
        {
            if (string.IsNullOrEmpty(name)) Map.Clear();
            else Map.Remove(name);
        }
    }

    public sealed class PhysicsSolver : IConstraintSolver
    {
        readonly PhysicsState state;
        public PhysicsSolver(PhysicsState state) { this.state = state; }

        struct Params
        {
            public float Inertia, Strength, Damping, Mass, ForceX, ForceY, Limit;
        }

        static void StepPoint(ref PhysicsState.Point p, float mx, float my, in Params prm, float dt)
        {
            p.OffX -= mx * prm.Inertia;
            p.OffY -= my * prm.Inertia;
            float mass = Math.Max(prm.Mass, 1e-4f);
            float c = prm.Damping * MathF.Sqrt(Math.Max(prm.Strength, 0f) * mass);
            float ax = (-prm.Strength * p.OffX - c * p.VelX) / mass + prm.ForceX;
            float ay = (-prm.Strength * p.OffY - c * p.VelY) / mass + prm.ForceY;
            p.VelX += ax * dt;
            p.VelY += ay * dt;
            float speed = MathF.Sqrt(p.VelX * p.VelX + p.VelY * p.VelY);
            if (speed > prm.Limit && speed > 0f)
            {
                float k = prm.Limit / speed;
                p.VelX *= k;
                p.VelY *= k;
            }
            p.OffX += p.VelX * dt;
            p.OffY += p.VelY * dt;
        }

        /// <summary>질점 하나를 steps단계 진행. 이번 프레임 이동량(cur − last)은 단계마다 고르게 나눈다.</summary>
        static void AdvancePoint(ref PhysicsState.Point p, float curX, float curY, uint steps, in Params prm, float dt)
        {
            if (steps == 0) return;
            float px = (curX - p.LastX) / steps, py = (curY - p.LastY) / steps;
            for (uint i = 0; i < steps; i++) StepPoint(ref p, px, py, prm, dt);
            p.LastX = curX;
            p.LastY = curY;
        }

        /// <summary>점 p의 지난 위치에서 바깥 움직임이 만든 이동(평행 이동 + 원점 기준 회전)을 뺀다.</summary>
        static void ShiftLast(ref PhysicsState.Point p, in WorldMotion w)
        {
            float r = w.Rotation * WorldMath.DegToRad;
            float cs = MathF.Cos(r), sn = MathF.Sin(r);
            float dx = w.X + (cs * p.LastX - sn * p.LastY) - p.LastX;
            float dy = w.Y + (sn * p.LastX + cs * p.LastY) - p.LastY;
            p.LastX -= dx;
            p.LastY -= dy;
        }

        public void Apply(SkeletonModel m, int index, PoseState pose, SolveContext ctx)
        {
            var c = (PhysicsConstraintData)m.Constraints[index];
            int bi = c.Bone;
            if (bi < 0) return;
            var v = pose.ConstraintValues[index];
            float rs = m.ReferenceScale / 100f;
            var prm = new Params { Inertia = v[0], Strength = v[1], Damping = v[2], Mass = v[3], ForceX = v[4] * rs, ForceY = -v[5] * rs, Limit = Math.Max(c.Limit, 0f) };
            float mix = v[6];
            var w = pose.World[bi];
            float len = m.Bones[bi].Length;
            float tipLocal = len > 0f ? len : Math.Max(m.ReferenceScale, 1f);
            float ox = w.X, oy = w.Y;
            var (tx, ty) = w.LocalToWorld(tipLocal, 0f);

            if (!state.Map.TryGetValue(c.Name, out var st))
            {
                st = new PhysicsState.ConstraintState();
                st.Origin.LastX = ox; st.Origin.LastY = oy;
                st.Tip.LastX = tx; st.Tip.LastY = ty;
                state.Map[c.Name] = st;
            }
            if (ctx.Physics.Kind == PhysicsStep.StepKind.Advance)
            {
                // 바깥 움직임은 본 이동량처럼 다룬다: 지난 위치를 반대로 옮겨 두면 (cur − last)에 더해지고,
                // inertia가 곱해지며, 단계가 0이면 last가 그대로라 다음 단계로 미뤄진다. 0이면 아무것도 하지 않는다(핵심과 같은 경로).
                if (!ctx.WorldMotion.IsZero)
                {
                    ShiftLast(ref st.Origin, ctx.WorldMotion);
                    ShiftLast(ref st.Tip, ctx.WorldMotion);
                }
                double fps = c.Fps > 0f ? c.Fps : 60.0;
                st.Remaining += Math.Max(ctx.Physics.Dt, 0f);
                double steps = Math.Max(Math.Floor(st.Remaining * fps + 1e-6), 0.0);
                st.Remaining = Math.Max(st.Remaining - steps / fps, 0.0);
                float stepDt = (float)(1.0 / fps);
                AdvancePoint(ref st.Origin, ox, oy, (uint)steps, prm, stepDt);
                AdvancePoint(ref st.Tip, tx, ty, (uint)steps, prm, stepDt);
            }
            float oX = st.Origin.OffX, oY = st.Origin.OffY, tX = st.Tip.OffX, tY = st.Tip.OffY;
            if ((oX == 0f && oY == 0f && tX == 0f && tY == 0f) || mix == 0f) return;
            var nw = w;
            nw.X += oX * c.X * mix;
            nw.Y += oY * c.Y * mix;
            float bx = tx - ox, by = ty - oy;
            float sx = bx + tX, sy = by + tY;
            float angle = WorldMath.WrapDeg(MathF.Atan2(sy, sx) * WorldMath.RadToDeg - MathF.Atan2(by, bx) * WorldMath.RadToDeg);
            WorldMath.RotateWorld(ref nw, angle * c.Rotate * mix);
            WorldMath.RotateWorldXAxis(ref nw, angle * c.ShearX * mix);
            float baseLen = MathF.Sqrt(bx * bx + by * by);
            if (baseLen > 1e-6f && c.ScaleX != 0f)
            {
                float s = MathF.Sqrt(sx * sx + sy * sy) / baseLen;
                float f = 1f + (s - 1f) * c.ScaleX * mix;
                nw.A *= f;
                nw.C *= f;
                if (c.Volume && MathF.Abs(f) > 1e-6f)
                {
                    nw.B /= f;
                    nw.D /= f;
                }
            }
            BoneTransform.SetWorld(m, pose, bi, nw);
            BoneTransform.UpdateFrom(m, pose, bi);
        }
    }
}
