// M10: .kgbone 가져오기 도구(유니티 EditMode 전용).
using System.IO;
using KGBone;
using KGBone.Core;
using NUnit.Framework;
using UnityEditor;
using UnityEngine;

namespace KGBone.Tests
{
    public class ImporterTests
    {
        public const string Tmp = "Assets/KGBoneTestTmp";

        [SetUp]
        public void SetUp()
        {
            if (Directory.Exists(Tmp)) AssetDatabase.DeleteAsset(Tmp);
            Directory.CreateDirectory(Tmp);
        }

        [TearDown]
        public void TearDown() => AssetDatabase.DeleteAsset(Tmp);

        static string Repo => GoldenFile.RepoRoot();

        public static void CopySampleImages(string destDir)
        {
            Directory.CreateDirectory(destDir);
            foreach (var png in Directory.GetFiles(Path.Combine(Repo, "samples", "images"), "*.png"))
                File.Copy(png, Path.Combine(destDir, Path.GetFileName(png)), true);
        }

        public static KGBoneSkeletonData Import(string assetPath)
        {
            AssetDatabase.Refresh(ImportAssetOptions.ForceSynchronousImport);
            AssetDatabase.ImportAsset(assetPath, ImportAssetOptions.ForceSynchronousImport | ImportAssetOptions.ForceUpdate);
            return AssetDatabase.LoadAssetAtPath<KGBoneSkeletonData>(assetPath);
        }

        /// <summary>unity-stage1 시험 파일(imagesPath "../images")을 임시 폴더에 놓고 가져온다.</summary>
        public static KGBoneSkeletonData ImportStage1()
        {
            CopySampleImages(Path.Combine(Tmp, "images"));
            Directory.CreateDirectory(Path.Combine(Tmp, "kg"));
            File.Copy(Path.Combine(GoldenFile.GoldenDir(), "unity-stage1.kgbone"), Path.Combine(Tmp, "kg", "unity-stage1.kgbone"), true);
            return Import(Tmp + "/kg/unity-stage1.kgbone");
        }

        [Test]
        public void m10_import_creates_data_texture_materials()
        {
            var data = ImportStage1();
            Assert.IsNotNull(data, "메인 에셋이 KGBoneSkeletonData가 아님");
            Assert.IsNotNull(data.Texture);
            foreach (BlendMode m in System.Enum.GetValues(typeof(BlendMode)))
                Assert.IsNotNull(data.MaterialFor(m), "머티리얼 없음: " + m);
            Assert.AreEqual(13, data.Model.Bones.Count);
            CollectionAssert.IsEmpty(data.Warnings);
            StringAssert.DoesNotContain("\"editor\"", data.SkeletonJson);
            Assert.IsTrue(data.TryGetImage("head", out Rect uv, out Vector2 size));
            Assert.AreEqual(new Vector2(64, 64), size);
            Assert.Greater(uv.width, 0f);
            Assert.AreEqual(100f, data.PixelsPerUnit);
        }

        [Test]
        public void m10_atlas_holds_raw_values_for_color_space_conversion_in_shader()
        {
            // 검토 I4: 선형 색 공간 프로젝트에서 텍스처가 sRGB로 풀려 화면(감마 공간 계산)과 색이 달라지던 문제.
            // 텍스처는 원래 값 그대로(선형 표시) 두고, 셰이더가 화면과 같은 계산을 한 뒤 색 공간에 맞게 바꾼다
            var data = ImportStage1();
            Assert.IsFalse(data.Texture.isDataSRGB);
        }

        [Test]
        public void m10_missing_and_korean_subfolder_images()
        {
            string dir = Path.Combine(Tmp, "k2");
            Directory.CreateDirectory(Path.Combine(dir, "images", "팔"));
            File.Copy(Path.Combine(Repo, "samples", "images", "gem.png"), Path.Combine(dir, "images", "팔", "왼쪽.png"), true);
            File.WriteAllText(Path.Combine(dir, "test.kgbone"), @"{""formatVersion"":1,""imagesPath"":""./images"",""skeleton"":{
                ""bones"":[{""name"":""root""}],
                ""slots"":[{""name"":""a"",""bone"":""root"",""attachment"":""x""},{""name"":""b"",""bone"":""root"",""attachment"":""없는그림""}],
                ""skins"":[{""name"":""default"",""attachments"":{""a"":{""x"":{""path"":""팔/왼쪽"",""width"":10,""height"":10}},""b"":{""없는그림"":{}}}}]}}");
            var data = Import(Tmp + "/k2/test.kgbone");
            Assert.IsNotNull(data);
            CollectionAssert.AreEqual(new[] { "이미지를 찾을 수 없습니다: 없는그림" }, data.Warnings);
            Assert.IsTrue(data.TryGetImage("팔/왼쪽", out _, out Vector2 size));
            Assert.AreEqual(new Vector2(32, 32), size);
            Assert.IsFalse(data.TryGetImage("없는그림", out _, out _));
        }

        [Test]
        public void m10_unsupported_features_warn()
        {
            CopySampleImages(Path.Combine(Tmp, "images"));
            Directory.CreateDirectory(Path.Combine(Tmp, "k3"));
            File.Copy(Path.Combine(Repo, "samples", "kgbone", "full-4.3.kgbone"), Path.Combine(Tmp, "k3", "full.kgbone"), true);
            var data = Import(Tmp + "/k3/full.kgbone");
            Assert.IsNotNull(data);
            // 2단계: 제약 5종을 모두 계산하므로 제약 경고는 없다(클리핑 등 다른 경고는 그대로)
            foreach (var w in data.Warnings) StringAssert.DoesNotContain("제약", w);
        }
    }
}
