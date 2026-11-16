// 파츠 PNG들을 한 장의 텍스처로 묶는다. 화면(KGBone 웹 판)과 같은 색 공식을 쓰기 위해 알파를 미리 곱한다.
using System.Collections.Generic;
using UnityEngine;

namespace KGBone.Editor
{
    static class TexturePacker
    {
        public const int Padding = 2;

        /// <summary>PNG 바이트 → 알파를 미리 곱한 텍스처(읽기 가능). 읽지 못하면 null.</summary>
        public static Texture2D LoadPremultiplied(byte[] png)
        {
            var tex = new Texture2D(2, 2, TextureFormat.RGBA32, false);
            if (!ImageConversion.LoadImage(tex, png, false))
            {
                Object.DestroyImmediate(tex);
                return null;
            }
            var px = tex.GetPixels32();
            for (int i = 0; i < px.Length; i++)
            {
                var c = px[i];
                px[i] = new Color32((byte)((c.r * c.a + 127) / 255), (byte)((c.g * c.a + 127) / 255), (byte)((c.b * c.a + 127) / 255), c.a);
            }
            tex.SetPixels32(px);
            tex.Apply(false, false);
            return tex;
        }

        /// <summary>
        /// 텍스처들을 묶는다. 결과의 UV 사각형은 sources와 같은 순서. maxSize를 넘어 줄어들었으면 shrunk = true.
        /// </summary>
        public static Texture2D Pack(IList<Texture2D> sources, int maxSize, out Rect[] rects, out bool shrunk)
        {
            var atlas = new Texture2D(2, 2, TextureFormat.RGBA32, false);
            var arr = new Texture2D[sources.Count];
            sources.CopyTo(arr, 0);
            rects = atlas.PackTextures(arr, Padding, maxSize, false);
            shrunk = false;
            for (int i = 0; i < arr.Length; i++)
            {
                // 줄어들지 않았다면 텍스처 안 크기 = 원본 크기(반올림 오차 1px 허용)
                if (rects[i].width * atlas.width < arr[i].width - 1 || rects[i].height * atlas.height < arr[i].height - 1) shrunk = true;
            }
            return atlas;
        }
    }
}
