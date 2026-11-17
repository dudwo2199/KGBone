// .kgbone 가져오기 도구: 파일을 Assets에 넣으면 KGBoneSkeletonData(메인 에셋) + 묶은 텍스처 + 블렌드 모드별 머티리얼을 만든다.
// 이미지는 .kgbone 기준 imagesPath 폴더의 PNG(첨부물이 참조하는 것만). 1단계에서 계산하지 않는 기능은 경고한다.
using System.Collections.Generic;
using System.IO;
using KGBone.Core;
using UnityEditor;
using UnityEditor.AssetImporters;
using UnityEngine;
using UnityEngine.Rendering;

namespace KGBone.Editor
{
    [ScriptedImporter(1, "kgbone")]
    public sealed class KGBoneImporter : ScriptedImporter
    {
        public const string ShaderPath = "Packages/com.kgbone.runtime/Shaders/KGBoneUnlit.shader";

        [Tooltip("KGBone 단위(픽셀) 몇 개가 유니티 1 단위인가")]
        public float pixelsPerUnit = 100;
        [Tooltip("묶은 텍스처의 최대 크기(픽셀). 넘으면 줄인다")]
        public int maxTextureSize = 4096;
        public FilterMode filterMode = FilterMode.Bilinear;

        public override void OnImportAsset(AssetImportContext ctx)
        {
            string text = File.ReadAllText(ctx.assetPath);
            JsonValue root;
            SkeletonModel model;
            try
            {
                root = JsonValue.Parse(text);
                model = KGBoneLoader.Load(text);
            }
            catch (KGBoneFormatException e)
            {
                ctx.LogImportError($"{ctx.assetPath}: {e.Message}");
                return;
            }

            var warnings = new List<string>();
            warnings.AddRange(model.ConstraintWarnings());
            if (model.HasClipping) warnings.Add("클리핑은 1단계 런타임에서 자르지 않고 그립니다");

            // 이미지 폴더(프로젝트 기준 경로로 정리)
            string projectRoot = Path.GetFullPath(".").Replace('\\', '/').TrimEnd('/') + "/";
            string assetDir = Path.GetDirectoryName(ctx.assetPath) ?? "";
            string imagesFull = Path.GetFullPath(Path.Combine(assetDir, model.ImagesPath)).Replace('\\', '/');
            bool inProject = imagesFull.StartsWith(projectRoot, System.StringComparison.OrdinalIgnoreCase);
            if (!inProject) warnings.Add($"이미지 폴더가 프로젝트 밖입니다: {imagesFull}");

            var textures = new List<Texture2D>();
            var paths = new List<string>();
            foreach (string path in KGBoneLoader.ImagePaths(model))
            {
                string file = imagesFull + "/" + path + ".png";
                if (inProject) ctx.DependsOnSourceAsset(file.Substring(projectRoot.Length));
                if (!File.Exists(file))
                {
                    warnings.Add($"이미지를 찾을 수 없습니다: {path}");
                    continue;
                }
                var tex = TexturePacker.LoadPremultiplied(File.ReadAllBytes(file));
                if (tex == null)
                {
                    warnings.Add($"이미지를 읽지 못했습니다: {path}");
                    continue;
                }
                textures.Add(tex);
                paths.Add(path);
            }

            string baseName = Path.GetFileNameWithoutExtension(ctx.assetPath);
            Texture2D atlas = null;
            var entries = new ImageEntry[textures.Count];
            if (textures.Count > 0)
            {
                atlas = TexturePacker.Pack(textures, maxTextureSize, out Rect[] rects, out bool shrunk);
                if (shrunk) warnings.Add($"묶은 텍스처가 {maxTextureSize}px를 넘어 줄였습니다");
                for (int i = 0; i < textures.Count; i++)
                    entries[i] = new ImageEntry { path = paths[i], uv = rects[i], size = new Vector2(textures[i].width, textures[i].height) };
                atlas.name = baseName + " 텍스처";
                atlas.filterMode = filterMode;
                atlas.wrapMode = TextureWrapMode.Clamp;
                atlas.Apply(false, true);
                ctx.AddObjectToAsset("texture", atlas);
            }
            foreach (var t in textures) DestroyImmediate(t);

            var materials = new Material[4];
            ctx.DependsOnSourceAsset(ShaderPath);
            var shader = AssetDatabase.LoadAssetAtPath<Shader>(ShaderPath) ?? Shader.Find("KGBone/Unlit");
            if (shader == null) warnings.Add("셰이더 KGBone/Unlit을 찾을 수 없습니다");
            else
            {
                foreach (var slot in model.Slots)
                {
                    int i = (int)slot.Blend;
                    if (materials[i] != null) continue;
                    var mat = new Material(shader) { name = $"{baseName} {ModelNames.BlendName(slot.Blend)}", mainTexture = atlas };
                    BlendFactors(slot.Blend, out var src, out var dst);
                    mat.SetFloat("_SrcBlend", (float)src);
                    mat.SetFloat("_DstBlend", (float)dst);
                    materials[i] = mat;
                    ctx.AddObjectToAsset("material-" + ModelNames.BlendName(slot.Blend), mat);
                }
            }

            foreach (var w in warnings) ctx.LogImportWarning($"{ctx.assetPath}: {w}");

            var data = ScriptableObject.CreateInstance<KGBoneSkeletonData>();
            data.name = baseName;
            data.Initialize(JsonWriter.WithoutTopLevel(root, "editor"), atlas, materials, entries, pixelsPerUnit, warnings.ToArray());
            ctx.AddObjectToAsset("data", data);
            ctx.SetMainObject(data);
        }

        /// <summary>미리 곱한 알파 기준 블렌드 함수(KGBone 화면 app/src/render/blend.ts와 같음).</summary>
        public static void BlendFactors(Core.BlendMode mode, out UnityEngine.Rendering.BlendMode src, out UnityEngine.Rendering.BlendMode dst)
        {
            switch (mode)
            {
                case Core.BlendMode.Additive: src = UnityEngine.Rendering.BlendMode.One; dst = UnityEngine.Rendering.BlendMode.One; break;
                case Core.BlendMode.Multiply: src = UnityEngine.Rendering.BlendMode.DstColor; dst = UnityEngine.Rendering.BlendMode.OneMinusSrcAlpha; break;
                case Core.BlendMode.Screen: src = UnityEngine.Rendering.BlendMode.One; dst = UnityEngine.Rendering.BlendMode.OneMinusSrcColor; break;
                default: src = UnityEngine.Rendering.BlendMode.One; dst = UnityEngine.Rendering.BlendMode.OneMinusSrcAlpha; break;
            }
        }
    }
}
