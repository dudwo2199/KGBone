// M11·M12: 재생 컴포넌트(유니티 EditMode 전용). 시간은 Advance로 직접 진행한다.
using System.Collections.Generic;
using System.IO;
using KGBone;
using KGBone.Core;
using NUnit.Framework;
using UnityEditor;
using UnityEngine;
using UnityEngine.TestTools;

namespace KGBone.Tests
{
    public class AnimationComponentTests
    {
        readonly List<GameObject> made = new List<GameObject>();

        [SetUp]
        public void SetUp()
        {
            if (Directory.Exists(ImporterTests.Tmp)) AssetDatabase.DeleteAsset(ImporterTests.Tmp);
            Directory.CreateDirectory(ImporterTests.Tmp);
        }

        [TearDown]
        public void TearDown()
        {
            foreach (var go in made) Object.DestroyImmediate(go);
            made.Clear();
            AssetDatabase.DeleteAsset(ImporterTests.Tmp);
        }

        KGBoneAnimation Make(KGBoneSkeletonData data)
        {
            var go = new GameObject("KGBone 테스트");
            made.Add(go);
            var a = go.AddComponent<KGBoneAnimation>();
            a.playOnAwake = false;
            a.skeletonData = data;
            return a;
        }

        static int VertexCount(KGBoneAnimation a)
        {
            var mesh = a.GetComponent<MeshFilter>().sharedMesh;
            return mesh == null ? 0 : mesh.vertexCount;
        }

        [Test]
        public void m11_play_advances_and_builds_mesh()
        {
            var data = ImporterTests.ImportStage1();
            var a = Make(data);
            a.Play("걷기", true);
            a.Advance(0.5f);
            Assert.AreEqual(0.5f, a.Time, 1e-6f);
            var want = new SkeletonPoser(data.Model).Compute(new PoseRequest { Animation = "걷기", Time = 0.5f, Looping = true });
            Assert.AreEqual(want.Draw.Count, a.LastPose.Draw.Count);
            int vertices = 0, groups = 0;
            BlendMode? prev = null;
            for (int i = 0; i < want.Draw.Count; i++)
            {
                Assert.AreEqual(want.Draw[i].Slot, a.LastPose.Draw[i].Slot);
                CollectionAssert.AreEqual(want.Draw[i].Positions, a.LastPose.Draw[i].Positions);
                vertices += want.Draw[i].Positions.Count / 2;
                if (prev != want.Draw[i].Blend) groups++;
                prev = want.Draw[i].Blend;
            }
            Assert.AreEqual(vertices, VertexCount(a));
            Assert.AreEqual(groups, a.GetComponent<MeshFilter>().sharedMesh.subMeshCount);
            Assert.AreEqual(groups, a.GetComponent<MeshRenderer>().sharedMaterials.Length);
        }

        [Test]
        public void m11_events_fired_once_per_key()
        {
            var a = Make(ImporterTests.ImportStage1());
            var got = new List<string>();
            a.EventFired += e => got.Add($"{e.Name}@{e.Time}");
            a.Play("걷기", true);
            // 0.25초씩 1.25초까지: [0,.25) 발@0, [.5,.75) 발@0.5, [1,1.25) 다음 바퀴 발@0.
            // 1초(길이와 같은 시간)의 숨 키는 구간 끝(끝 제외)에 걸려 나지 않는다(핵심과 같은 규칙)
            for (int i = 0; i < 5; i++) a.Advance(0.25f);
            CollectionAssert.AreEqual(new[] { "발@0", "발@0.5", "발@0" }, got);
        }

        [Test]
        public void m11_unknown_animation_keeps_state()
        {
            var a = Make(ImporterTests.ImportStage1());
            a.Play("걷기", true);
            LogAssert.Expect(LogType.Warning, "애니메이션을 찾을 수 없습니다: 없는애니");
            a.Play("없는애니", true);
            Assert.AreEqual("걷기", a.CurrentAnimation);
            Assert.IsTrue(a.IsPlaying);
            a.Play(null, true);
            Assert.IsNull(a.CurrentAnimation);
            Assert.IsFalse(a.IsPlaying);
        }

        [Test]
        public void m11_non_loop_holds_last_frame_and_completes_once()
        {
            var a = Make(ImporterTests.ImportStage1());
            int completed = 0;
            a.Completed += () => completed++;
            a.Play("걷기", false);
            a.Advance(0.7f);
            a.Advance(0.7f);
            Assert.AreEqual(1f, a.Time, 1e-6f);
            Assert.AreEqual(1, completed);
            Assert.IsFalse(a.IsPlaying);
            a.Advance(0.5f);
            Assert.AreEqual(1f, a.Time, 1e-6f);
            Assert.AreEqual(1, completed);
        }

        [Test]
        public void m11_no_data_draws_nothing()
        {
            var a = Make(null);
            Assert.DoesNotThrow(() => a.Advance(0.1f));
            Assert.AreEqual(0, VertexCount(a));
            Assert.IsFalse(a.TryGetBone("root", out _, out _, out _));
        }

        [Test]
        public void m12_try_get_bone_with_flip()
        {
            var data = ImporterTests.ImportStage1();
            var a = Make(data);
            a.Stop();
            Assert.IsTrue(a.TryGetBone("팔-보통", out Vector3 p1, out float r1, out Vector2 s1));
            var w = new SkeletonPoser(data.Model).Compute(new PoseRequest()).Bones[data.Model.BoneIndex("팔-보통")];
            Assert.AreEqual(w.X / 100f, p1.x, 1e-4f);
            Assert.AreEqual(w.Y / 100f, p1.y, 1e-4f);
            Assert.AreEqual(Mathf.Atan2(w.C, w.A) * Mathf.Rad2Deg, r1, 1e-3f);
            a.flipX = true;
            a.Advance(0);
            Assert.IsTrue(a.TryGetBone("팔-보통", out Vector3 p2, out _, out _));
            Assert.AreEqual(-p1.x, p2.x, 1e-4f);
            Assert.AreEqual(p1.y, p2.y, 1e-4f);
            Assert.IsFalse(a.TryGetBone("없는본", out _, out _, out _));
        }
    }
}
