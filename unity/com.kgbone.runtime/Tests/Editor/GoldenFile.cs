// 테스트가 저장소 파일(시험 파일, 정답 파일)을 찾는 도우미. dotnet test와 유니티 EditMode 양쪽에서 쓴다(순수 C#).
using System.IO;
using NUnit.Framework;

namespace KGBone.Tests
{
    public static class GoldenFile
    {
        /// <summary>저장소 루트: 테스트 폴더에서 위로 올라가며 .git(폴더 또는 워크트리의 파일)이 있는 곳.</summary>
        public static string RepoRoot()
        {
            var dir = new DirectoryInfo(TestContext.CurrentContext.TestDirectory);
            while (dir != null)
            {
                string git = Path.Combine(dir.FullName, ".git");
                if (Directory.Exists(git) || File.Exists(git)) return dir.FullName;
                dir = dir.Parent;
            }
            throw new DirectoryNotFoundException("저장소 루트(.git)를 찾을 수 없습니다: " + TestContext.CurrentContext.TestDirectory);
        }
    }
}
