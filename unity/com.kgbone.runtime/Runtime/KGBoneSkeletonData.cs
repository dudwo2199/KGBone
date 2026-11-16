// 가져온 .kgbone 한 개(가져오기 도구가 만드는 메인 에셋). 뼈대 JSON(편집 상태 제외), 묶은 텍스처, 블렌드 모드별 머티리얼,
// 이미지 경로 → 텍스처 안 위치 표, 가져오기 경고를 담는다. 모델은 처음 쓸 때 JSON을 읽어 만들고 같은 에셋을 쓰는 캐릭터끼리 공유한다.
using System;
using System.Collections.Generic;
using KGBone.Core;
using UnityEngine;

namespace KGBone
{
    [Serializable]
    public struct ImageEntry
    {
        /// <summary>이미지 경로(확장자 없음, imagesPath 기준, '/' 구분).</summary>
        public string path;
        /// <summary>묶은 텍스처 안 위치(0~1 UV, 유니티 방향: v 위로).</summary>
        public Rect uv;
        /// <summary>원본 픽셀 크기.</summary>
        public Vector2 size;
    }

    public sealed class KGBoneSkeletonData : ScriptableObject
    {
        [SerializeField, HideInInspector] string skeletonJson;
        [SerializeField] Texture2D texture;
        [SerializeField] Material[] materials = new Material[4];
        [SerializeField] ImageEntry[] images = new ImageEntry[0];
        [SerializeField] float pixelsPerUnit = 100;
        [SerializeField] string[] warnings = new string[0];

        [NonSerialized] SkeletonModel model;
        [NonSerialized] Dictionary<string, int> imageIndex;

        /// <summary>편집 상태(editor)를 뺀 .kgbone JSON.</summary>
        public string SkeletonJson => skeletonJson;
        public Texture2D Texture => texture;
        public float PixelsPerUnit => pixelsPerUnit;
        public IReadOnlyList<string> Warnings => warnings ?? new string[0];

        /// <summary>뼈대 모델(처음 쓸 때 읽어 캐시).</summary>
        public SkeletonModel Model => model ?? (model = KGBoneLoader.Load(skeletonJson));

        /// <summary>블렌드 모드의 머티리얼. 그 모드를 쓰는 슬롯이 없으면 null.</summary>
        public Material MaterialFor(BlendMode mode)
        {
            int i = (int)mode;
            return materials != null && i < materials.Length ? materials[i] : null;
        }

        public bool TryGetImage(string path, out Rect uv, out Vector2 sizePx)
        {
            if (imageIndex == null)
            {
                imageIndex = new Dictionary<string, int>(StringComparer.Ordinal);
                for (int i = 0; i < images.Length; i++) imageIndex[images[i].path] = i;
            }
            if (path != null && imageIndex.TryGetValue(path, out int k))
            {
                uv = images[k].uv;
                sizePx = images[k].size;
                return true;
            }
            uv = default;
            sizePx = default;
            return false;
        }

        // 다시 가져오거나 인스펙터에서 바뀌면 캐시를 버린다
        void OnEnable() => ClearCache();
        void OnValidate() => ClearCache();

        void ClearCache()
        {
            model = null;
            imageIndex = null;
        }

        /// <summary>가져오기 도구가 채운다.</summary>
        internal void Initialize(string json, Texture2D tex, Material[] mats, ImageEntry[] entries, float ppu, string[] warns)
        {
            skeletonJson = json;
            texture = tex;
            materials = mats;
            images = entries;
            pixelsPerUnit = ppu;
            warnings = warns;
            model = null;
            imageIndex = null;
        }
    }
}
