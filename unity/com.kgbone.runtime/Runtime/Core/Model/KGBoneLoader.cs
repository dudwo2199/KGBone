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
                ReferenceScale = sk.Get("referenceScale")?.AsFloat(100) ?? 100,
            };
            foreach (var b in Items(sk.Get("bones"))) m.Bones.Add(ReadBone(b, m));
            foreach (var s in Items(sk.Get("slots"))) m.Slots.Add(ReadSlot(s, m));
            foreach (var c in Items(sk.Get("constraints"))) m.Constraints.Add(ReadConstraint(c, S(c, "type") ?? "", m));
            // formatVersion 0(초기 시험판): 제약이 종류별 배열로 따로 있다. order 순으로 합쳐 뒤에 잇는다(핵심 migrate.rs).
            var legacy = new List<(float order, int kind, int index, JsonValue v)>();
            for (int kind = 0; kind < LegacyKinds.Length; kind++)
            {
                var arr = Items(sk.Get(LegacyKinds[kind]));
                for (int i = 0; i < arr.Count; i++)
                    if (arr[i].Kind == JsonKind.Object) legacy.Add((F(arr[i], "order", 0), kind, i, arr[i]));
            }
            legacy.Sort((x, y) => x.order != y.order ? x.order.CompareTo(y.order) : x.kind != y.kind ? x.kind.CompareTo(y.kind) : x.index.CompareTo(y.index));
            foreach (var l in legacy) m.Constraints.Add(ReadConstraint(l.v, LegacyKinds[l.kind], m));
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

        static readonly string[] LegacyKinds = { "ik", "transform", "path", "physics" };

        static int[] BoneList(JsonValue v, SkeletonModel m)
        {
            var list = Items(v);
            var r = new int[list.Count];
            for (int i = 0; i < r.Length; i++) r[i] = m.BoneIndex(list[i].AsString());
            return r;
        }

        static BoneProperty Property(JsonValue o, string k)
        {
            switch (S(o, k))
            {
                case "rotate": return BoneProperty.Rotate;
                case "y": return BoneProperty.Y;
                case "scaleX": return BoneProperty.ScaleX;
                case "scaleY": return BoneProperty.ScaleY;
                case "shearY": return BoneProperty.ShearY;
                default: return BoneProperty.X;
            }
        }

        static SpacingMode Spacing(string s)
        {
            switch (s)
            {
                case "fixed": return SpacingMode.Fixed;
                case "percent": return SpacingMode.Percent;
                case "proportional": return SpacingMode.Proportional;
                default: return SpacingMode.Length;
            }
        }

        static RotateMode Rotate(string s)
        {
            switch (s)
            {
                case "chain": return RotateMode.Chain;
                case "chainScale": return RotateMode.ChainScale;
                default: return RotateMode.Tangent;
            }
        }

        static ConstraintData ReadConstraint(JsonValue v, string type, SkeletonModel m)
        {
            ConstraintData c;
            switch (type)
            {
                case "ik":
                    c = new IkConstraintData
                    {
                        Bones = BoneList(v.Get("bones"), m),
                        Target = m.BoneIndex(S(v, "target")),
                        Mix = F(v, "mix", 1), Softness = F(v, "softness", 0),
                        BendPositive = B(v, "bendPositive", true), Compress = B(v, "compress", false), Stretch = B(v, "stretch", false),
                        Uniform = B(v, "uniform", false), Volume = B(v, "volume", false),
                    };
                    break;
                case "transform":
                    var t = new TransformConstraintData
                    {
                        Bones = BoneList(v.Get("bones"), m),
                        Source = m.BoneIndex(S(v, "source")),
                        LocalSource = B(v, "localSource", false), LocalTarget = B(v, "localTarget", false),
                        Additive = B(v, "additive", false), Clamp = B(v, "clamp", false),
                        MixRotate = F(v, "mixRotate", 1), MixX = F(v, "mixX", 1), MixY = F(v, "mixY", 1),
                        MixScaleX = F(v, "mixScaleX", 1), MixScaleY = F(v, "mixScaleY", 1), MixShearY = F(v, "mixShearY", 1),
                    };
                    foreach (var p in Items(v.Get("properties")))
                    {
                        var map = new TransformMapping { From = Property(p, "from"), Offset = F(p, "offset", 0) };
                        foreach (var to in Items(p.Get("to")))
                            map.To.Add(new TransformTo { Property = Property(to, "property"), Offset = F(to, "offset", 0), Max = F(to, "max", 0), Scale = F(to, "scale", 1) });
                        t.Properties.Add(map);
                    }
                    // 비어 있으면 6개 속성의 같은 속성 기본 매핑(핵심 effective_properties)
                    if (t.Properties.Count == 0)
                        for (int i = 0; i < 6; i++)
                        {
                            var map = new TransformMapping { From = (BoneProperty)i };
                            map.To.Add(new TransformTo { Property = (BoneProperty)i });
                            t.Properties.Add(map);
                        }
                    c = t;
                    break;
                case "path":
                    c = new PathConstraintData
                    {
                        Bones = BoneList(v.Get("bones"), m),
                        Target = m.SlotIndex(S(v, "target")),
                        PositionMode = S(v, "positionMode") == "fixed" ? PositionMode.Fixed : PositionMode.Percent,
                        SpacingMode = Spacing(S(v, "spacingMode")),
                        RotateMode = Rotate(S(v, "rotateMode")),
                        OffsetRotation = F(v, "offsetRotation", 0), Position = F(v, "position", 0), Spacing = F(v, "spacing", 0),
                        MixRotate = F(v, "mixRotate", 1), MixX = F(v, "mixX", 1), MixY = F(v, "mixY", 1),
                    };
                    break;
                case "physics":
                    c = new PhysicsConstraintData
                    {
                        Bone = m.BoneIndex(S(v, "bone")),
                        X = F(v, "x", 0), Y = F(v, "y", 0), Rotate = F(v, "rotate", 0), ScaleX = F(v, "scaleX", 0), ShearX = F(v, "shearX", 0),
                        Limit = F(v, "limit", 5000), Fps = F(v, "fps", 60), Inertia = F(v, "inertia", 1), Strength = F(v, "strength", 100),
                        Damping = F(v, "damping", 1), Mass = F(v, "mass", 1), Wind = F(v, "wind", 0), Gravity = F(v, "gravity", 0),
                        Mix = F(v, "mix", 1), Volume = B(v, "volume", false),
                    };
                    break;
                case "slider":
                    c = new SliderConstraintData
                    {
                        Animation = S(v, "animation") ?? "",
                        Time = F(v, "time", 0), Mix = F(v, "mix", 1),
                        Loop = B(v, "loop", false), Additive = B(v, "additive", false), Local = B(v, "local", false),
                        // 이름이 있는데 못 찾으면 MissingBone(제약 비활성)
                        Bone = S(v, "bone") == null ? -1 : m.BoneIndex(S(v, "bone")) is int sb && sb >= 0 ? sb : SliderConstraintData.MissingBone,
                        Property = Property(v, "property"),
                        PropertyFrom = F(v, "propertyFrom", 0), PropertyTo = F(v, "propertyTo", 1),
                    };
                    break;
                default:
                    throw new KGBoneFormatException($"모르는 제약 종류: '{type}'");
            }
            c.Name = S(v, "name") ?? "";
            c.SkinRequired = B(v, "skinRequired", false);
            return c;
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
            foreach (var c in Items(v.Get("constraints"))) if (c.AsString() != null) skin.Constraints.Add(c.AsString());
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
                case "path":
                    a = new PathAttachmentData
                    {
                        Closed = B(v, "closed", false), ConstantSpeed = B(v, "constantSpeed", true),
                        VertexCount = v.Get("vertexCount")?.AsInt(0) ?? 0,
                        Weighted = B(v, "weighted", false),
                        Vertices = v.Get("vertices")?.AsFloatArray() ?? new float[0],
                        Lengths = v.Get("lengths")?.AsFloatArray() ?? new float[0],
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
            foreach (var c in v.Get("constraints")?.Members ?? new KeyValuePair<string, JsonValue>[0])
            {
                // 이름 ""의 키는 모든 물리 제약에 적용된다(Constraint = AllPhysics)
                int ci = c.Key.Length == 0 ? ConstraintTimeline.AllPhysics : m.ConstraintIndex(c.Key);
                foreach (var tl in c.Value.Members)
                {
                    var keys = ReadKeys(tl.Value, ref d);
                    if (ci != -1) a.Constraints.Add(new ConstraintTimeline { Constraint = ci, Name = tl.Key, Keys = keys });
                }
            }
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
