// .kgbone을 다시 가져오면 열린 씬에서 그 데이터를 쓰는 캐릭터를 바로 다시 그린다.
// 에디터 모드에서는 KGBoneAnimation.Update가 계산하지 않아, 알려 주지 않으면 예전 그림이 남는다.
using System;
using UnityEditor;
using UnityEngine;

namespace KGBone.Editor
{
    sealed class KGBoneReimportRefresher : AssetPostprocessor
    {
        static void OnPostprocessAllAssets(string[] imported, string[] deleted, string[] moved, string[] movedFrom)
        {
            bool any = false;
            foreach (var path in imported)
                if (path.EndsWith(".kgbone", StringComparison.OrdinalIgnoreCase)) { any = true; break; }
            if (!any) return;

            bool redrew = false;
            foreach (var a in UnityEngine.Object.FindObjectsByType<KGBoneAnimation>(FindObjectsSortMode.None))
            {
                if (a.skeletonData == null) continue;
                if (Array.IndexOf(imported, AssetDatabase.GetAssetPath(a.skeletonData)) < 0) continue;
                a.OnDataReimported();
                redrew = true;
            }
            if (redrew) SceneView.RepaintAll();
        }
    }
}
