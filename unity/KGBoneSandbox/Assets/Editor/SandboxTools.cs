// 확인용 프로젝트 도구: URP 설정 만들기, Assets/Samples의 .kgbone마다 캐릭터를 놓은 장면 만들기, 화면을 PNG로 찍기.
// 메뉴: KGBone > 확인 장면 만들기 / 화면 찍기
// 배치 모드: Unity.exe -batchmode -projectPath unity/KGBoneSandbox -executeMethod KGBoneSandbox.SandboxTools.CaptureBatch -logFile -
//   (그래픽 장치가 필요하므로 -nographics를 붙이지 않는다) → Logs/capture-<시점>.png
using System.Collections.Generic;
using System.IO;
using KGBone;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.Rendering;
using UnityEngine.Rendering.Universal;

namespace KGBoneSandbox
{
    public static class SandboxTools
    {
        const string ScenePath = "Assets/Scenes/Sandbox.unity";
        const float Spacing = 4f;

        static void EnsurePipeline()
        {
            if (GraphicsSettings.defaultRenderPipeline is UniversalRenderPipelineAsset) return;
            Directory.CreateDirectory("Assets/Settings");
            var rendererData = ScriptableObject.CreateInstance<UniversalRendererData>();
            AssetDatabase.CreateAsset(rendererData, "Assets/Settings/KGBoneRenderer.asset");
            var urp = UniversalRenderPipelineAsset.Create(rendererData);
            AssetDatabase.CreateAsset(urp, "Assets/Settings/KGBoneURP.asset");
            GraphicsSettings.defaultRenderPipeline = urp;
            QualitySettings.renderPipeline = urp;
            AssetDatabase.SaveAssets();
        }

        [MenuItem("KGBone/확인 장면 만들기")]
        public static void BuildScene()
        {
            EnsurePipeline();
            AssetDatabase.Refresh(ImportAssetOptions.ForceSynchronousImport);
            var scene = EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
            var cam = new GameObject("Main Camera").AddComponent<Camera>();
            cam.tag = "MainCamera";
            cam.orthographic = true;
            cam.clearFlags = CameraClearFlags.SolidColor;
            cam.backgroundColor = new Color(0.16f, 0.16f, 0.18f);

            var paths = new List<string>();
            foreach (var guid in AssetDatabase.FindAssets("t:KGBoneSkeletonData", new[] { "Assets/Samples" }))
                paths.Add(AssetDatabase.GUIDToAssetPath(guid));
            paths.Sort(System.StringComparer.Ordinal);
            for (int i = 0; i < paths.Count; i++)
            {
                var data = AssetDatabase.LoadAssetAtPath<KGBoneSkeletonData>(paths[i]);
                var go = new GameObject(Path.GetFileNameWithoutExtension(paths[i]));
                go.transform.position = new Vector3(i * Spacing, 0, 0);
                var anim = go.AddComponent<KGBoneAnimation>();
                anim.skeletonData = data;
                anim.animationName = data.Model.Animations.Count > 0 ? data.Model.Animations[0].Name : null;
            }
            cam.transform.position = new Vector3((paths.Count - 1) * Spacing * 0.5f, 1.2f, -10f);
            cam.orthographicSize = 2.6f;
            Directory.CreateDirectory("Assets/Scenes");
            EditorSceneManager.SaveScene(scene, ScenePath);
            Debug.Log($"[KGBone] 확인 장면: 캐릭터 {paths.Count}개 → {ScenePath}");
        }

        [MenuItem("KGBone/화면 찍기")]
        public static void Capture()
        {
            BuildScene();
            var cam = Camera.main;
            var anims = Object.FindObjectsByType<KGBoneAnimation>(FindObjectsSortMode.InstanceID);
            int width = Mathf.Max(800, 400 * anims.Length), height = 520;
            cam.aspect = (float)width / height;
            cam.orthographicSize = Mathf.Max(2.6f, anims.Length * Spacing * 0.5f / cam.aspect);
            foreach (float t in new[] { 0f, 0.25f, 0.5f, 0.75f })
            {
                foreach (var a in anims)
                {
                    if (!string.IsNullOrEmpty(a.animationName)) a.Play(a.animationName, true);
                    a.Advance(t);
                }
                var rt = new RenderTexture(width, height, 24, RenderTextureFormat.ARGB32);
                cam.targetTexture = rt;
                cam.Render();
                RenderTexture.active = rt;
                var tex = new Texture2D(width, height, TextureFormat.RGBA32, false);
                tex.ReadPixels(new Rect(0, 0, width, height), 0, 0);
                tex.Apply();
                string file = Path.Combine("Logs", $"capture-{t:0.00}.png");
                File.WriteAllBytes(file, tex.EncodeToPNG());
                cam.targetTexture = null;
                RenderTexture.active = null;
                Object.DestroyImmediate(rt);
                Object.DestroyImmediate(tex);
                Debug.Log($"[KGBone] 찍음: {Path.GetFullPath(file)}");
            }
        }

        /// <summary>배치 모드용: 찍고 끝낸다.</summary>
        public static void CaptureBatch()
        {
            try
            {
                Capture();
                EditorApplication.Exit(0);
            }
            catch (System.Exception e)
            {
                Debug.LogException(e);
                EditorApplication.Exit(1);
            }
        }
    }
}
