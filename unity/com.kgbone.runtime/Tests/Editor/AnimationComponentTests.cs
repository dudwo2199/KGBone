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

        // ---------- 최종 검토 지적 ----------
        [Test]
        public void m11_play_inside_event_handler_does_not_throw()
        {
            // 검토 C1: 이벤트 처리기에서 Play를 부르면 반복 중인 이벤트 목록이 바뀌어 예외가 나던 문제
            var a = Make(ImporterTests.ImportStage1());
            var got = new List<string>();
            a.EventFired += e =>
            {
                got.Add(e.Name);
                if (e.Name == "발" && a.CurrentAnimation == "걷기") a.Play("정지", true);
            };
            a.Play("걷기", true);
            Assert.DoesNotThrow(() => a.Advance(0.25f));
            CollectionAssert.AreEqual(new[] { "발" }, got);
            Assert.AreEqual("정지", a.CurrentAnimation);
        }

        [Test]
        public void m11_non_loop_fires_end_key_and_zero_length_keys_once()
        {
            // 검토 I1: 반복하지 않으면 끝 시간의 키(숨@1)와 길이 0 애니메이션의 0초 키가 빠지던 문제
            var a = Make(ImporterTests.ImportStage1());
            var got = new List<string>();
            a.EventFired += e => got.Add($"{e.Name}@{e.Time}");
            a.Play("걷기", false);
            a.Advance(0.6f);
            a.Advance(0.6f);
            a.Advance(0.6f);
            CollectionAssert.AreEqual(new[] { "발@0", "발@0.5", "숨@1" }, got);
            got.Clear();
            a.Play("정지", false);
            a.Advance(0.1f);
            a.Advance(0.1f);
            CollectionAssert.AreEqual(new[] { "숨@0" }, got);
        }

        [Test]
        public void m11_negative_time_scale_fires_no_events()
        {
            // 검토 I2: 시간을 거꾸로 돌리면 거의 모든 이벤트가 매 프레임 나던 문제
            var a = Make(ImporterTests.ImportStage1());
            int count = 0;
            a.EventFired += e => count++;
            a.Play("걷기", true);
            a.Time = 0.9f;
            a.timeScale = -1;
            a.Advance(0.1f);
            a.Advance(0.1f);
            Assert.AreEqual(0, count);
            Assert.AreEqual(0.7f, a.Time, 1e-5f);
        }

        // ---------- M20 트랙 컴포넌트 ----------
        static KGBoneSkeletonData ImportTracks() => ImporterTests.ImportGolden("unity-stage3-tracks");

        static WorldBone Plain(KGBoneSkeletonData data, string anim, float time, bool loop, string bone)
        {
            var p = new SkeletonPoser(data.Model);
            var r = p.Compute(new PoseRequest { Animation = anim, Time = time, Looping = loop });
            return r.Bones[p.BoneIndex(bone)];
        }

        static void SameBone(WorldBone want, WorldBone got, string what)
        {
            Assert.AreEqual(want.A, got.A, 1e-4f, what + ".a");
            Assert.AreEqual(want.C, got.C, 1e-4f, what + ".c");
            Assert.AreEqual(want.X, got.X, 1e-3f, what + ".x");
            Assert.AreEqual(want.Y, got.Y, 1e-3f, what + ".y");
        }

        WorldBone Bone(KGBoneAnimation a, string name) => a.LastPose.Bones[a.skeletonData.Model.BoneIndex(name)];

        [Test]
        public void m20_existing_play_unchanged()
        {
            var data = ImporterTests.ImportStage1();
            var a = Make(data);
            a.Play("걷기", true);
            a.Advance(0.25f);
            a.Advance(0.4f);
            Assert.AreEqual(0.65f, a.Time, 1e-6f);
            var p = new SkeletonPoser(data.Model);
            var want = p.Compute(new PoseRequest { Animation = "걷기", Time = a.Time, Looping = true });
            for (int i = 0; i < want.Bones.Length; i++) SameBone(want.Bones[i], a.LastPose.Bones[i], "bone " + i);
            Assert.AreEqual("걷기", a.GetTrackAnimation(0));
        }

        [Test]
        public void m20_walk_with_attack_overlay()
        {
            var data = ImportTracks();
            var a = Make(data);
            a.SetAnimation(0, "걷기", true);
            a.SetAnimation(1, "공격", false);
            a.AddEmpty(1, 0, 0.2f);
            a.Advance(0.1f);
            SameBone(Plain(data, "걷기", 0.1f, true, "다리"), Bone(a, "다리"), "다리(걷기 그대로)");
            Assert.AreNotEqual(Plain(data, "걷기", 0.1f, true, "팔").A, Bone(a, "팔").A, "팔은 공격 값");
            for (int i = 0; i < 6; i++) a.Advance(0.1f); // 0.7초: 공격 끝(0.4) + 0.2초 빠짐 = 0.6초에 다 빠짐
            SameBone(Plain(data, "걷기", 0.7f, true, "팔"), Bone(a, "팔"), "팔이 걷기로 돌아옴");
            Assert.IsNull(a.GetTrackAnimation(1));
        }

        [Test]
        public void m20_animation_name_change_uses_default_mix()
        {
            var data = ImportTracks();
            var a = Make(data);
            a.defaultMix = 0.2f;
            a.Play("걷기", true);
            a.Advance(0.5f);
            a.animationName = "달리기";
            a.Advance(0.1f);
            Assert.AreEqual("달리기", a.CurrentAnimation);
            float walk = Plain(data, "걷기", 0.6f, true, "다리").A, run = Plain(data, "달리기", 0.1f, true, "다리").A, got = Bone(a, "다리").A;
            Assert.Greater(got, System.Math.Min(walk, run) + 1e-4f);
            Assert.Less(got, System.Math.Max(walk, run) - 1e-4f);
        }

        [Test]
        public void m20_track_completed_and_event_track()
        {
            var a = Make(ImportTracks());
            var events = new List<KGBoneEvent>();
            var done = new List<string>();
            int track0Completed = 0;
            a.EventFired += e => events.Add(e);
            a.TrackCompleted += (t, n) => done.Add(t + ":" + n);
            a.Completed += () => track0Completed++;
            a.SetAnimation(1, "공격", false);
            a.Advance(0.3f);
            Assert.AreEqual(1, events.Count);
            Assert.AreEqual("타격", events[0].Name);
            Assert.AreEqual(1, events[0].Track);
            Assert.AreEqual("공격", events[0].Animation);
            Assert.AreEqual(3, events[0].Int);
            a.Advance(0.2f);
            a.Advance(0.2f);
            CollectionAssert.AreEqual(new[] { "1:공격" }, done);
            Assert.AreEqual(0, track0Completed);
        }

        [Test]
        public void m20_unknown_animation_keeps_track()
        {
            var a = Make(ImportTracks());
            a.SetAnimation(0, "걷기", true);
            LogAssert.Expect(LogType.Warning, "애니메이션을 찾을 수 없습니다: 없음");
            Assert.IsNull(a.SetAnimation(0, "없음", true));
            Assert.IsNull(a.AddAnimation(0, "없음", true));
            Assert.IsNull(a.SetAnimation(1, "없음", true));
            LogAssert.NoUnexpectedReceived();
            Assert.AreEqual("걷기", a.GetTrackAnimation(0));
            Assert.IsNull(a.GetTrackAnimation(1));
        }

        // ---------- M21 첨부물·스킨 합치기 ----------
        static string DrawnImage(KGBoneAnimation a, string slot)
        {
            foreach (var d in a.LastPose.Draw) if (d.Slot == slot) return d.Image;
            return null;
        }

        [Test]
        public void m21_set_attachment_swaps_weapon()
        {
            var a = Make(ImportTracks());
            a.SetAnimation(0, "걷기", true);
            Assert.AreEqual("hat-red", DrawnImage(a, "무기"));
            a.SetAttachment("무기", "도끼");
            a.Advance(0.1f);
            Assert.AreEqual("hat-blue", DrawnImage(a, "무기"));
            a.SetAttachment("무기", null);
            Assert.IsNull(DrawnImage(a, "무기"), "null이면 숨김");
            a.ClearAttachment("무기");
            Assert.AreEqual("hat-red", DrawnImage(a, "무기"));
        }

        [Test]
        public void m21_set_skins_combines()
        {
            var a = Make(ImportTracks());
            a.SetSkins("몸", "칼");
            Assert.AreEqual("몸", a.skin);
            Assert.AreEqual("gem", DrawnImage(a, "갑옷"));
            Assert.AreEqual("tail", DrawnImage(a, "무기"), "뒤 스킨(칼)의 무기");
            a.skin = "몸"; // 필드를 그대로 두면 합친 상태 유지
            a.Advance(0);
            Assert.AreEqual("tail", DrawnImage(a, "무기"));
            a.skin = "칼"; // 필드를 바꾸면 그 스킨 하나
            a.Advance(0);
            Assert.IsNull(DrawnImage(a, "갑옷"));
            Assert.AreEqual("tail", DrawnImage(a, "무기"));
        }

        [Test]
        public void m21_unknown_names_warn_once()
        {
            var a = Make(ImportTracks());
            LogAssert.Expect(LogType.Warning, "슬롯을 찾을 수 없습니다: 없는슬롯");
            LogAssert.Expect(LogType.Warning, "첨부물을 찾을 수 없습니다: 무기/없는것");
            LogAssert.Expect(LogType.Warning, "본을 찾을 수 없습니다: 없는본");
            LogAssert.Expect(LogType.Warning, "스킨을 찾을 수 없습니다: 없는스킨");
            for (int i = 0; i < 2; i++)
            {
                a.SetAttachment("없는슬롯", "칼");
                a.SetAttachment("무기", "없는것");
                a.SetBoneLocal("없는본", new BoneOverride { Rotation = 1 });
                a.SetBoneWorldRotation("없는본", 10);
                a.SetSkins("없는스킨");
            }
            LogAssert.NoUnexpectedReceived();
            a.Advance(0);
            Assert.AreEqual("hat-red", DrawnImage(a, "무기"));
        }

        // ---------- M22 본 덮어쓰기 ----------
        [Test]
        public void m22_world_position_with_flip_and_scale()
        {
            var a = Make(ImportTracks());
            a.transform.position = new Vector3(1, 2, 0);
            a.transform.localScale = new Vector3(2, 2, 1);
            a.transform.rotation = Quaternion.Euler(0, 0, 20);
            a.flipX = true;
            var target = new Vector3(3, 1, 0);
            a.SetBoneWorldPosition("팔", target);
            Assert.IsTrue(a.TryGetBone("팔", out var p, out _, out _));
            Assert.AreEqual(target.x, p.x, 1e-3f);
            Assert.AreEqual(target.y, p.y, 1e-3f);
            a.ClearAllBones();
            Assert.IsTrue(a.TryGetBone("팔", out p, out _, out _));
            Assert.Greater((p - target).magnitude, 0.1f);
        }

        [Test]
        public void m22_world_rotation_follows_mouse_angle()
        {
            foreach (bool flip in new[] { false, true })
            {
                var a = Make(ImportTracks());
                a.flipX = flip;
                a.transform.rotation = Quaternion.Euler(0, 0, 30);
                a.SetBoneWorldRotation("팔", 45);
                Assert.IsTrue(a.TryGetBone("팔", out _, out float rot, out _));
                Assert.AreEqual(0f, Mathf.DeltaAngle(45f, rot), 1e-2f, "flipX " + flip);
                a.SetBoneLocal("팔", new BoneOverride { Rotation = 10 });
                a.ClearBone("팔");
                Assert.IsTrue(a.TryGetBone("팔", out _, out float back, out _));
                Assert.Greater(Mathf.Abs(Mathf.DeltaAngle(45f, back)), 1f, "해제하면 원래대로");
            }
        }

        static int KGBoneMeshCount()
        {
            int n = 0;
            foreach (var m in Resources.FindObjectsOfTypeAll<Mesh>()) if (m.name == "KGBone") n++;
            return n;
        }

        [Test]
        public void m11_domain_reload_does_not_leak_mesh()
        {
            // 검토 I5: 스크립트를 다시 컴파일하면(도메인 다시 불러오기) 컴포넌트가 메시 참조를 잃는데, DontSave 메시는 남아 쌓이던 문제.
            // 다시 불러오기 순서를 흉내 낸다: 꺼짐(OnDisable) → 관리 필드 사라짐 → 켜짐(OnEnable)
            var a = Make(ImporterTests.ImportStage1());
            a.Stop(); // 데이터를 넣은 뒤 한 번 그려 메시를 만든다
            int before = KGBoneMeshCount();
            Assert.Greater(before, 0);
            var meshField = typeof(KGBoneAnimation).GetField("mesh", System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
            for (int i = 0; i < 3; i++)
            {
                a.enabled = false;
                meshField.SetValue(a, null);
                a.enabled = true;
            }
            Assert.AreEqual(before, KGBoneMeshCount());
            Assert.Greater(VertexCount(a), 0);
        }

        [Test]
        public void m11_reimported_data_refreshes_pose()
        {
            // 검토 I6: .kgbone을 다시 가져와 같은 데이터 에셋의 내용이 바뀌어도 예전 뼈대를 그리던 문제
            var data = ImporterTests.ImportStage1();
            var a = Make(data);
            a.Stop();
            Assert.AreEqual(13, a.LastPose.Bones.Length);
            data.Initialize(@"{""skeleton"":{""bones"":[{""name"":""root""},{""name"":""a"",""parent"":""root""}]}}",
                data.Texture, new Material[4], new ImageEntry[0], 100, new string[0]);
            a.Advance(0);
            Assert.AreEqual(2, a.LastPose.Bones.Length);
        }

        [Test]
        public void m11_reimport_redraws_scene_without_advance()
        {
            // 에디터에서 .kgbone을 고쳐 저장하고 유니티로 옮기면(다시 가져오기) 재생하지 않아도 씬의 캐릭터가 새 내용으로 다시 그려져야 한다.
            // 에디터 모드에서는 Update가 계산하지 않으므로, 다시 가져오기를 알려 주지 않으면 예전 그림이 남던 문제
            var data = ImporterTests.ImportStage1();
            var a = Make(data);
            a.Stop();
            Assert.AreEqual(13, a.LastPose.Bones.Length);
            string path = AssetDatabase.GetAssetPath(data);
            File.WriteAllText(path, @"{""formatVersion"":1,""skeleton"":{""bones"":[{""name"":""root""},{""name"":""a"",""parent"":""root""}]}}");
            ImporterTests.Import(path);
            Assert.AreEqual(2, a.LastPose.Bones.Length);
            Assert.AreEqual(0, VertexCount(a));
        }

        [UnityTest]
        public System.Collections.IEnumerator m11_spawned_from_code_plays_on_start()
        {
            // 게임 코드가 AddComponent 뒤에 데이터와 애니메이션 이름을 넣으면 시작하자마자 재생해야 한다.
            // Awake는 AddComponent 안에서 바로 불려 그때는 데이터가 없어, 자동 재생이 빠지던 문제
            yield return new EnterPlayMode();
            // 플레이 모드 안에서는 가져오기가 미뤄질 수 있어 데이터를 메모리에서 만든다
            var data = ScriptableObject.CreateInstance<KGBoneSkeletonData>();
            data.Initialize(@"{""skeleton"":{""bones"":[{""name"":""root""}],""animations"":[{""name"":""걷기""},{""name"":""정지""}]}}",
                null, new Material[4], new ImageEntry[0], 100, new string[0]);
            var auto = new GameObject("자동").AddComponent<KGBoneAnimation>();
            auto.skeletonData = data;
            auto.animationName = "걷기";
            // Start 전에 직접 Play를 부른 것은 자동 재생이 덮어쓰지 않는다
            var manual = new GameObject("직접").AddComponent<KGBoneAnimation>();
            manual.skeletonData = data;
            manual.animationName = "걷기";
            manual.Play("정지", true);
            yield return null;
            yield return null;
            bool autoPlaying = auto.IsPlaying;
            string autoAnim = auto.CurrentAnimation, manualAnim = manual.CurrentAnimation;
            yield return new ExitPlayMode();
            Assert.IsTrue(autoPlaying);
            Assert.AreEqual("걷기", autoAnim);
            Assert.AreEqual("정지", manualAnim);
        }

        [UnityTest]
        public System.Collections.IEnumerator m11_editor_redraws_after_script_assigns_fields()
        {
            // 에디터 모드에서 스크립트로 데이터·스킨·애니메이션·뒤집기를 바꾸면 다음 에디터 갱신에서 다시 그려야 한다
            var data = ImporterTests.ImportStage1();
            var a = Make(null);
            Assert.AreEqual(0, VertexCount(a));
            a.skeletonData = data;
            for (int i = 0; i < 10 && VertexCount(a) == 0; i++) yield return null;
            Assert.Greater(VertexCount(a), 0);
            Assert.AreEqual(13, a.LastPose.Bones.Length);
            var before = a.GetComponent<MeshFilter>().sharedMesh.vertices[0];
            a.flipX = true;
            for (int i = 0; i < 10 && a.GetComponent<MeshFilter>().sharedMesh.vertices[0].x == before.x; i++) yield return null;
            var after = a.GetComponent<MeshFilter>().sharedMesh.vertices[0];
            Assert.AreEqual(-before.x, after.x, 1e-4f);
        }

        // ---------- M16 컴포넌트 물리 ----------
        const string PhysicsJson = @"{""skeleton"":{""bones"":[{""name"":""root""},{""name"":""tail"",""parent"":""root"",""length"":50}],
            ""constraints"":[{""type"":""physics"",""name"":""p"",""bone"":""tail"",""x"":1,""y"":1,""rotate"":1,""gravity"":GRAVITY}],
            ""animations"":[{""name"":""흔들"",""bones"":{""root"":{""rotate"":[{""time"":0,""v"":[0]},{""time"":0.5,""v"":[30]}]}}}]}}";

        static KGBoneSkeletonData PhysicsData(float gravity = 30)
        {
            var data = ScriptableObject.CreateInstance<KGBoneSkeletonData>();
            data.Initialize(PhysicsJson.Replace("GRAVITY", gravity.ToString(System.Globalization.CultureInfo.InvariantCulture)),
                null, new Material[4], new ImageEntry[0], 100, new string[0]);
            return data;
        }

        // 플레이 모드 안에서 만든 캐릭터는 Update가 끼어들지 않도록 한 프레임 안에서 Advance로만 진행한다
        static KGBoneAnimation Spawn(KGBoneSkeletonData data, string name = "물리")
        {
            var a = new GameObject(name).AddComponent<KGBoneAnimation>();
            a.playOnAwake = false;
            a.skeletonData = data;
            return a;
        }

        static WorldBone Tail(KGBoneAnimation a) => a.LastPose.Bones[1];

        [UnityTest]
        public System.Collections.IEnumerator m16_physics_advances_in_play_mode()
        {
            yield return new EnterPlayMode();
            var a = Spawn(PhysicsData());
            a.Advance(0);
            float setupY = Tail(a).Y;
            for (int i = 0; i < 30; i++) a.Advance(1 / 60f);
            float y = Tail(a).Y;
            yield return new ExitPlayMode();
            Assert.Less(y, setupY - 0.01f);
        }

        [Test]
        public void m16_no_physics_in_edit_mode()
        {
            var a = Make(PhysicsData());
            a.Advance(0);
            float setupY = Tail(a).Y;
            for (int i = 0; i < 30; i++) a.Advance(1 / 60f);
            Assert.AreEqual(setupY, Tail(a).Y, 1e-6f);
        }

        [UnityTest]
        public System.Collections.IEnumerator m16_physics_continues_after_stop()
        {
            yield return new EnterPlayMode();
            var a = Spawn(PhysicsData(0));
            a.Play("흔들", false);
            for (int i = 0; i < 20; i++) a.Advance(1 / 60f);
            a.Stop();
            var stopped = Tail(a);
            for (int i = 0; i < 5; i++) a.Advance(1 / 60f);
            var later = Tail(a);
            // 애니메이션은 멈췄지만(셋업 포즈) 흔들림은 계속 움직이다 가라앉는다
            for (int i = 0; i < 600; i++) a.Advance(1 / 60f);
            var settled = Tail(a);
            yield return new ExitPlayMode();
            // 꼬리 원점은 뼈대 원점이라 흔들림은 회전으로 나타난다
            Assert.Greater(Mathf.Abs(Mathf.DeltaAngle(later.RotationX, stopped.RotationX)), 1e-2f);
            Assert.AreEqual(0f, settled.RotationX, 1e-2f);
        }

        [UnityTest]
        public System.Collections.IEnumerator m16_move_factor_zero_ignores_transform_move()
        {
            yield return new EnterPlayMode();
            var data = PhysicsData(0);
            var still = Spawn(data, "제자리");
            var moved = Spawn(data, "이동");
            var reacting = Spawn(data, "반응");
            reacting.physicsMoveFactor = 1;
            reacting.physicsRotateFactor = 1;
            float maxDiff0 = 0, maxDiff1 = 0;
            for (int i = 0; i < 30; i++)
            {
                moved.transform.position += new Vector3(0.2f, 0, 0);
                reacting.transform.position += new Vector3(0.2f, 0, 0);
                reacting.transform.Rotate(0, 0, 2);
                still.Advance(1 / 60f);
                moved.Advance(1 / 60f);
                reacting.Advance(1 / 60f);
                maxDiff0 = Mathf.Max(maxDiff0, Mathf.Abs(Tail(moved).X - Tail(still).X), Mathf.Abs(Tail(moved).Y - Tail(still).Y));
                maxDiff1 = Mathf.Max(maxDiff1, Mathf.Abs(Tail(reacting).X - Tail(still).X), Mathf.Abs(Tail(reacting).Y - Tail(still).Y));
            }
            yield return new ExitPlayMode();
            Assert.AreEqual(0f, maxDiff0, 1e-6f, "반영 0이면 GameObject 이동은 물리에 들어가지 않는다");
            Assert.Greater(maxDiff1, 0.1f, "반영 1이면 뒤처진다");
        }

        [UnityTest]
        public System.Collections.IEnumerator m16_reset_physics_after_teleport_has_no_jump()
        {
            yield return new EnterPlayMode();
            var data = PhysicsData(0);
            var a = Spawn(data);
            a.physicsMoveFactor = 1;
            a.Advance(1 / 60f);
            a.transform.position = new Vector3(50, 0, 0);
            a.ResetPhysics();
            a.Advance(1 / 60f);
            var w = Tail(a);
            yield return new ExitPlayMode();
            Assert.AreEqual(0f, w.X, 1e-6f);
            Assert.AreEqual(0f, w.Y, 1e-6f);
        }

        [UnityTest]
        public System.Collections.IEnumerator m16_large_first_frame_clamped()
        {
            yield return new EnterPlayMode();
            var data = PhysicsData();
            var big = Spawn(data, "큰 프레임");
            var cap = Spawn(data, "0.1초");
            big.Advance(0); cap.Advance(0);
            big.Advance(1f);
            cap.Advance(0.1f);
            var wb = Tail(big);
            var wc = Tail(cap);
            yield return new ExitPlayMode();
            Assert.AreEqual(0.1f, KGBoneAnimation.PhysicsDt(1f));
            Assert.AreEqual(0f, KGBoneAnimation.PhysicsDt(-1f));
            Assert.AreEqual(wc.X, wb.X, 1e-6f);
            Assert.AreEqual(wc.Y, wb.Y, 1e-6f);
            Assert.Less(wc.Y, -1e-3f);
        }

        [Test]
        public void m10_importer_no_constraint_warning()
        {
            // 제약 5종이 든 파일(물리 포함)을 가져와도 제약 경고가 없다
            var dir = ImporterTests.Tmp + "/k5";
            Directory.CreateDirectory(dir);
            ImporterTests.CopySampleImages(ImporterTests.Tmp + "/images");
            File.Copy(Path.Combine(GoldenFile.RepoRoot(), "samples", "kgbone", "unity-stage2-physics.kgbone"), dir + "/phys.kgbone", true);
            var data = ImporterTests.Import(dir + "/phys.kgbone");
            Assert.IsNotNull(data);
            foreach (var w in data.Warnings) StringAssert.DoesNotContain("제약", w);
        }

        [Test]
        public void m12_follower_tracks_bone()
        {
            var a = Make(ImporterTests.ImportStage1());
            a.transform.position = new Vector3(1, 2, 0);
            a.Play("걷기", true);
            a.Advance(0.3f);
            var go = new GameObject("따라가기");
            made.Add(go);
            var f = go.AddComponent<KGBoneBoneFollower>();
            f.target = a;
            f.boneName = "팔-보통";
            f.followRotation = true;
            f.Apply();
            Assert.IsTrue(a.TryGetBone("팔-보통", out Vector3 p, out float r, out _));
            Assert.Less((go.transform.position - p).magnitude, 1e-4f);
            Assert.AreEqual(Mathf.DeltaAngle(0, r), Mathf.DeltaAngle(0, go.transform.eulerAngles.z), 1e-3f);
            // 위치만 따라가기면 회전은 그대로
            f.followRotation = false;
            go.transform.rotation = Quaternion.identity;
            f.Apply();
            Assert.AreEqual(0f, go.transform.eulerAngles.z, 1e-4f);
        }
    }
}
