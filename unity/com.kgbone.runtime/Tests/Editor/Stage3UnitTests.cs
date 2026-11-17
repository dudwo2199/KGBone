// 유니티 런타임 3단계 손 계산 테스트(시퀀스·클리핑·트랙·스킨 합치기·덮어쓰기). dotnet test와 유니티 EditMode 양쪽에서 돈다(순수 C#, NUnit).
using System.Collections.Generic;
using System.Linq;
using KGBone.Core;
using NUnit.Framework;

namespace KGBone.Tests
{
    public class Stage3UnitTests
    {
        static Key[] SeqKey(string mode, float start, float delay, float time = 0) =>
            new[] { new Key { Time = time, S = mode, V = new[] { start, delay } } };

        // ---------- M17 시퀀스 ----------
        [Test]
        public void m17_sequence_frame_modes()
        {
            int F(string mode, float t) => SequenceKeys.Frame(SeqKey(mode, 0, 0.1f), t, 4).Value;
            Assert.AreEqual(0, F("hold", 0.25f));
            Assert.AreEqual(2, F("once", 0.25f));
            Assert.AreEqual(2, F("loop", 0.25f));
            Assert.AreEqual(2, F("pingpong", 0.25f));
            Assert.AreEqual(1, F("onceReverse", 0.25f));
            Assert.AreEqual(1, F("loopReverse", 0.25f));
            Assert.AreEqual(1, F("pingpongReverse", 0.25f));
            Assert.AreEqual(3, F("once", 0.55f));
            Assert.AreEqual(1, F("loop", 0.55f));
            Assert.AreEqual(1, F("pingpong", 0.55f));
        }

        [Test]
        public void m17_sequence_delay_zero_holds_start()
        {
            Assert.AreEqual(2, SequenceKeys.Frame(SeqKey("loop", 2, 0), 0.7f, 4).Value);
            Assert.IsNull(SequenceKeys.Frame(new Key[0], 0.7f, 4));
        }

        [Test]
        public void m17_image_path_digits()
        {
            var r = new RegionAttachment { Path = "base", Sequence = new SequenceData { Count = 5, Start = 1, Digits = 2 } };
            Assert.AreEqual("base04", r.ImagePath("x", 3));
            r.Sequence.Digits = 0;
            Assert.AreEqual("base4", r.ImagePath("x", 3));
            Assert.AreEqual("base1", r.ImagePath("x", null));
        }

        [Test]
        public void m17_image_paths_include_all_frames()
        {
            var m = KGBoneLoader.Load(@"{""formatVersion"":1,""skeleton"":{""bones"":[{""name"":""root""}],""slots"":[{""name"":""s"",""bone"":""root"",""attachment"":""a""}],
                ""skins"":[{""name"":""default"",""attachments"":{""s"":{""a"":{""path"":""f"",""sequence"":{""count"":4,""start"":1}}}}}]}}");
            CollectionAssert.AreEqual(new[] { "f1", "f2", "f3", "f4" }, KGBoneLoader.ImagePaths(m).ToArray());
        }
    }
}
