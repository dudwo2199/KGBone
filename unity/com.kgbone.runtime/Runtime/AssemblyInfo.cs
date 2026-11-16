// 가져오기 도구(KGBone.Editor)와 테스트만 데이터 에셋의 내부 초기화 함수를 쓸 수 있게 한다.
using System.Runtime.CompilerServices;

[assembly: InternalsVisibleTo("KGBone.Editor")]
[assembly: InternalsVisibleTo("KGBone.Tests")]
