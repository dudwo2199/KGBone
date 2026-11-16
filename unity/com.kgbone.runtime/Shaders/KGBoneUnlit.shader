// KGBone 런타임 셰이더(URP): 미리 곱한 알파 텍스처 × 정점 색 + 어두운 색(tint black).
// 색 공식은 KGBone 화면(app/src/render/gl.ts)과 같다: rgb = (t.rgb × light.rgb + (t.a − t.rgb) × dark) × light.a, a = t.a × light.a
// 블렌드 함수는 머티리얼 속성(_SrcBlend, _DstBlend)으로 정한다(app/src/render/blend.ts의 4종).
Shader "KGBone/Unlit"
{
    Properties
    {
        _MainTex ("Texture", 2D) = "white" {}
        [HideInInspector] _SrcBlend ("Src Blend", Float) = 1
        [HideInInspector] _DstBlend ("Dst Blend", Float) = 10
    }
    SubShader
    {
        Tags { "RenderPipeline" = "UniversalPipeline" "Queue" = "Transparent" "RenderType" = "Transparent" "IgnoreProjector" = "True" }
        Pass
        {
            Name "KGBoneUnlit"
            Blend [_SrcBlend] [_DstBlend]
            ZWrite Off
            Cull Off

            HLSLPROGRAM
            #pragma vertex vert
            #pragma fragment frag
            #include "Packages/com.unity.render-pipelines.universal/ShaderLibrary/Core.hlsl"

            TEXTURE2D(_MainTex);
            SAMPLER(sampler_MainTex);

            CBUFFER_START(UnityPerMaterial)
                float4 _MainTex_ST;
                float _SrcBlend;
                float _DstBlend;
            CBUFFER_END

            struct Attributes
            {
                float3 positionOS : POSITION;
                float4 color : COLOR;
                float2 uv : TEXCOORD0;
                float4 dark : TEXCOORD1;
            };

            struct Varyings
            {
                float4 positionCS : SV_POSITION;
                float4 color : COLOR;
                float2 uv : TEXCOORD0;
                float3 dark : TEXCOORD1;
            };

            Varyings vert(Attributes i)
            {
                Varyings o;
                o.positionCS = TransformObjectToHClip(i.positionOS);
                o.color = i.color;
                o.uv = i.uv;
                o.dark = i.dark.rgb;
                return o;
            }

            half4 frag(Varyings i) : SV_Target
            {
                half4 t = SAMPLE_TEXTURE2D(_MainTex, sampler_MainTex, i.uv);
                half k = i.color.a;
                half3 rgb = (t.rgb * i.color.rgb + (t.a - t.rgb) * i.dark) * k;
                return half4(rgb, t.a * k);
            }
            ENDHLSL
        }
    }
}
