// IK 제약(1본·2본, 부피 보존). KGBone 핵심 crates/kgbone-core/src/pose/ik.rs와 같은 계산.
// 계산은 모두 "첫 본의 부모 공간"(부모의 상속 행렬 + 부모 원점)에서 한다.
using System;

namespace KGBone.Core
{
    public sealed class IkSolver : IConstraintSolver
    {
        struct Opts
        {
            public float Mix;
            public bool Compress, Stretch, Uniform, Volume;
        }

        public void Apply(SkeletonModel m, int index, PoseState pose, SolveContext ctx)
        {
            var c = (IkConstraintData)m.Constraints[index];
            var vals = pose.ConstraintValues[index];
            float softness = Math.Max(vals[1], 0f);
            float bend = vals[2] < 0f ? -1f : 1f;
            var o = new Opts { Mix = vals[0], Compress = vals[3] > 0.5f, Stretch = vals[4] > 0.5f, Uniform = c.Uniform, Volume = c.Volume };
            if (c.Target < 0) return;
            float tx = pose.World[c.Target].X, ty = pose.World[c.Target].Y;
            // 핵심은 못 찾은 본 이름을 빼고 센다(활성 판정에서 이미 걸러지지만 같은 규칙을 지킨다)
            int n = 0, b0 = -1, b1 = -1;
            foreach (int b in c.Bones)
            {
                if (b < 0) continue;
                if (n == 0) b0 = b; else if (n == 1) b1 = b;
                n++;
            }
            if (n == 1) OneBone(m, pose, b0, tx, ty, o);
            else if (n == 2) TwoBone(m, pose, b0, b1, tx, ty, softness, bend, o);
        }

        static WorldBone Space(SkeletonModel m, PoseState pose, int i) =>
            WorldMath.ParentSpace(BoneTransform.ParentWorld(m, pose.World, i), pose.Inherits[i]);

        /// <summary>본 i가 월드 점 (tx, ty)를 향하는 로컬 회전과, 부모 공간에서의 거리.</summary>
        static (float rot, float dist) Aim(SkeletonModel m, PoseState pose, int i, float tx, float ty)
        {
            var (px, py) = Space(m, pose, i).WorldToLocal(tx, ty);
            var l = pose.Locals[i];
            float dx = px - l.X, dy = py - l.Y;
            return (MathF.Atan2(dy, dx) * WorldMath.RadToDeg - l.ShearX, MathF.Sqrt(dx * dx + dy * dy));
        }

        static void ApplyScale(PoseState pose, int i, float f, Opts o)
        {
            ref var l = ref pose.Locals[i];
            l.ScaleX *= f;
            if (o.Uniform) l.ScaleY *= f;
            else if (o.Volume && MathF.Abs(f) > 1e-6f) l.ScaleY /= f;
        }

        static void OneBone(SkeletonModel m, PoseState pose, int i, float tx, float ty, Opts o)
        {
            var (rot, d) = Aim(m, pose, i, tx, ty);
            float cur = pose.Locals[i].Rotation;
            pose.Locals[i].Rotation = cur + WorldMath.WrapDeg(rot - cur) * o.Mix;
            float len = m.Bones[i].Length * pose.Locals[i].ScaleX;
            if (MathF.Abs(len) > 1e-5f && ((o.Compress && d < len) || (o.Stretch && d > len)))
            {
                float s = d / len;
                ApplyScale(pose, i, 1f + (s - 1f) * o.Mix, o);
            }
            BoneTransform.UpdateFrom(m, pose, i);
        }

        static float Hyp(float x, float y) => MathF.Sqrt(x * x + y * y);

        static void TwoBone(SkeletonModel m, PoseState pose, int p, int c, float tx, float ty, float softness, float bend, Opts o)
        {
            var space = Space(m, pose, p);
            var lp = pose.Locals[p];
            float p0x = lp.X, p0y = lp.Y;
            var cw = pose.World[c];
            var (c0x, c0y) = space.WorldToLocal(cw.X, cw.Y);
            var (tipwx, tipwy) = cw.LocalToWorld(m.Bones[c].Length, 0f);
            var (tipx, tipy) = space.WorldToLocal(tipwx, tipwy);
            var (ttx, tty) = space.WorldToLocal(tx, ty);
            float l1 = Hyp(c0x - p0x, c0y - p0y);
            float l2 = Hyp(tipx - c0x, tipy - c0y);
            float d = Hyp(ttx - p0x, tty - p0y);
            if (l1 < 1e-5f || d < 1e-5f)
            {
                OneBone(m, pose, c, tx, ty, o);
                return;
            }
            float maxl = l1 + l2;
            float a1 = l1, a2 = l2, dd = d, k = 1f;
            if (o.Stretch && d > maxl && maxl > 1e-5f)
            {
                k = d / maxl;
                a1 *= k;
                a2 *= k;
            }
            else
            {
                if (softness > 0f && d > maxl - softness)
                {
                    float start = maxl - softness;
                    dd = start + softness * (1f - MathF.Exp(-(d - start) / softness));
                }
                dd = Math.Max(Math.Min(dd, maxl), MathF.Abs(l1 - l2));
            }
            float cosA = Math.Min(Math.Max((a1 * a1 + dd * dd - a2 * a2) / (2f * a1 * dd), -1f), 1f);
            float angA = MathF.Acos(cosA) * WorldMath.RadToDeg;
            float thetaT = MathF.Atan2(tty - p0y, ttx - p0x) * WorldMath.RadToDeg;
            float theta1 = thetaT - bend * angA;
            float aPc = MathF.Atan2(c0y - p0y, c0x - p0x) * WorldMath.RadToDeg;
            float delta = WorldMath.WrapDeg(theta1 - aPc);
            // 자식이 향할 점(부모 공간): 목표 방향으로 d'만큼
            float dir = thetaT * WorldMath.DegToRad;
            var (qx, qy) = space.LocalToWorld(p0x + dd * MathF.Cos(dir), p0y + dd * MathF.Sin(dir));

            var origP = pose.Locals[p];
            float origCRot = pose.Locals[c].Rotation;
            // 부모를 끝까지(mix 1) 돌렸을 때 자식의 목표 회전
            pose.Locals[p].Rotation = origP.Rotation + delta;
            if (k != 1f) ApplyScale(pose, p, k, o);
            BoneTransform.UpdateFrom(m, pose, p);
            var (rotC, _) = Aim(m, pose, c, qx, qy);
            // mix 반영
            pose.Locals[p] = origP;
            pose.Locals[p].Rotation = origP.Rotation + delta * o.Mix;
            if (k != 1f) ApplyScale(pose, p, 1f + (k - 1f) * o.Mix, o);
            pose.Locals[c].Rotation = origCRot + WorldMath.WrapDeg(rotC - origCRot) * o.Mix;
            BoneTransform.UpdateFrom(m, pose, Math.Min(p, c));
        }
    }
}
