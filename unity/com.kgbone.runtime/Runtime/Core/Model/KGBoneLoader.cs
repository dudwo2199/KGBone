// .kgbone 텍스트 → SkeletonModel. 생략된 필드는 FORMAT.md의 기본값. 모르는 필드는 무시한다(런타임은 다시 쓰지 않는다).
using System;
using System.Collections.Generic;

namespace KGBone.Core
{
    public static class KGBoneLoader
    {
        public static SkeletonModel Load(string kgboneText)
        {
            var root = JsonValue.Parse(kgboneText);
            if (root.Kind != JsonKind.Object) throw new KGBoneFormatException("최상위가 객체가 아닙니다");
            var sk = root.Get("skeleton");
            if (sk == null || sk.Kind != JsonKind.Object) throw new KGBoneFormatException("skeleton이 없습니다");

            var m = new SkeletonModel
            {
                ImagesPath = root.Get("imagesPath")?.AsString("./images") ?? "./images",
                Fps = sk.Get("fps")?.AsFloat(30) ?? 30,
            };
            foreach (var b in Items(sk.Get("bones"))) m.Bones.Add(ReadBone(b, m));
            foreach (var s in Items(sk.Get("slots"))) m.Slots.Add(ReadSlot(s, m));
            m.ConstraintCount = Items(sk.Get("constraints")).Count;
            // formatVersion 0(초기 시험판): 제약이 종류별 배열로 따로 있다(FORMAT.md 7장).
            foreach (var legacy in new[] { "ik", "transform", "path", "physics" }) m.ConstraintCount += Items(sk.Get(legacy)).Count;
            foreach (var s in Items(sk.Get("skins"))) m.Skins.Add(ReadSkin(s, m));
            foreach (var e in Items(sk.Get("events"))) m.Events.Add(ReadEvent(e));
            foreach (var a in Items(sk.Get("animations"))) m.Animations.Add(ReadAnimation(a, m));
            return m;
        }

        /// <summary>모든 스킨의 그림 첨부물이 쓰는 이미지 경로(중복 없음, 처음 나온 순서).</summary>
        public static IEnumerable<string> ImagePaths(SkeletonModel m)
        {
            var seen = new HashSet<string>(StringComparer.Ordinal);
            foreach (var skin in m.Skins)
                foreach (var (_, name, att) in skin.All())
                {
                    string p = att.ImagePath(name);
                    if (p != null && seen.Add(p)) yield return p;
                }
        }

        static readonly IReadOnlyList<JsonValue> Empty = new JsonValue[0];
        static IReadOnlyList<JsonValue> Items(JsonValue v) => v != null && v.Kind == JsonKind.Array ? v.Items : Empty;
        static float F(JsonValue o, string k, float d) => o.Get(k)?.AsFloat(d) ?? d;
        static string S(JsonValue o, string k) => o.Get(k)?.AsString();
        static bool B(JsonValue o, string k, bool d) => o.Get(k)?.AsBool(d) ?? d;

        static BoneData ReadBone(JsonValue v, SkeletonModel m)
        {
            var b = new BoneData
            {
                Name = S(v, "name") ?? throw new KGBoneFormatException("이름 없는 본이 있습니다"),
                Length = F(v, "length", 0),
                X = F(v, "x", 0),
                Y = F(v, "y", 0),
                Rotation = F(v, "rotation", 0),
                ScaleX = F(v, "scaleX", 1),
                ScaleY = F(v, "scaleY", 1),
                ShearX = F(v, "shearX", 0),
                ShearY = F(v, "shearY", 0),
                Inherit = ModelNames.ParseInherit(S(v, "inherit")) ?? Inherit.Normal,
                SkinRequired = B(v, "skinRequired", false),
            };
            string parent = S(v, "parent");
            // 부모는 자기보다 앞에 있을 때만 인정한다(핵심 parent_indices와 같음)
            if (parent != null) b.Parent = m.BoneIndex(parent);
            return b;
        }

        static SlotData ReadSlot(JsonValue v, SkeletonModel m)
        {
            string dark = S(v, "dark");
            return new SlotData
            {
                Name = S(v, "name") ?? throw new KGBoneFormatException("이름 없는 슬롯이 있습니다"),
                Bone = m.BoneIndex(S(v, "bone")),
                Color = ColorF.Parse(S(v, "color"), ColorF.White),
                Dark = dark == null ? (ColorF?)null : ColorF.Parse(dark, new ColorF(0, 0, 0, 1)),
                Attachment = S(v, "attachment"),
                Blend = ModelNames.ParseBlend(S(v, "blend")),
            };
        }

        static SkinData ReadSkin(JsonValue v, SkeletonModel m)
        {
            var skin = new SkinData { Name = S(v, "name") ?? SkinData.DefaultName };
            foreach (var b in Items(v.Get("bones"))) if (b.AsString() != null) skin.Bones.Add(b.AsString());
            var atts = v.Get("attachments");
            if (atts == null) return skin;
            foreach (var slot in atts.Members)
            {
                var map = new Dictionary<string, AttachmentData>(StringComparer.Ordinal);
                foreach (var a in slot.Value.Members)
                {
                    var att = ReadAttachment(a.Value);
                    if (att is OtherAttachment o && o.Type == "clipping") m.HasClipping = true;
                    if (att.Sequence != null) m.HasSequence = true;
                    map[a.Key] = att;
                }
                skin.Attachments[slot.Key] = map;
            }
            return skin;
        }

        static AttachmentData ReadAttachment(JsonValue v)
        {
            string type = S(v, "type") ?? "region";
            AttachmentData a;
            switch (type)
            {
                case "region":
                    a = new RegionAttachment
                    {
                        X = F(v, "x", 0), Y = F(v, "y", 0), Rotation = F(v, "rotation", 0),
                        ScaleX = F(v, "scaleX", 1), ScaleY = F(v, "scaleY", 1),
                        Width = F(v, "width", 32), Height = F(v, "height", 32),
                        Color = ColorF.Parse(S(v, "color"), ColorF.White),
                    };
                    break;
                case "mesh":
                    var tri = Items(v.Get("triangles"));
                    var ints = new int[tri.Count];
                    for (int i = 0; i < ints.Length; i++) ints[i] = tri[i].AsInt();
                    a = new MeshAttachment
                    {
                        Width = F(v, "width", 32), Height = F(v, "height", 32),
                        Color = ColorF.Parse(S(v, "color"), ColorF.White),
                        Uvs = v.Get("uvs")?.AsFloatArray() ?? new float[0],
                        Triangles = ints,
                        Vertices = v.Get("vertices")?.AsFloatArray() ?? new float[0],
                        Weighted = B(v, "weighted", false),
                    };
                    break;
                case "linkedmesh":
                    a = new LinkedMeshAttachment
                    {
                        Color = ColorF.Parse(S(v, "color"), ColorF.White),
                        Parent = S(v, "parent"),
                        Skin = S(v, "skin"),
                        Timelines = B(v, "timelines", true),
                    };
                    break;
                default:
                    a = new OtherAttachment { Type = type };
                    break;
            }
            a.Path = S(v, "path");
            var seq = v.Get("sequence");
            if (seq != null && seq.Kind == JsonKind.Object && !(a is OtherAttachment))
                a.Sequence = new SequenceData
                {
                    Count = seq.Get("count")?.AsInt(1) ?? 1,
                    Start = seq.Get("start")?.AsInt(1) ?? 1,
                    Digits = seq.Get("digits")?.AsInt(0) ?? 0,
                    Setup = seq.Get("setup")?.AsInt(0) ?? 0,
                };
            return a;
        }

        static EventData ReadEvent(JsonValue v) => new EventData
        {
            Name = S(v, "name"),
            Int = v.Get("int")?.AsInt(0) ?? 0,
            Float = F(v, "float", 0),
            String = S(v, "string") ?? "",
            Volume = F(v, "volume", 1),
            Balance = F(v, "balance", 0),
        };

        static Key ReadKey(JsonValue v)
        {
            var k = new Key
            {
                Time = F(v, "time", 0),
                V = v.Get("v")?.AsFloatArray() ?? new float[0],
                S = S(v, "s"),
                Offset = v.Get("offset")?.AsInt(0) ?? 0,
            };
            var c = v.Get("curve");
            if (c == null || c.Kind == JsonKind.Null) k.Curve = CurveType.Linear;
            else if (c.Kind == JsonKind.String)
            {
                string cs = c.AsString();
                if (cs == "stepped") k.Curve = CurveType.Stepped;
                else if (cs == "linear") k.Curve = CurveType.Linear;
                else throw new KGBoneFormatException("curve는 \"stepped\" 또는 숫자 배열이어야 합니다");
            }
            else if (c.Kind == JsonKind.Array) { k.Curve = CurveType.Bezier; k.Bezier = c.AsFloatArray(); }
            else throw new KGBoneFormatException("curve는 \"stepped\" 또는 숫자 배열이어야 합니다");
            return k;
        }

        static Key[] ReadKeys(JsonValue arr, ref float duration)
        {
            var list = Items(arr);
            var keys = new Key[list.Count];
            for (int i = 0; i < keys.Length; i++) keys[i] = ReadKey(list[i]);
            if (keys.Length > 0) duration = Math.Max(duration, keys[keys.Length - 1].Time);
            return keys;
        }

        static AnimationData ReadAnimation(JsonValue v, SkeletonModel m)
        {
            var a = new AnimationData { Name = S(v, "name") };
            float d = 0;
            foreach (var bone in v.Get("bones")?.Members ?? new KeyValuePair<string, JsonValue>[0])
            {
                int bi = m.BoneIndex(bone.Key);
                foreach (var tl in bone.Value.Members)
                {
                    var keys = ReadKeys(tl.Value, ref d);
                    if (bi >= 0) a.Bones.Add(new BoneTimeline { Bone = bi, Name = tl.Key, Keys = keys });
                }
            }
            foreach (var slot in v.Get("slots")?.Members ?? new KeyValuePair<string, JsonValue>[0])
            {
                int si = m.SlotIndex(slot.Key);
                foreach (var tl in slot.Value.Members)
                {
                    var keys = ReadKeys(tl.Value, ref d);
                    if (si >= 0) a.Slots.Add(new SlotTimeline { Slot = si, Name = tl.Key, Keys = keys });
                }
            }
            // 제약 타임라인은 1단계에서 쓰지 않지만 길이에는 들어간다
            foreach (var c in v.Get("constraints")?.Members ?? new KeyValuePair<string, JsonValue>[0])
                foreach (var tl in c.Value.Members) ReadKeys(tl.Value, ref d);
            foreach (var skin in v.Get("deform")?.Members ?? new KeyValuePair<string, JsonValue>[0])
                foreach (var slot in skin.Value.Members)
                    foreach (var att in slot.Value.Members)
                        a.Deform.Add(new DeformTimeline { Skin = skin.Key, Slot = slot.Key, Attachment = att.Key, Keys = ReadKeys(att.Value, ref d) });
            foreach (var skin in v.Get("sequence")?.Members ?? new KeyValuePair<string, JsonValue>[0])
                foreach (var slot in skin.Value.Members)
                    foreach (var att in slot.Value.Members) ReadKeys(att.Value, ref d);
            foreach (var k in Items(v.Get("drawOrder")))
            {
                var dk = new DrawOrderKey { Time = F(k, "time", 0) };
                foreach (var o in Items(k.Get("offsets"))) dk.Offsets.Add((S(o, "slot") ?? "", o.Get("offset")?.AsInt(0) ?? 0));
                a.DrawOrder.Add(dk);
                d = Math.Max(d, dk.Time);
            }
            foreach (var k in Items(v.Get("events")))
            {
                var ek = new EventKey
                {
                    Time = F(k, "time", 0),
                    Name = S(k, "name") ?? "",
                    Int = k.Get("int") == null ? (int?)null : k.Get("int").AsInt(),
                    Float = k.Get("float") == null ? (float?)null : k.Get("float").AsFloat(),
                    String = S(k, "string"),
                    Volume = k.Get("volume") == null ? (float?)null : k.Get("volume").AsFloat(),
                    Balance = k.Get("balance") == null ? (float?)null : k.Get("balance").AsFloat(),
                };
                a.Events.Add(ek);
                d = Math.Max(d, ek.Time);
            }
            a.Duration = d;
            return a;
        }
    }
}
