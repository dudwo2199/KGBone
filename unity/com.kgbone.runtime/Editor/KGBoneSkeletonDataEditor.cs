// KGBoneSkeletonData 인스펙터: 가져오기 경고를 위에 보여 주고, 본·슬롯·애니메이션 수를 요약한다.
using KGBone.Core;
using UnityEditor;

namespace KGBone.Editor
{
    [CustomEditor(typeof(KGBoneSkeletonData))]
    public sealed class KGBoneSkeletonDataEditor : UnityEditor.Editor
    {
        public override void OnInspectorGUI()
        {
            var data = (KGBoneSkeletonData)target;
            foreach (var w in data.Warnings) EditorGUILayout.HelpBox(w, MessageType.Warning);
            SkeletonModel m = null;
            try
            {
                m = data.Model;
            }
            catch (KGBoneFormatException e)
            {
                EditorGUILayout.HelpBox(e.Message, MessageType.Error);
            }
            if (m != null)
            {
                EditorGUILayout.LabelField("본", m.Bones.Count.ToString());
                EditorGUILayout.LabelField("슬롯", m.Slots.Count.ToString());
                EditorGUILayout.LabelField("스킨", string.Join(", ", m.Skins.ConvertAll(s => s.Name)));
                EditorGUILayout.LabelField("애니메이션", string.Join(", ", m.Animations.ConvertAll(a => a.Name)));
            }
            EditorGUILayout.Space();
            using (new EditorGUI.DisabledScope(true)) DrawDefaultInspector();
        }
    }
}
