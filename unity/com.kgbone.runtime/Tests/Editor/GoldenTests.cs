// M07: 유니티 런타임 계산부의 결과가 KGBone 핵심(Rust)과 같은지, 정답 파일(tools/unity-golden.mjs가 생성)과 비교한다.
// dotnet test와 유니티 EditMode 양쪽에서 돈다(순수 C#, NUnit).
using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using KGBone.Core;
using NUnit.Framework;

namespace KGBone.Tests
{
    public class GoldenTests
    {
        const float Tol = 1e-3f;

        public sealed class GoldenCase
        {
            public string File;
            public int Index;
            public JsonValue Request;
            public JsonValue Output;
            public string KgbonePath;
            public override string ToString()
            {
                string anim = Request.Get("animation")?.AsString() ?? "셋업";
                string skin = Request.Get("skin")?.AsString();
                string t = Request.Get("time").AsFloat().ToString("0.###", CultureInfo.InvariantCulture);
                return $"{File}/{Index}/{anim}@{t}{(skin != null ? "/" + skin : "")}";
            }
        }

        public static IEnumerable<TestCaseData> Cases()
        {
            foreach (var path in GoldenFile.All())
            {
                var root = JsonValue.Parse(System.IO.File.ReadAllText(path));
                string kgbone = Path.Combine(Path.GetDirectoryName(path), root.Get("kgbone").AsString());
                var cases = root.Get("cases").Items;
                for (int i = 0; i < cases.Count; i++)
                {
                    var c = new GoldenCase
                    {
                        File = Path.GetFileNameWithoutExtension(path),
                        Index = i,
                        Request = cases[i].Get("request"),
                        Output = cases[i].Get("output"),
                        KgbonePath = kgbone,
                    };
                    yield return new TestCaseData(c).SetName("m07_golden_" + c);
                }
            }
        }

        static readonly Dictionary<string, SkeletonModel> Models = new Dictionary<string, SkeletonModel>();

        [TestCaseSource(nameof(Cases))]
        public void m07_all_golden_cases_match_core(GoldenCase c)
        {
            if (!Models.TryGetValue(c.KgbonePath, out var model))
                Models[c.KgbonePath] = model = KGBoneLoader.Load(System.IO.File.ReadAllText(c.KgbonePath));
            var r = c.Request;
            var lastTime = r.Get("lastTime");
            var req = new PoseRequest
            {
                Animation = r.Get("animation")?.AsString(),
                Time = r.Get("time").AsFloat(),
                LastTime = lastTime == null || lastTime.Kind == JsonKind.Null ? (float?)null : lastTime.AsFloat(),
                Skin = r.Get("skin")?.AsString(),
                Looping = r.Get("looping").AsBool(),
            };
            var got = new SkeletonPoser(model).Compute(req);
            Compare(c.Output, got);
        }

        static void Near(float want, float got, string path)
        {
            if (Math.Abs(want - got) > Tol || float.IsNaN(got))
                Assert.Fail($"{path}: 정답 {want.ToString("R", CultureInfo.InvariantCulture)}, 계산 {got.ToString("R", CultureInfo.InvariantCulture)}");
        }

        static void Same<T>(T want, T got, string path)
        {
            if (!EqualityComparer<T>.Default.Equals(want, got)) Assert.Fail($"{path}: 정답 [{want}], 계산 [{got}]");
        }

        static void NearList(JsonValue want, IReadOnlyList<float> got, string path)
        {
            Same(want.Items.Count, got.Count, path + ".length");
            for (int i = 0; i < got.Count; i++) Near(want.Items[i].AsFloat(), got[i], $"{path}[{i}]");
        }

        static void Compare(JsonValue want, PoseResult got)
        {
            var bones = want.Get("bones").Items;
            Same(bones.Count, got.Bones.Length, "bones.length");
            for (int i = 0; i < bones.Count; i++)
            {
                var w = bones[i];
                var g = got.Bones[i];
                Near(w.Get("a").AsFloat(), g.A, $"bones[{i}].a");
                Near(w.Get("b").AsFloat(), g.B, $"bones[{i}].b");
                Near(w.Get("c").AsFloat(), g.C, $"bones[{i}].c");
                Near(w.Get("d").AsFloat(), g.D, $"bones[{i}].d");
                Near(w.Get("x").AsFloat(), g.X, $"bones[{i}].x");
                Near(w.Get("y").AsFloat(), g.Y, $"bones[{i}].y");
            }
            var active = want.Get("active").Items;
            for (int i = 0; i < active.Count; i++) Same(active[i].AsBool(), got.Active[i], $"active[{i}]");

            var draw = want.Get("draw").Items;
            Same(draw.Count, got.Draw.Count, "draw.length");
            for (int i = 0; i < draw.Count; i++)
            {
                var w = draw[i];
                var g = got.Draw[i];
                string p = $"draw[{i}]";
                Same(w.Get("slot").AsString(), g.Slot, p + ".slot");
                Same(w.Get("attachment").AsString(), g.Attachment, p + ".attachment");
                Same(w.Get("image").AsString(), g.Image, p + ".image");
                Same(w.Get("blend").AsString(), ModelNames.BlendName(g.Blend), p + ".blend");
                NearList(w.Get("color"), new[] { g.Color.R, g.Color.G, g.Color.B, g.Color.A }, p + ".color");
                var dark = w.Get("dark");
                bool wantDark = dark != null && dark.Kind != JsonKind.Null;
                Same(wantDark, g.Dark.HasValue, p + ".dark(있음)");
                if (wantDark) NearList(dark, new[] { g.Dark.Value.R, g.Dark.Value.G, g.Dark.Value.B }, p + ".dark");
                NearList(w.Get("positions"), g.Positions, p + ".positions");
                NearList(w.Get("uvs"), g.Uvs, p + ".uvs");
                var idx = w.Get("indices").Items;
                Same(idx.Count, g.Indices.Count, p + ".indices.length");
                for (int j = 0; j < idx.Count; j++) Same(idx[j].AsInt(), g.Indices[j], $"{p}.indices[{j}]");
            }

            var events = want.Get("events").Items;
            Same(events.Count, got.Events.Count, "events.length");
            for (int i = 0; i < events.Count; i++)
            {
                var w = events[i];
                var g = got.Events[i];
                string p = $"events[{i}]";
                Same(w.Get("name").AsString(), g.Name, p + ".name");
                Near(w.Get("time").AsFloat(), g.Time, p + ".time");
                Same(w.Get("int").AsInt(), g.Int, p + ".int");
                Near(w.Get("float").AsFloat(), g.Float, p + ".float");
                Same(w.Get("string").AsString(), g.String, p + ".string");
                Near(w.Get("volume").AsFloat(), g.Volume, p + ".volume");
                Near(w.Get("balance").AsFloat(), g.Balance, p + ".balance");
            }
        }
    }
}
